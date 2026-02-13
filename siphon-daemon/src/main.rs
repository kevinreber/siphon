//! Siphon Daemon
//!
//! Background service for capturing developer activity.
//! Runs on localhost:9847 and stores events in SQLite.

mod api;
pub mod clipboard;
pub mod dedup;
pub mod hotkey;
pub mod idle;
pub mod meeting;
pub mod redact;
mod storage;
pub mod summary;
pub mod triggers;
pub mod watcher;
pub mod window;

use axum::{
    routing::{get, post},
    Router,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::interval;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use crate::clipboard::{ClipboardConfig, ClipboardTracker};
use crate::dedup::{DedupConfig, Deduplicator};
use crate::hotkey::{HotkeyConfig, HotkeyManager};
use crate::idle::{IdleConfig, IdleDetector};
use crate::meeting::{MeetingConfig, MeetingDetector};
use crate::storage::{EventSource, EventStore};
use crate::watcher::{FileWatcher, WatcherConfig};
use crate::window::{WindowConfig, WindowTracker};

/// Shared application state
pub struct AppState {
    pub store: Mutex<EventStore>,
    pub dedup: Mutex<Deduplicator>,
    pub idle_detector: Mutex<IdleDetector>,
    pub file_watcher: Mutex<Option<FileWatcher>>,
    pub window_tracker: Mutex<Option<WindowTracker>>,
    pub clipboard_tracker: Mutex<Option<ClipboardTracker>>,
    pub hotkey_manager: Mutex<Option<HotkeyManager>>,
    pub meeting_detector: Mutex<MeetingDetector>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    info!("Starting Siphon daemon...");

    // Initialize storage
    let store = EventStore::new()?;
    info!("Database initialized at {:?}", store.db_path());

    // Run automatic cleanup on startup (retain 30 days by default)
    let retention_days = std::env::var("SIPHON_RETENTION_DAYS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30u32);

    if let Ok(deleted) = store.cleanup_old_events(retention_days) {
        if deleted > 0 {
            info!(
                "Startup cleanup: removed {} events older than {} days",
                deleted, retention_days
            );
        }
    }

    // Initialize deduplicator
    let dedup = Deduplicator::new(DedupConfig::default());
    info!("Event deduplication enabled");

    // Initialize idle detector
    let idle_detector = IdleDetector::new(IdleConfig::default());
    info!("Idle detection enabled");

    // Initialize file watcher (optional - based on env var)
    let file_watcher = if let Ok(watch_paths) = std::env::var("SIPHON_WATCH_PATHS") {
        let paths: Vec<PathBuf> = watch_paths
            .split(':')
            .map(PathBuf::from)
            .filter(|p| p.exists())
            .collect();

        if !paths.is_empty() {
            let config = WatcherConfig {
                paths: paths.clone(),
                debounce_ms: 500,
                recursive: true,
            };
            let mut watcher = FileWatcher::new(config);
            match watcher.start() {
                Ok(_) => {
                    info!("File watcher started for {} paths", paths.len());
                    Some(watcher)
                }
                Err(e) => {
                    warn!("Failed to start file watcher: {}", e);
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    // Initialize window tracker (optional - disabled with SIPHON_DISABLE_WINDOW_TRACKING=1)
    let window_tracker = if std::env::var("SIPHON_DISABLE_WINDOW_TRACKING").is_ok() {
        info!("Window tracking disabled via environment variable");
        None
    } else {
        info!("Window tracking enabled");
        Some(WindowTracker::new(WindowConfig::default()))
    };

    // Initialize clipboard tracker (optional - disabled with SIPHON_DISABLE_CLIPBOARD_TRACKING=1)
    let clipboard_tracker = if std::env::var("SIPHON_DISABLE_CLIPBOARD_TRACKING").is_ok() {
        info!("Clipboard tracking disabled via environment variable");
        None
    } else {
        let tracker = ClipboardTracker::new(ClipboardConfig::default());
        if tracker.is_available() {
            info!("Clipboard tracking enabled");
            Some(tracker)
        } else {
            warn!("Clipboard tracking unavailable (no clipboard access)");
            None
        }
    };

    // Initialize hotkey manager (optional - disabled with SIPHON_DISABLE_HOTKEYS=1)
    // NOTE: On macOS, this must be created on the main thread (which we are on)
    let hotkey_manager = if std::env::var("SIPHON_DISABLE_HOTKEYS").is_ok() {
        info!("Hotkey system disabled via environment variable");
        None
    } else {
        let manager = HotkeyManager::new(HotkeyConfig::default());
        if manager.is_available() {
            manager.start_listener();
            info!("Hotkey system enabled (Cmd+Shift+M to mark moment)");
            Some(manager)
        } else {
            warn!("Hotkey system unavailable");
            None
        }
    };

    // Initialize meeting detector
    let meeting_detector = MeetingDetector::new(MeetingConfig::default());
    info!("Meeting detection enabled");

    let state = Arc::new(AppState {
        store: Mutex::new(store),
        dedup: Mutex::new(dedup),
        idle_detector: Mutex::new(idle_detector),
        file_watcher: Mutex::new(file_watcher),
        window_tracker: Mutex::new(window_tracker),
        clipboard_tracker: Mutex::new(clipboard_tracker),
        hotkey_manager: Mutex::new(hotkey_manager),
        meeting_detector: Mutex::new(meeting_detector),
    });

    // Spawn background task for file watching and idle detection
    let state_clone = Arc::clone(&state);
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(1));
        loop {
            ticker.tick().await;

            // Check for file events
            if let Ok(mut watcher_guard) = state_clone.file_watcher.try_lock() {
                if let Some(ref mut watcher) = *watcher_guard {
                    let events = watcher.poll_events();
                    if !events.is_empty() {
                        if let Ok(store) = state_clone.store.lock() {
                            for event in events {
                                let event_json = serde_json::to_string(&event).unwrap_or_default();
                                let project = watcher::detect_project_root(std::path::Path::new(
                                    &event.file_path,
                                ))
                                .and_then(|p| {
                                    p.file_name().map(|n| n.to_string_lossy().to_string())
                                });

                                if let Err(e) = store.insert_event(
                                    EventSource::Filesystem,
                                    &event.action,
                                    &event_json,
                                    project.as_deref(),
                                ) {
                                    warn!("Failed to store file event: {}", e);
                                }
                            }
                        }
                    }
                }
            }

            // Check for window changes and get current window for other trackers
            let (current_app, current_window) = if let Ok(mut tracker_guard) = state_clone.window_tracker.try_lock() {
                if let Some(ref mut tracker) = *tracker_guard {
                    if let Some(window_event) = tracker.check_active_window() {
                        // Record activity for idle detection with app name for categorization
                        let app_name = window_event.current.app_name.clone();
                        if let Ok(mut idle) = state_clone.idle_detector.try_lock() {
                            idle.record_activity_with_app("window_change", Some(&app_name));
                        }

                        // Store the window change event
                        if let Ok(store) = state_clone.store.lock() {
                            let event_json =
                                serde_json::to_string(&window_event).unwrap_or_default();
                            if let Err(e) = store.insert_event(
                                EventSource::Window,
                                "window_change",
                                &event_json,
                                None,
                            ) {
                                warn!("Failed to store window event: {}", e);
                            }
                        }
                    }
                    // Return current window for other trackers
                    (
                        tracker.current_window().map(|w| w.app_name.clone()),
                        tracker.current_window().cloned(),
                    )
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            // Check for meeting state changes
            if let Ok(mut detector_guard) = state_clone.meeting_detector.try_lock() {
                let meeting_events = detector_guard.check_window(current_window.as_ref());
                let in_meeting = detector_guard.in_meeting();

                // Sync meeting state with idle detector
                if let Ok(mut idle) = state_clone.idle_detector.try_lock() {
                    idle.set_in_meeting(in_meeting);
                }

                for event in meeting_events {
                    // Record activity for idle detection
                    if let Ok(mut idle) = state_clone.idle_detector.try_lock() {
                        idle.record_activity("meeting");
                    }

                    // Store the meeting event
                    if let Ok(store) = state_clone.store.lock() {
                        let event_json = serde_json::to_string(&event).unwrap_or_default();
                        if let Err(e) = store.insert_event(
                            EventSource::Meeting,
                            &event.event_type.to_string(),
                            &event_json,
                            None,
                        ) {
                            warn!("Failed to store meeting event: {}", e);
                        }
                    }
                }
            }

            // Check for clipboard changes
            if let Ok(mut tracker_guard) = state_clone.clipboard_tracker.try_lock() {
                if let Some(ref mut tracker) = *tracker_guard {
                    if let Some(clipboard_event) = tracker.check_clipboard(current_app) {
                        // Record activity for idle detection
                        if let Ok(mut idle) = state_clone.idle_detector.try_lock() {
                            idle.record_activity("clipboard");
                        }

                        // Store the clipboard change event
                        if let Ok(store) = state_clone.store.lock() {
                            let event_json =
                                serde_json::to_string(&clipboard_event).unwrap_or_default();
                            if let Err(e) = store.insert_event(
                                EventSource::Clipboard,
                                "clipboard_change",
                                &event_json,
                                None,
                            ) {
                                warn!("Failed to store clipboard event: {}", e);
                            }
                        }
                    }
                }
            }

            // Check for hotkey triggers
            if let Ok(manager_guard) = state_clone.hotkey_manager.try_lock() {
                if let Some(ref manager) = *manager_guard {
                    for trigger in manager.poll_triggers() {
                        info!("Hotkey triggered: {:?}", trigger.action);

                        // Record activity for idle detection
                        if let Ok(mut idle) = state_clone.idle_detector.try_lock() {
                            idle.record_activity("hotkey");
                        }

                        // Store the hotkey event
                        if let Ok(store) = state_clone.store.lock() {
                            let event_json =
                                serde_json::to_string(&trigger).unwrap_or_default();
                            if let Err(e) = store.insert_event(
                                EventSource::Hotkey,
                                &trigger.action.to_string(),
                                &event_json,
                                None,
                            ) {
                                warn!("Failed to store hotkey event: {}", e);
                            }
                        }
                    }
                }
            }

            // Check idle state (every 10 seconds)
            static mut IDLE_COUNTER: u32 = 0;
            unsafe {
                IDLE_COUNTER += 1;
                if IDLE_COUNTER >= 10 {
                    IDLE_COUNTER = 0;
                    if let Ok(mut idle) = state_clone.idle_detector.try_lock() {
                        if let Some(idle_event) = idle.check_idle() {
                            if let Ok(store) = state_clone.store.lock() {
                                let event_json =
                                    serde_json::to_string(&idle_event).unwrap_or_default();
                                let _ = store.insert_event(
                                    EventSource::Shell, // Use shell as source for idle events
                                    "idle_state_change",
                                    &event_json,
                                    None,
                                );
                            }
                        }
                    }
                }
            }
        }
    });

    // Configure CORS for VS Code extension
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Resolve UI directory: check ~/.siphon/ui/ first, then bundled ui/ next to binary
    let ui_dir = {
        let home_ui = dirs::home_dir()
            .map(|h| h.join(".siphon").join("ui"))
            .unwrap_or_default();
        let binary_ui = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("ui")))
            .unwrap_or_default();
        // Also check relative to working directory (for development)
        let cwd_ui = std::env::current_dir()
            .map(|d| d.join("siphon-ui"))
            .unwrap_or_default();

        if home_ui.join("index.html").exists() {
            info!("Serving UI from {:?}", home_ui);
            Some(home_ui)
        } else if binary_ui.join("index.html").exists() {
            info!("Serving UI from {:?}", binary_ui);
            Some(binary_ui)
        } else if cwd_ui.join("index.html").exists() {
            info!("Serving UI from {:?}", cwd_ui);
            Some(cwd_ui)
        } else {
            info!("No UI directory found (checked ~/.siphon/ui/, binary dir, cwd). Dashboard disabled.");
            None
        }
    };

    // Build router
    let mut app = Router::new()
        // Health check
        .route("/health", get(api::health))
        // Event ingestion
        .route("/events/shell", post(api::ingest_shell_event))
        .route("/events/editor", post(api::ingest_editor_event))
        .route("/events/filesystem", post(api::ingest_filesystem_event))
        // Watch management
        .route("/watch", post(api::add_watch_path))
        .route("/watch", axum::routing::delete(api::remove_watch_path))
        // Idle/session endpoints
        .route("/session", get(api::get_session_info))
        // Window tracking
        .route("/window", get(api::get_active_window))
        // Meeting tracking
        .route("/meeting", get(api::get_meeting_state))
        // Summary/insights
        .route("/summary", get(api::get_session_summary))
        // Query endpoints
        .route("/events", get(api::get_events))
        .route("/events/recent", get(api::get_recent_events))
        .route("/stats", get(api::get_stats))
        // Storage management
        .route("/storage", get(api::get_storage_info))
        .route("/storage/cleanup", post(api::cleanup_events))
        .layer(cors)
        .with_state(state);

    // Serve static UI files if directory exists
    if let Some(ui_path) = ui_dir {
        app = app.fallback_service(ServeDir::new(ui_path));
    }

    // Bind to localhost only
    let addr = "127.0.0.1:9847";
    info!("Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
