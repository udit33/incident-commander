use crate::domain::{AddNoteRequest, ApiError, CreateIncidentRequest, IncidentStatus, ListQuery};
use crate::infra::{self, AppError};
use axum::{
    extract::{Path, Query, Request, State},
    http::{header::HeaderName, HeaderValue, StatusCode},
    middleware::Next,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub api_key: Option<String>,
}

pub async fn auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<axum::response::Response, StatusCode> {
    if let Some(expected) = &state.api_key {
        let key_header = HeaderName::from_static("x-api-key");
        let got = request
            .headers()
            .get(key_header)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();

        if got != expected {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    Ok(next.run(request).await)
}

pub async fn health() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "auth": "x-api-key (optional via API_KEY env)"
        })),
    )
}

pub async fn create_incident(
    State(state): State<AppState>,
    Json(payload): Json<CreateIncidentRequest>,
) -> Result<axum::response::Response, AppError> {
    let title = payload.title.trim();
    if title.is_empty() {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                error: "title cannot be empty".into(),
            }),
        )
            .into_response());
    }

    let incident = infra::create_incident(&state.db, title, payload.description, payload.severity).await?;
    Ok((StatusCode::CREATED, Json(incident)).into_response())
}

pub async fn list_incidents(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<impl IntoResponse, AppError> {
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let incidents = infra::list_incidents(&state.db, query.status, query.severity, limit, offset).await?;

    Ok((
        StatusCode::OK,
        [(HeaderName::from_static("x-page-limit"), HeaderValue::from_str(&limit.to_string()).unwrap()),
         (HeaderName::from_static("x-page-offset"), HeaderValue::from_str(&offset.to_string()).unwrap())],
        Json(incidents),
    ))
}

pub async fn get_incident(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<axum::response::Response, AppError> {
    match infra::get_incident_by_id(&state.db, id).await? {
        Some(incident) => Ok((StatusCode::OK, Json(incident)).into_response()),
        None => Ok(not_found(id)),
    }
}

pub async fn get_timeline(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<axum::response::Response, AppError> {
    match infra::get_timeline(&state.db, id).await? {
        Some(events) => Ok((StatusCode::OK, Json(events)).into_response()),
        None => Ok(not_found(id)),
    }
}

pub async fn ack_incident(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<axum::response::Response, AppError> {
    match infra::update_status(&state.db, id, IncidentStatus::Acknowledged).await? {
        Some(incident) => Ok((StatusCode::OK, Json(incident)).into_response()),
        None => Ok(not_found(id)),
    }
}

pub async fn resolve_incident(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<axum::response::Response, AppError> {
    match infra::update_status(&state.db, id, IncidentStatus::Resolved).await? {
        Some(incident) => Ok((StatusCode::OK, Json(incident)).into_response()),
        None => Ok(not_found(id)),
    }
}

pub async fn add_note(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<AddNoteRequest>,
) -> Result<axum::response::Response, AppError> {
    let note = payload.note.trim();
    if note.is_empty() {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                error: "note cannot be empty".into(),
            }),
        )
            .into_response());
    }

    match infra::add_note(&state.db, id, note.to_string()).await? {
        Some(incident) => Ok((StatusCode::OK, Json(incident)).into_response()),
        None => Ok(not_found(id)),
    }
}

fn not_found(id: Uuid) -> axum::response::Response {
    (
        StatusCode::NOT_FOUND,
        Json(ApiError {
            error: format!("incident {} not found", id),
        }),
    )
        .into_response()
}
