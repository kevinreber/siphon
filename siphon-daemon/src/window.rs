//! Window tracking module
//!
//! Tracks the active window/application and captures context including:
//! - App name and window title
//! - Browser URLs (for Safari, Chrome, etc.)
//! - Time spent in each window

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
#[cfg(target_os = "macos")]
use std::process::Command;
use std::time::{Duration, Instant};
use tracing::debug;
#[cfg(target_os = "macos")]
use tracing::warn;

/// Known browser bundle identifiers for URL extraction
#[cfg(target_os = "macos")]
const BROWSER_BUNDLES: &[(&str, &str)] = &[
    ("com.apple.Safari", "Safari"),
    ("com.google.Chrome", "Google Chrome"),
    ("com.google.Chrome.beta", "Google Chrome Beta"),
    ("com.google.Chrome.dev", "Google Chrome Dev"),
    ("com.google.Chrome.canary", "Google Chrome Canary"),
    ("com.microsoft.edgemac", "Microsoft Edge"),
    ("com.microsoft.edgemac.Beta", "Microsoft Edge Beta"),
    ("com.microsoft.edgemac.Dev", "Microsoft Edge Dev"),
    ("com.brave.Browser", "Brave Browser"),
    ("com.brave.Browser.beta", "Brave Browser Beta"),
    ("com.operasoftware.Opera", "Opera"),
    ("com.vivaldi.Vivaldi", "Vivaldi"),
    ("org.mozilla.firefox", "Firefox"),
    ("com.arc.Arc", "Arc"),
];

/// Configuration for window tracking
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// Minimum time between window checks
    pub poll_interval: Duration,
    /// Whether to extract browser URLs
    pub extract_urls: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(1),
            extract_urls: true,
        }
    }
}

/// Information about the currently active window
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowInfo {
    /// Application name
    pub app_name: String,
    /// Window title (may be empty if Screen Recording permission not granted)
    pub title: String,
    /// Process ID
    pub process_id: u64,
    /// Bundle identifier (macOS only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_id: Option<String>,
    /// Browser URL if this is a browser window
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Window position and size
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounds: Option<WindowBounds>,
}

/// Window position and dimensions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Event emitted when the active window changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowChangeEvent {
    /// Previous window (None if this is the first event)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous: Option<WindowInfo>,
    /// New active window
    pub current: WindowInfo,
    /// How long the previous window was active (in milliseconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_duration_ms: Option<u64>,
    /// Timestamp of the change
    pub timestamp: DateTime<Utc>,
}

/// Tracks the active window and emits events on changes
pub struct WindowTracker {
    config: WindowConfig,
    last_window: Option<WindowInfo>,
    last_change_time: Instant,
    last_check_time: Instant,
}

impl WindowTracker {
    /// Create a new window tracker
    pub fn new(config: WindowConfig) -> Self {
        Self {
            config,
            last_window: None,
            last_change_time: Instant::now(),
            last_check_time: Instant::now(),
        }
    }

    /// Check the active window and return an event if it changed
    pub fn check_active_window(&mut self) -> Option<WindowChangeEvent> {
        // Respect poll interval
        if self.last_check_time.elapsed() < self.config.poll_interval {
            return None;
        }
        self.last_check_time = Instant::now();

        // Get current active window
        let current = self.get_active_window()?;

        // Check if window changed (compare app_name and title)
        let changed = match &self.last_window {
            Some(prev) => prev.app_name != current.app_name || prev.title != current.title,
            None => true,
        };

        if changed {
            let now = Instant::now();
            let duration_ms = if self.last_window.is_some() {
                Some(self.last_change_time.elapsed().as_millis() as u64)
            } else {
                None
            };

            let event = WindowChangeEvent {
                previous: self.last_window.take(),
                current: current.clone(),
                previous_duration_ms: duration_ms,
                timestamp: Utc::now(),
            };

            self.last_window = Some(current);
            self.last_change_time = now;

            debug!(
                "Window changed to: {} - {}",
                event.current.app_name, event.current.title
            );

            Some(event)
        } else {
            None
        }
    }

    /// Get the current active window information
    fn get_active_window(&self) -> Option<WindowInfo> {
        #[cfg(target_os = "macos")]
        {
            self.get_active_window_macos()
        }

        #[cfg(not(target_os = "macos"))]
        {
            self.get_active_window_other()
        }
    }

