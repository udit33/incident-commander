use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
use std::{fmt::Display, net::SocketAddr, str::FromStr};
use tracing::info;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum IncidentStatus {
    Open,
    Acknowledged,
    Resolved,
}

impl Display for IncidentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            IncidentStatus::Open => "open",
            IncidentStatus::Acknowledged => "acknowledged",
            IncidentStatus::Resolved => "resolved",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for IncidentStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "open" => Ok(IncidentStatus::Open),
            "acknowledged" => Ok(IncidentStatus::Acknowledged),
            "resolved" => Ok(IncidentStatus::Resolved),
            _ => Err(format!("invalid status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Severity::Low => "low",
            Severity::Medium => "medium",
            Severity::High => "high",
            Severity::Critical => "critical",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for Severity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "low" => Ok(Severity::Low),
            "medium" => Ok(Severity::Medium),
            "high" => Ok(Severity::High),
            "critical" => Ok(Severity::Critical),
            _ => Err(format!("invalid severity: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum EventType {
    Created,
    StatusChanged,
    NoteAdded,
}

impl Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            EventType::Created => "created",
            EventType::StatusChanged => "status_changed",
            EventType::NoteAdded => "note_added",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for EventType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "created" => Ok(EventType::Created),
            "status_changed" => Ok(EventType::StatusChanged),
            "note_added" => Ok(EventType::NoteAdded),
            _ => Err(format!("invalid event_type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IncidentEvent {
    id: Uuid,
    event_type: EventType,
    message: String,
    created_at: DateTime<Utc>,
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
    events: Vec<IncidentEvent>,
}

#[derive(Debug, Deserialize)]
struct CreateIncidentRequest {
    title: String,
    description: Option<String>,
    severity: Severity,
}

#[derive(Debug, Deserialize)]
struct AddNoteRequest {
    note: String,
}

#[derive(Debug, Serialize)]
struct ApiError {
    error: String,
}

#[derive(thiserror::Error, Debug)]
enum AppError {
    #[error("database error")]
    Db(#[from] sqlx::Error),
    #[error("invalid data: {0}")]
    InvalidData(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                error: self.to_string(),
            }),
        )
            .into_response()
    }
}

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

    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("failed to connect sqlite");

    init_db(&db).await.expect("failed to initialize schema");

    let state = AppState { db };

    let app = Router::new()
        .route("/health", get(health))
        .route("/incidents", get(list_incidents).post(create_incident))
        .route("/incidents/{id}", get(get_incident))
        .route("/incidents/{id}/ack", post(ack_incident))
        .route("/incidents/{id}/resolve", post(resolve_incident))
        .route("/incidents/{id}/notes", post(add_note))
        .route("/incidents/{id}/timeline", get(get_timeline))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("incident-commander backend listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind listener");

    axum::serve(listener, app).await.expect("server failed");
}

async fn init_db(db: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS incidents (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            description TEXT,
            severity TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )
    .execute(db)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS incident_events (
            id TEXT PRIMARY KEY,
            incident_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            message TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY(incident_id) REFERENCES incidents(id) ON DELETE CASCADE
        );
        "#,
    )
    .execute(db)
    .await?;

    Ok(())
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

async fn create_incident(
    State(state): State<AppState>,
    Json(payload): Json<CreateIncidentRequest>,
) -> Result<axum::response::Response, AppError> {
    if payload.title.trim().is_empty() {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                error: "title cannot be empty".to_string(),
            }),
        )
            .into_response());
    }

    let now = Utc::now();
    let id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO incidents (id, title, description, severity, status, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
    )
    .bind(id.to_string())
    .bind(payload.title.trim())
    .bind(payload.description.clone())
    .bind(payload.severity.to_string())
    .bind(IncidentStatus::Open.to_string())
    .bind(now.to_rfc3339())
    .bind(now.to_rfc3339())
    .execute(&state.db)
    .await?;

    insert_event(
        &state.db,
        id,
        EventType::Created,
        "Incident created".to_string(),
        now,
    )
    .await?;

    let incident = get_incident_by_id(&state.db, id).await?.ok_or_else(|| {
        AppError::InvalidData("created incident missing right after insert".to_string())
    })?;

    Ok((StatusCode::CREATED, Json(incident)).into_response())
}

async fn list_incidents(State(state): State<AppState>) -> Result<Json<Vec<Incident>>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT id, title, description, severity, status, created_at, updated_at
        FROM incidents
        ORDER BY created_at ASC
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    let mut incidents = Vec::with_capacity(rows.len());
    for row in rows {
        incidents.push(row_to_incident_without_events(&row)?);
    }

    for incident in &mut incidents {
        incident.events = list_events(&state.db, incident.id).await?;
    }

    Ok(Json(incidents))
}

