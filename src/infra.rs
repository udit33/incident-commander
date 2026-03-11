use crate::domain::{ApiError, EventType, Incident, IncidentEvent, IncidentStatus, Severity};
use axum::{http::StatusCode, response::IntoResponse, Json};
use chrono::{DateTime, Utc};
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
use std::str::FromStr;
use uuid::Uuid;

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("database error")]
    Db(#[from] sqlx::Error),
    #[error("invalid data: {0}")]
    InvalidData(String),
    #[error("invalid transition: {0}")]
    InvalidTransition(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            AppError::InvalidData(_) | AppError::InvalidTransition(_) => StatusCode::BAD_REQUEST,
            AppError::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (
            status,
            Json(ApiError {
                error: self.to_string(),
            }),
        )
            .into_response()
    }
}

pub async fn connect_db(url: &str) -> Result<SqlitePool, AppError> {
    Ok(SqlitePoolOptions::new().max_connections(5).connect(url).await?)
}

pub async fn init_db(db: &SqlitePool) -> Result<(), AppError> {
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS incidents (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            description TEXT,
            severity TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );"#,
    )
    .execute(db)
    .await?;

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS incident_events (
            id TEXT PRIMARY KEY,
            incident_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            message TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY(incident_id) REFERENCES incidents(id) ON DELETE CASCADE
        );"#,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn create_incident(
    db: &SqlitePool,
    title: &str,
    description: Option<String>,
    severity: Severity,
) -> Result<Incident, AppError> {
    let now = Utc::now();
    let id = Uuid::new_v4();

    sqlx::query("INSERT INTO incidents (id,title,description,severity,status,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7)")
        .bind(id.to_string())
        .bind(title)
        .bind(description)
        .bind(severity.to_string())
        .bind(IncidentStatus::Open.to_string())
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(db)
        .await?;

    insert_event(db, id, EventType::Created, "Incident created".into(), now).await?;

    get_incident_by_id(db, id)
        .await?
        .ok_or_else(|| AppError::InvalidData("created incident missing".into()))
}

pub async fn list_incidents(
    db: &SqlitePool,
    status: Option<IncidentStatus>,
    severity: Option<Severity>,
    limit: i64,
    offset: i64,
) -> Result<Vec<Incident>, AppError> {
    let rows = sqlx::query(
        r#"SELECT id,title,description,severity,status,created_at,updated_at
           FROM incidents
           WHERE (?1 IS NULL OR status = ?1)
             AND (?2 IS NULL OR severity = ?2)
           ORDER BY created_at ASC
           LIMIT ?3 OFFSET ?4"#,
    )
    .bind(status.map(|s| s.to_string()))
    .bind(severity.map(|s| s.to_string()))
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let mut i = row_to_incident_without_events(&row)?;
        i.events = list_events(db, i.id).await?;
        out.push(i);
    }
    Ok(out)
}

pub async fn get_incident_by_id(db: &SqlitePool, id: Uuid) -> Result<Option<Incident>, AppError> {
    let row = sqlx::query("SELECT id,title,description,severity,status,created_at,updated_at FROM incidents WHERE id = ?1")
        .bind(id.to_string())
        .fetch_optional(db)
        .await?;

    let Some(row) = row else { return Ok(None) };
    let mut i = row_to_incident_without_events(&row)?;
    i.events = list_events(db, i.id).await?;
    Ok(Some(i))
}

pub async fn update_status(db: &SqlitePool, id: Uuid, status: IncidentStatus) -> Result<Option<Incident>, AppError> {
    let Some(current) = get_incident_by_id(db, id).await? else {
        return Ok(None);
    };

    if current.status == status {
        return Ok(Some(current));
    }

    if !current.status.can_transition_to(&status) {
        return Err(AppError::InvalidTransition(format!(
            "{} -> {}",
            current.status, status
        )));
    }

    let now = Utc::now();
    sqlx::query("UPDATE incidents SET status = ?1, updated_at = ?2 WHERE id = ?3")
        .bind(status.to_string())
        .bind(now.to_rfc3339())
        .bind(id.to_string())
        .execute(db)
        .await?;

    insert_event(db, id, EventType::StatusChanged, format!("Status changed to {}", status), now).await?;
    get_incident_by_id(db, id).await
}

pub async fn add_note(db: &SqlitePool, id: Uuid, note: String) -> Result<Option<Incident>, AppError> {
    if !incident_exists(db, id).await? {
        return Ok(None);
    }

    let now = Utc::now();
    sqlx::query("UPDATE incidents SET updated_at = ?1 WHERE id = ?2")
        .bind(now.to_rfc3339())
        .bind(id.to_string())
        .execute(db)
        .await?;

    insert_event(db, id, EventType::NoteAdded, note, now).await?;
    get_incident_by_id(db, id).await
}

pub async fn get_timeline(db: &SqlitePool, id: Uuid) -> Result<Option<Vec<IncidentEvent>>, AppError> {
    if !incident_exists(db, id).await? {
        return Ok(None);
    }
    Ok(Some(list_events(db, id).await?))
}

async fn incident_exists(db: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
    Ok(sqlx::query("SELECT 1 FROM incidents WHERE id = ?1")
        .bind(id.to_string())
        .fetch_optional(db)
        .await?
        .is_some())
}

async fn insert_event(
    db: &SqlitePool,
    incident_id: Uuid,
    event_type: EventType,
    message: String,
    at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO incident_events (id,incident_id,event_type,message,created_at) VALUES (?1,?2,?3,?4,?5)")
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
    let rows = sqlx::query("SELECT id,event_type,message,created_at FROM incident_events WHERE incident_id = ?1 ORDER BY created_at ASC")
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
