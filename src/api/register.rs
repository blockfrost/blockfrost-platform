use axum::Json;
use axum::{extract::State, Json as JsonExt};
use deadpool_diesel::postgres::Pool;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{models::Request, schema};

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseSuccess {
    route: String,
    status: String,
}

#[derive(Serialize, Deserialize)]
pub struct ResponseError {
    status: String,
    reason: String,
    details: String,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct Payload {
    mode: String,
    port: i32,
    secret: String,
    reward_address: String,
}

pub async fn route(
    State(pool): State<Pool>,
    JsonExt(payload): JsonExt<Payload>,
) -> Result<Json<ResponseSuccess>, Json<ResponseError>> {
    let db_pool = pool.get().await.map_err(|_| ResponseError {
        reason: "Database connection error".to_string(),
        status: "failed".to_string(),
        details: "Failed to get database connection from pool".to_string(),
    })?;

    println!("Creating a new request with data payload {:?}", payload);

    let new_request = Request {
        id: 0,
        status: "pending".to_string(),
    };

    let result = db_pool
        .interact(|db_pool| {
            diesel::insert_into(schema::requests::table)
                .values(new_request)
                .returning(Request::as_returning())
                .get_result(db_pool)
        })
        .await
        .map_err(|_| ResponseError {
            status: "failed".to_string(),
            reason: "Database interaction error".to_string(),
            details: "Failed to insert new request. Please contant our support".to_string(),
        })?
        .map_err(|_| ResponseError {
            status: "failed".to_string(),
            reason: "Database interaction error".to_string(),
            details: "Failed to insert new request. Please contant our support".to_string(),
        })?;

    let success_response = ResponseSuccess {
        status: "registered".to_string(),
        route: format!("/request/{}", result.id),
    };

    Ok(Json(success_response))
}