async fn get_incident(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<axum::response::Response, AppError> {
    match get_incident_by_id(&state.db, id).await? {
        Some(incident) => Ok((StatusCode::OK, Json(incident)).into_response()),
        None => Ok(not_found(id)),
    }
}

async fn get_timeline(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<axum::response::Response, AppError> {
    if !incident_exists(&state.db, id).await? {
        return Ok(not_found(id));
    }

    let events = list_events(&state.db, id).await?;
    Ok((StatusCode::OK, Json(events)).into_response())
}

async fn ack_incident(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<axum::response::Response, AppError> {
    update_status(&state.db, id, IncidentStatus::Acknowledged).await
}

async fn resolve_incident(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<axum::response::Response, AppError> {
    update_status(&state.db, id, IncidentStatus::Resolved).await
}

async fn add_note(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<AddNoteRequest>,
) -> Result<axum::response::Response, AppError> {
    let note = payload.note.trim();
    if note.is_empty() {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                error: "note cannot be empty".to_string(),
            }),
        )
            .into_response());
    }

    if !incident_exists(&state.db, id).await? {
        return Ok(not_found(id));
    }

    let now = Utc::now();

    sqlx::query("UPDATE incidents SET updated_at = ?1 WHERE id = ?2")
        .bind(now.to_rfc3339())
        .bind(id.to_string())
        .execute(&state.db)
        .await?;

    insert_event(&state.db, id, EventType::NoteAdded, note.to_string(), now).await?;

    let incident = get_incident_by_id(&state.db, id).await?.ok_or_else(|| {
        AppError::InvalidData("incident missing after note write".to_string())
    })?;

    Ok((StatusCode::OK, Json(incident)).into_response())
}

async fn update_status(
    db: &SqlitePool,
    id: Uuid,
    status: IncidentStatus,
) -> Result<axum::response::Response, AppError> {
    if !incident_exists(db, id).await? {
        return Ok(not_found(id));
    }

    let now = Utc::now();

    sqlx::query("UPDATE incidents SET status = ?1, updated_at = ?2 WHERE id = ?3")
        .bind(status.to_string())
        .bind(now.to_rfc3339())
        .bind(id.to_string())
        .execute(db)
        .await?;

    insert_event(
        db,
        id,
        EventType::StatusChanged,
        format!("Status changed to {}", status),
        now,
    )
    .await?;

    let incident = get_incident_by_id(db, id).await?.ok_or_else(|| {
        AppError::InvalidData("incident missing after status update".to_string())
    })?;

    Ok((StatusCode::OK, Json(incident)).into_response())
}

async fn incident_exists(db: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
    let row = sqlx::query("SELECT 1 FROM incidents WHERE id = ?1")
        .bind(id.to_string())
        .fetch_optional(db)
        .await?;

    Ok(row.is_some())
}

async fn insert_event(
    db: &SqlitePool,
    incident_id: Uuid,
    event_type: EventType,
    message: String,
    at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO incident_events (id, incident_id, event_type, message, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(incident_id.to_string())
    .bind(event_type.to_string())
    .bind(message)
    .bind(at.to_rfc3339())
    .execute(db)
    .await?;

    Ok(())
}

async fn list_events(db: &SqlitePool, incident_id: Uuid) -> Result<Vec<IncidentEvent>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT id, event_type, message, created_at
        FROM incident_events
        WHERE incident_id = ?1
        ORDER BY created_at ASC
        "#,
    )
    .bind(incident_id.to_string())
    .fetch_all(db)
    .await?;

    let mut events = Vec::with_capacity(rows.len());
    for row in rows {
        let id_raw: String = row.try_get("id")?;
        let event_type_raw: String = row.try_get("event_type")?;
        let created_at_raw: String = row.try_get("created_at")?;

        events.push(IncidentEvent {
            id: Uuid::parse_str(&id_raw).map_err(|e| AppError::InvalidData(e.to_string()))?,
            event_type: EventType::from_str(&event_type_raw).map_err(AppError::InvalidData)?,
            message: row.try_get("message")?,
            created_at: DateTime::parse_from_rfc3339(&created_at_raw)
                .map_err(|e| AppError::InvalidData(e.to_string()))?
                .with_timezone(&Utc),
        });
    }

    Ok(events)
}

async fn get_incident_by_id(db: &SqlitePool, id: Uuid) -> Result<Option<Incident>, AppError> {
    let row = sqlx::query(
        r#"
        SELECT id, title, description, severity, status, created_at, updated_at
        FROM incidents
        WHERE id = ?1
        "#,
    )
    .bind(id.to_string())
    .fetch_optional(db)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let mut incident = row_to_incident_without_events(&row)?;
    incident.events = list_events(db, incident.id).await?;

    Ok(Some(incident))
}

fn row_to_incident_without_events(row: &sqlx::sqlite::SqliteRow) -> Result<Incident, AppError> {
    let id_raw: String = row.try_get("id")?;
    let severity_raw: String = row.try_get("severity")?;
    let status_raw: String = row.try_get("status")?;
    let created_at_raw: String = row.try_get("created_at")?;
    let updated_at_raw: String = row.try_get("updated_at")?;

    Ok(Incident {
        id: Uuid::parse_str(&id_raw).map_err(|e| AppError::InvalidData(e.to_string()))?,
        title: row.try_get("title")?,
        description: row.try_get("description")?,
        severity: Severity::from_str(&severity_raw).map_err(AppError::InvalidData)?,
        status: IncidentStatus::from_str(&status_raw).map_err(AppError::InvalidData)?,
        created_at: DateTime::parse_from_rfc3339(&created_at_raw)
            .map_err(|e| AppError::InvalidData(e.to_string()))?
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_at_raw)
            .map_err(|e| AppError::InvalidData(e.to_string()))?
            .with_timezone(&Utc),
        events: vec![],
    })
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
