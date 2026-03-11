use incident_commander::{api::AppState, build_router, infra};
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

    let app = build_router(AppState { db, api_key });

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("incident-commander backend listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind listener");

    axum::serve(listener, app).await.expect("server failed");
}
