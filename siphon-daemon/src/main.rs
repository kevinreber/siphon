//! Siphon Daemon
//!
//! Background service for capturing developer activity.
//! Runs on localhost:9847 and stores events in SQLite.

mod api;
mod storage;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::{Arc, Mutex};
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use crate::storage::EventStore;

/// Shared application state
pub struct AppState {
    pub store: Mutex<EventStore>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    info!("Starting Siphon daemon...");

    // Initialize storage
    let store = EventStore::new()?;
    info!("Database initialized at {:?}", store.db_path());

    let state = Arc::new(AppState { store: Mutex::new(store) });

    // Configure CORS for VS Code extension
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(api::health))
        // Event ingestion
        .route("/events/shell", post(api::ingest_shell_event))
        .route("/events/editor", post(api::ingest_editor_event))
        // Query endpoints
        .route("/events", get(api::get_events))
        .route("/events/recent", get(api::get_recent_events))
        .route("/stats", get(api::get_stats))
        .layer(cors)
        .with_state(state);

    // Bind to localhost only
    let addr = "127.0.0.1:9847";
    info!("Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
