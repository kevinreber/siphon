//! HTTP API handlers

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::redact::redact_command;
use crate::storage::{EditorEventData, EventSource, ShellEventData};
use crate::AppState;

/// Health check response
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Health check endpoint
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Shell event request body
#[derive(Debug, Deserialize)]
pub struct ShellEventRequest {
    pub command: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub cwd: String,
    #[serde(default)]
    pub git_branch: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
}

/// Ingest shell event
pub async fn ingest_shell_event(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ShellEventRequest>,
) -> impl IntoResponse {
    // Redact sensitive information from command
    let redaction_result = redact_command(&payload.command);

    // If command should be skipped entirely (e.g., password manager commands), return success without storing
    let redacted_command = match redaction_result.command {
        Some(cmd) => cmd,
        None => {
            info!("Skipped sensitive command (not stored)");
            return (
                StatusCode::CREATED,
                Json(serde_json::json!({ "id": null, "skipped": true })),
            );
        }
    };

    if redaction_result.was_redacted {
        info!(
            "Redacted {} sensitive pattern(s) from command",
            redaction_result.redaction_count
        );
    }

    // Detect project from cwd
    let project = detect_project(&payload.cwd);

    let event_data = ShellEventData {
        command: redacted_command.clone(),
        exit_code: payload.exit_code,
        duration_ms: payload.duration_ms,
        cwd: payload.cwd.clone(),
        git_branch: payload.git_branch.clone(),
    };

    let event_data_json = serde_json::to_string(&event_data).unwrap_or_default();

    // Determine event type based on command characteristics
    let event_type = if payload.exit_code != 0 {
        "command_failed"
    } else {
        "command"
    };

    let store = state.store.lock().unwrap();
    match store.insert_event(EventSource::Shell, event_type, &event_data_json, project.as_deref())
    {
        Ok(id) => {
            info!(
                "Recorded shell event: {} (exit: {}, duration: {}ms)",
                truncate_command(&redacted_command),
                payload.exit_code,
                payload.duration_ms
            );
            (StatusCode::CREATED, Json(serde_json::json!({ "id": id })))
        }
        Err(e) => {
            tracing::error!("Failed to store event: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        }
    }
}

/// Editor event request body
#[derive(Debug, Deserialize)]
pub struct EditorEventRequest {
    pub action: String,
    pub file_path: String,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub lines_changed: Option<i32>,
}

/// Ingest editor event
pub async fn ingest_editor_event(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<EditorEventRequest>,
) -> impl IntoResponse {
    // Detect project from file path
    let project = detect_project(&payload.file_path);

    let event_data = EditorEventData {
        action: payload.action.clone(),
        file_path: payload.file_path.clone(),
        language: payload.language.clone(),
        lines_changed: payload.lines_changed,
    };

    let event_data_json = serde_json::to_string(&event_data).unwrap_or_default();

    let store = state.store.lock().unwrap();
    match store.insert_event(
        EventSource::Editor,
        &payload.action,
        &event_data_json,
        project.as_deref(),
    ) {
        Ok(id) => {
            info!("Recorded editor event: {} on {}", payload.action, payload.file_path);
            (StatusCode::CREATED, Json(serde_json::json!({ "id": id })))
        }
        Err(e) => {
            tracing::error!("Failed to store event: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        }
    }
}

/// Query parameters for events endpoint
#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    #[serde(default = "default_hours")]
    pub hours: u32,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub project: Option<String>,
}

fn default_hours() -> u32 {
    24
}

/// Get events
pub async fn get_events(
    State(state): State<Arc<AppState>>,
    Query(query): Query<EventsQuery>,
) -> impl IntoResponse {
    let store = state.store.lock().unwrap();
    match store.get_recent_events(query.hours) {
        Ok(events) => (StatusCode::OK, Json(serde_json::json!({ "events": events }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

/// Get recent events (last 2 hours by default)
pub async fn get_recent_events(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let store = state.store.lock().unwrap();
    match store.get_recent_events(2) {
        Ok(events) => (StatusCode::OK, Json(serde_json::json!({ "events": events }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
    }
}

/// Stats response
#[derive(Serialize)]
pub struct StatsResponse {
    pub total_events: i64,
    pub events_by_source: Vec<(String, i64)>,
}

/// Get stats
pub async fn get_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let store = state.store.lock().unwrap();

    let total = store.get_total_count().unwrap_or(0);
    let by_source = store.get_stats().unwrap_or_default();

    Json(StatsResponse {
        total_events: total,
        events_by_source: by_source,
    })
}

/// Detect project name from path
fn detect_project(path: &str) -> Option<String> {
    // Find the last component that looks like a project directory
    // Usually the directory containing .git, package.json, Cargo.toml, etc.

    let path = std::path::Path::new(path);

    // Walk up the path looking for project markers
    let mut current = Some(path);
    while let Some(p) = current {
        if p.join(".git").exists()
            || p.join("package.json").exists()
            || p.join("Cargo.toml").exists()
            || p.join("go.mod").exists()
            || p.join("pyproject.toml").exists()
        {
            return p.file_name().and_then(|n| n.to_str()).map(String::from);
        }
        current = p.parent();
    }

    // Fallback: use the immediate parent directory name
    path.parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .map(String::from)
}

/// Truncate command for logging
fn truncate_command(cmd: &str) -> &str {
    if cmd.len() > 50 {
        &cmd[..50]
    } else {
        cmd
    }
}
