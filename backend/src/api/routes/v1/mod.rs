pub mod timeline;

pub(crate) mod mod_helpers;

use axum::Router;

use crate::api::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().merge(timeline::router())
}
