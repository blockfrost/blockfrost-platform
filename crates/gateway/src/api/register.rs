use crate::blockfrost::BlockfrostAPI;
use crate::config::Config;
use crate::db::DB;
use crate::errors::APIError;
use crate::load_balancer::{AccessToken, LoadBalancerState};
use crate::models::RequestNewItem;
use crate::payload::Payload;
use axum::body::Bytes;
use axum::extract::ConnectInfo;
use axum::http::HeaderMap;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::info;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseSuccess {
    route: Uuid,
    status: String,
    /// Experimental: a list of WebSocket URIs and access tokens that the
    /// `blockfrost-platform` should connect to. Blockfrost.io request and
    /// responses, as well as network reconfiguration requests (in the future)
    /// will be will be passed to the `blockfrost-platform` over the socket(s),
    /// eventually eliminating the need for each relay to expose a public
    /// routable port, and making network configuration on their side much
    /// easier. We keep the previous setup and backwards compatibility, and just
    /// observe this experiment.
    load_balancers: Vec<LoadBalancer>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoadBalancer {
    uri: String,
    access_token: AccessToken,
}

pub async fn route(
    Extension(db): Extension<DB>,
    Extension(config): Extension<Config>,
    Extension(blockfrost_api): Extension<BlockfrostAPI>,
    Extension(load_balancer): Extension<LoadBalancerState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<ResponseSuccess>, APIError> {
    let payload: Payload = match serde_json::from_slice(&body) {
        Ok(payload) => payload,
        Err(e) => return Err(APIError::Validation(e.to_string())),
    };

    // validate POST payload
    Payload::validate(&payload)?;

    info!("Received valid payload for registration: {:?}", payload);

    let is_testnet_address = payload.reward_address.starts_with("addr_test");

    if config.server.network.is_testnet() {
        if !is_testnet_address {
            return Err(APIError::Validation(
                "Network and address mismatch: mainnet address provided on testnet".to_string(),
            ));
        }
    } else if is_testnet_address {
        return Err(APIError::Validation(
            "Network and address mismatch: testnet address provided on mainnet".to_string(),
        ));
    }

    // How the client sees us, needed for the load balancing experiment:
    let our_host: String = headers
        .get("Host")
        .and_then(|a| a.to_str().ok())
        .ok_or(APIError::Validation(
            "The request didn't set the Host: header field.".to_string(), // unreachable in HTTP ≥ 1.1
        ))?
        .to_string();

    // check if user has correct secret
    let authorized_user = db.authorize_user(payload.secret).await?;

    // get IP address
    let ip_string = if let Some(ip_header_value) = headers
        .get("HTTP_DO_CONNECTING_IP")
        .or_else(|| headers.get("CF-Connecting-IP"))
        .or_else(|| headers.get("X-Forwarded-For"))
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|val| val.to_str().ok())
    {
        // multiple ips are provided, take the first.
        ip_header_value
            .split(',')
            .next()
            .unwrap_or("")
            .trim()
            .to_string()
    } else {
        // fallback to the IP from the connection info (useful for localhost testing)
        addr.ip().to_string()
    };

    let ip_address: IpAddr = ip_string
        .parse()
        .map_err(|_| APIError::Validation(format!("Invalid IP address: {ip_string}")))?;

    let skip_port_check_secret = std::env::var("SKIP_PORT_CHECK_SECRET").ok();

    // Allow bypassing check for open port via header X-SKIP-PORT-CHECK = env SKIP_PORT_CHECK_SECRET
    let skip_port_check = headers
        .get("X-SKIP-PORT-CHECK")
        .and_then(|v| v.to_str().ok())
        .and_then(|header_val| {
            skip_port_check_secret
                .as_ref()
                .map(|secret| header_val.eq_ignore_ascii_case(secret))
        })
        .unwrap_or(false);
    if skip_port_check {
        info!("Skipping port check. Client passed X-SKIP-PORT-CHECK header.");
    } else {
        info!(
            "The server will now check if the IP address {} is reachable on port {}",
            ip_string, payload.port
        );

        let socket_addr = SocketAddr::new(ip_address, payload.port as u16);

        if !is_port_open(socket_addr).await {
            info!(
                "Failed to connect to IP {} on port {}",
                ip_string, payload.port
            );
            return Err(APIError::NotAccessible {
                ip: socket_addr,
                port: socket_addr.port(),
            });
        }

        info!(
            "Successfully checked that IP {} is reachable on port {}",
            ip_string, payload.port
        );
    }

    // check if NFT is at the address
    let asset = blockfrost_api
        .nft_exists(&payload.reward_address, &config.blockfrost.nft_asset)
        .await
        .map_err(|_| APIError::License(payload.reward_address.clone()))?;

    info!("NFT exists at address {}", payload.reward_address);

    let new_item_request = RequestNewItem {
        user_id: authorized_user.user_id,
        mode: payload.mode.clone(),
        ip_address: ip_address.to_string(),
        port: payload.port,
        route: payload.api_prefix.to_string(),
        reward_address: payload.reward_address.clone(),
        asset_name: Some(asset.asset_name.as_str().to_string()),
    };

    let token = load_balancer
        .new_access_token(
            asset.asset_name,
            payload.api_prefix,
            &payload.reward_address,
        )
        .await;

    let success_response = ResponseSuccess {
        status: "registered".to_string(),
        route: payload.api_prefix,
        load_balancers: vec![LoadBalancer {
            // We can’t know if it’s HTTPS or HTTP here, so we have to count on `blockfrost-platform`:
            uri: format!("//{our_host}/ws"),
            access_token: token,
        }],
    };

    db.insert_request(new_item_request).await?;

    Ok(Json(success_response))
}

async fn is_port_open(ip: SocketAddr) -> bool {
    let connection_future = TcpStream::connect(ip);

    matches!(
        timeout(Duration::from_secs(10), connection_future).await,
        Ok(Ok(_))
    )
}
