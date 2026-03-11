pub mod api;
pub mod domain;
pub mod infra;

use api::AppState;
use axum::{middleware, routing::{get, post}, Router};

pub fn build_router(state: AppState) -> Router {
    let protected = Router::new()
        .route("/incidents", get(api::list_incidents).post(api::create_incident))
        .route("/incidents/{id}", get(api::get_incident))
        .route("/incidents/{id}/ack", post(api::ack_incident))
        .route("/incidents/{id}/resolve", post(api::resolve_incident))
        .route("/incidents/{id}/notes", post(api::add_note))
        .route("/incidents/{id}/timeline", get(api::get_timeline))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            api::auth_middleware,
        ));

    Router::new()
        .route("/health", get(api::health))
        .merge(protected)
        .with_state(state)
}
