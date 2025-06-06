pub mod hidden;
pub mod regular;

use super::state::{ApiPrefix, AppState};
use axum::Router;

pub fn nest_routes(
    prefix: &ApiPrefix,
    regular: Router<AppState>,
    hidden: Router<AppState>,
) -> Router<AppState> {
    if prefix.0.is_none() {
        regular.merge(hidden)
    } else {
        let prefix_ = prefix.clone();
        regular
            .clone()
            .nest(&prefix.to_string(), regular.merge(hidden))
            // Some of our health monitors request `GET /{prefix}/`:
            .route(
                &format!("{}/", prefix),
                axum::routing::get(|| async move {
                    axum::response::Redirect::permanent(&prefix_.to_string())
                }),
            )
    }
}
