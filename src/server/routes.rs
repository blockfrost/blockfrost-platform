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
        regular
            .clone()
            .nest(&prefix.to_string(), regular.merge(hidden))
    }
}
