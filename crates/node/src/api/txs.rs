use crate::client::Node;
use api_provider::types::{
    AccountsAddressesResponse, AccountsDelegationsResponse, AccountsRegistrationsResponse,
    AccountsResponse, AccountsRewardsResponse,
};
use common::{pagination::Pagination, types::ApiResult};

pub struct NodeTxs<'a> {
    pub(crate) inner: &'a Node,
}

impl Node {
    pub fn txs(&self) -> DolosAccounts<'_> {
        DolosAccounts { inner: self }
    }
}

impl NodeTxs<'_> {
    pub async fn submit(&self, body: &str) -> ApiResult<AccountsResponse> {
        // Allow both hex-encoded and raw binary bodies
        let binary_tx = binary_or_hex_heuristic(body.as_ref());

        // XXX: Axum must not abort Ouroboros protocols in the middle, hence a separate Tokio task:
        let response_body = tokio::spawn(async move {
            // Submit transaction
            let mut node = node.get().await?;
            let response = node.submit_transaction(binary_tx).await;

            if response.is_ok() {
                counter!("tx_submit_success").increment(1)
            } else {
                counter!("tx_submit_failure").increment(1)
            }

            response
        })
        .await
        .expect("submit_transaction panic!")?;

        let mut response_headers = HeaderMap::new();

        response_headers.insert(
            "blockfrost-platform-response",
            response_body.to_string().parse()?,
        );

        Ok((response_headers, Json(response_body)))
    }
}
