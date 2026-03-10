use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    incidents: Arc<RwLock<HashMap<Uuid, Incident>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum IncidentStatus {
    Open,
    Acknowledged,
    Resolved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Incident {
    id: Uuid,
    title: String,
    description: Option<String>,
    severity: Severity,
    status: IncidentStatus,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct CreateIncidentRequest {
    title: String,
    description: Option<String>,
    severity: Severity,
}

#[derive(Debug, Serialize)]
struct ApiError {
    error: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "incident_commander=info,tower_http=info".to_string()),
        )
        .init();

    let state = AppState {
        incidents: Arc::new(RwLock::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/incidents", get(list_incidents).post(create_incident))
        .route("/incidents/{id}", get(get_incident))
        .route("/incidents/{id}/ack", post(ack_incident))
        .route("/incidents/{id}/resolve", post(resolve_incident))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("incident-commander backend listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind listener");

    axum::serve(listener, app)
        .await
        .expect("server failed");
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

async fn create_incident(
    State(state): State<AppState>,
    Json(payload): Json<CreateIncidentRequest>,
) -> impl IntoResponse {
    let now = Utc::now();
    let incident = Incident {
        id: Uuid::new_v4(),
        title: payload.title,
        description: payload.description,
        severity: payload.severity,
        status: IncidentStatus::Open,
        created_at: now,
        updated_at: now,
    };

    let mut incidents = state.incidents.write().await;
    incidents.insert(incident.id, incident.clone());

    (StatusCode::CREATED, Json(incident))
}

async fn list_incidents(State(state): State<AppState>) -> impl IntoResponse {
    let incidents = state.incidents.read().await;
    let mut values: Vec<Incident> = incidents.values().cloned().collect();
    values.sort_by_key(|i| i.created_at);
    Json(values)
}

async fn get_incident(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let incidents = state.incidents.read().await;
    match incidents.get(&id) {
        Some(incident) => (StatusCode::OK, Json(incident.clone())).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiError {
                error: format!("incident {} not found", id),
            }),
        )
            .into_response(),
    }
}

async fn ack_incident(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    update_status(state, id, IncidentStatus::Acknowledged).await
}

async fn resolve_incident(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    update_status(state, id, IncidentStatus::Resolved).await
}

async fn update_status(
    state: AppState,
    id: Uuid,
    status: IncidentStatus,
) -> axum::response::Response {
    let mut incidents = state.incidents.write().await;

    match incidents.get_mut(&id) {
        Some(incident) => {
            incident.status = status;
            incident.updated_at = Utc::now();
            (StatusCode::OK, Json(incident.clone())).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiError {
                error: format!("incident {} not found", id),
            }),
        )
            .into_response(),
    }
}
