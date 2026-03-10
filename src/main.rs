mod api;
mod domain;
mod infra;

use api::AppState;
use axum::{middleware, routing::{get, post}, Router};
use std::net::SocketAddr;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "incident_commander=info,tower_http=info".to_string()),
        )
        .init();

    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://./incident_commander.db".to_string());
    let api_key = std::env::var("API_KEY").ok();

    let db = infra::connect_db(&db_url).await.expect("failed to connect sqlite");
    infra::init_db(&db).await.expect("failed to initialize schema");

    let state = AppState { db, api_key };

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

    let app = Router::new()
        .route("/health", get(api::health))
        .merge(protected)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("incident-commander backend listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind listener");

    axum::serve(listener, app).await.expect("server failed");
}
