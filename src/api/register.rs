use axum::Json;
use axum::{extract::State, Json as JsonExt};
use deadpool_diesel::postgres::Pool;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::APIError;
use crate::payload::Payload;
use crate::{
    models::{Request, RequestNewItem},
    schema,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseSuccess {
    route: String,
    status: String,
}

pub async fn route(
    State(pool): State<Pool>,
    JsonExt(payload): JsonExt<Payload>,
) -> Result<Json<ResponseSuccess>, APIError> {
    Payload::validate(&payload)?;

    let db_pool = pool.get().await.map_err(APIError::DbConnectionError)?;

    // check if backend is online

    // check if NFT is at the address

    let new_item_request = RequestNewItem {
        user_id: Uuid::new_v4().to_string(),
        mode: payload.mode.clone(),
        ip_address: "TODO".to_string(),
        port: 0,
        reward_address: payload.reward_address.clone(),
    };

    let result = db_pool
        .interact(|db_pool| {
            diesel::insert_into(schema::requests::table)
                .values(new_item_request)
                .returning(Request::as_returning())
                .get_result(db_pool)
        })
        .await
        .map_err(|_| APIError::DbInteractionError("Failed to interact with db pool".to_string()))?
        .map_err(|_| APIError::DbInteractionError("Failed to insert new request".to_string()))?;

    let success_response = ResponseSuccess {
        status: "registered".to_string(),
        route: format!("https:://YOUR_SERVER_URL/{}", result.user_id),
    };

    Ok(Json(success_response))
}