    /// macOS implementation using active-win-pos-rs
    #[cfg(target_os = "macos")]
    fn get_active_window_macos(&self) -> Option<WindowInfo> {
        use active_win_pos_rs::get_active_window;

        match get_active_window() {
            Ok(window) => {
                let bundle_id = window.app_name.clone(); // This is actually the bundle path on macOS
                let app_name = window.app_name.clone();

                // Try to get browser URL if this looks like a browser
                let url = if self.config.extract_urls {
                    self.get_browser_url(&app_name)
                } else {
                    None
                };

                Some(WindowInfo {
                    app_name,
                    title: window.title,
                    process_id: window.process_id,
                    bundle_id: Some(bundle_id),
                    url,
                    bounds: Some(WindowBounds {
                        x: window.position.x,
                        y: window.position.y,
                        width: window.position.width,
                        height: window.position.height,
                    }),
                })
            }
            Err(e) => {
                debug!("Failed to get active window: {:?}", e);
                None
            }
        }
    }

    /// Fallback for non-macOS platforms
    #[cfg(not(target_os = "macos"))]
    fn get_active_window_other(&self) -> Option<WindowInfo> {
        use active_win_pos_rs::get_active_window;

        match get_active_window() {
            Ok(window) => Some(WindowInfo {
                app_name: window.app_name,
                title: window.title,
                process_id: window.process_id,
                bundle_id: None,
                url: None, // URL extraction only supported on macOS for now
                bounds: Some(WindowBounds {
                    x: window.position.x,
                    y: window.position.y,
                    width: window.position.width,
                    height: window.position.height,
                }),
            }),
            Err(e) => {
                debug!("Failed to get active window: {:?}", e);
                None
            }
        }
    }

    /// Get the current URL from a browser using AppleScript (macOS only)
    #[cfg(target_os = "macos")]
    fn get_browser_url(&self, app_name: &str) -> Option<String> {
        // Check if this is a known browser
        let browser_name = BROWSER_BUNDLES
            .iter()
            .find(|(_, name)| app_name.contains(name))
            .map(|(_, name)| *name);

        let browser = browser_name.or_else(|| {
            // Fallback: check app name directly
            let lower = app_name.to_lowercase();
            if lower.contains("safari") {
                Some("Safari")
            } else if lower.contains("chrome") {
                Some("Google Chrome")
            } else if lower.contains("firefox") {
                Some("Firefox")
            } else if lower.contains("edge") {
                Some("Microsoft Edge")
            } else if lower.contains("brave") {
                Some("Brave Browser")
            } else if lower.contains("arc") {
                Some("Arc")
            } else if lower.contains("opera") {
                Some("Opera")
            } else if lower.contains("vivaldi") {
                Some("Vivaldi")
            } else {
                None
            }
        })?;

        // Build AppleScript based on browser
        let script = match browser {
            "Safari" => r#"tell application "Safari" to get URL of front document"#.to_string(),
            "Firefox" => {
                // Firefox doesn't support AppleScript URL access well
                return None;
            }
            _ => {
                // Chrome-based browsers (Chrome, Edge, Brave, Arc, Opera, Vivaldi)
                format!(
                    r#"tell application "{}" to get URL of active tab of front window"#,
                    browser
                )
            }
        };

        // Execute AppleScript
        match Command::new("osascript").args(["-e", &script]).output() {
            Ok(output) => {
                if output.status.success() {
                    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !url.is_empty() && url != "missing value" {
                        Some(url)
                    } else {
                        None
                    }
                } else {
                    debug!(
                        "AppleScript failed for {}: {}",
                        browser,
                        String::from_utf8_lossy(&output.stderr)
                    );
                    None
                }
            }
            Err(e) => {
                warn!("Failed to execute osascript: {}", e);
                None
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    #[allow(dead_code)]
    fn get_browser_url(&self, _app_name: &str) -> Option<String> {
        // URL extraction not supported on non-macOS platforms yet
        None
    }

    /// Get the current window without triggering a change event
    pub fn current_window(&self) -> Option<&WindowInfo> {
        self.last_window.as_ref()
    }

    /// Get how long the current window has been active
    pub fn current_window_duration(&self) -> Duration {
        self.last_change_time.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_config_default() {
        let config = WindowConfig::default();
        assert_eq!(config.poll_interval, Duration::from_secs(1));
        assert!(config.extract_urls);
    }

    #[test]
    fn test_window_tracker_creation() {
        let tracker = WindowTracker::new(WindowConfig::default());
        assert!(tracker.last_window.is_none());
        assert!(tracker.current_window().is_none());
    }
}
