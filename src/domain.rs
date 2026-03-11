use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IncidentStatus {
    Open,
    Acknowledged,
    Resolved,
}

impl IncidentStatus {
    pub fn can_transition_to(&self, next: &IncidentStatus) -> bool {
        matches!(
            (self, next),
            (IncidentStatus::Open, IncidentStatus::Acknowledged)
                | (IncidentStatus::Open, IncidentStatus::Resolved)
                | (IncidentStatus::Acknowledged, IncidentStatus::Resolved)
        )
    }
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
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
pub struct IncidentEvent {
    pub id: Uuid,
    pub event_type: EventType,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incident {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub severity: Severity,
    pub status: IncidentStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub events: Vec<IncidentEvent>,
}

#[derive(Debug, Deserialize)]
pub struct CreateIncidentRequest {
    pub title: String,
    pub description: Option<String>,
    pub severity: Severity,
}

#[derive(Debug, Deserialize)]
pub struct AddNoteRequest {
    pub note: String,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub status: Option<IncidentStatus>,
    pub severity: Option<Severity>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
}
