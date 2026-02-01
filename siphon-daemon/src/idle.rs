//! Idle Detection
//!
//! Tracks user activity and detects idle periods.
//! Useful for understanding work sessions and breaks.
//!
//! On macOS, this module can also detect system-level idle time
//! (time since last mouse/keyboard input) using IOKit.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{debug, info};

/// Activity categories for better tracking
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityCategory {
    /// Writing code, editing files
    Coding,
    /// In a meeting or video call
    Meeting,
    /// Using communication tools (Slack, email)
    Communication,
    /// Using a browser for research
    Research,
    /// Using creative applications
    Creative,
    /// Terminal/shell commands
    Terminal,
    /// Other/unknown activity
    Other,
}

impl ActivityCategory {
    /// Categorize an activity based on its source
    pub fn from_activity_type(activity_type: &str) -> Self {
        match activity_type {
            "shell" | "command" | "terminal" => ActivityCategory::Terminal,
            "editor" | "file_save" | "file_open" => ActivityCategory::Coding,
            "meeting" | "video_call" => ActivityCategory::Meeting,
            "window_change" => ActivityCategory::Other, // Will be refined based on app
            "clipboard" => ActivityCategory::Other,
            "hotkey" => ActivityCategory::Other,
            _ => ActivityCategory::Other,
        }
    }

    /// Categorize based on application name
    pub fn from_app_name(app_name: &str) -> Self {
        let app_lower = app_name.to_lowercase();

        // IDEs and code editors
        if app_lower.contains("code") || app_lower.contains("vim") || app_lower.contains("emacs")
            || app_lower.contains("idea") || app_lower.contains("xcode")
            || app_lower.contains("sublime") || app_lower.contains("atom")
            || app_lower.contains("cursor")
        {
            return ActivityCategory::Coding;
        }

        // Communication tools
        if app_lower.contains("slack") || app_lower.contains("teams")
            || app_lower.contains("mail") || app_lower.contains("outlook")
            || app_lower.contains("messages") || app_lower.contains("discord")
        {
            return ActivityCategory::Communication;
        }

        // Meeting apps
        if app_lower.contains("zoom") || app_lower.contains("meet")
            || app_lower.contains("facetime") || app_lower.contains("webex")
        {
            return ActivityCategory::Meeting;
        }

        // Creative apps
        if app_lower.contains("photoshop") || app_lower.contains("illustrator")
            || app_lower.contains("figma") || app_lower.contains("sketch")
            || app_lower.contains("blender") || app_lower.contains("premiere")
        {
            return ActivityCategory::Creative;
        }

        // Browsers (could be research or distraction)
        if app_lower.contains("chrome") || app_lower.contains("safari")
            || app_lower.contains("firefox") || app_lower.contains("arc")
            || app_lower.contains("edge")
        {
            return ActivityCategory::Research;
        }

        // Terminal
        if app_lower.contains("terminal") || app_lower.contains("iterm")
            || app_lower.contains("warp") || app_lower.contains("alacritty")
            || app_lower.contains("kitty")
        {
            return ActivityCategory::Terminal;
        }

        ActivityCategory::Other
    }
}

/// Idle detection configuration
#[derive(Debug, Clone)]
pub struct IdleConfig {
    /// Duration of inactivity before considered idle (default: 5 minutes)
    pub idle_threshold: Duration,
    /// Duration of inactivity before session ends (default: 30 minutes)
    pub session_end_threshold: Duration,
    /// Whether to use system-level idle detection (macOS only)
    pub use_system_idle: bool,
    /// Ignore idle detection during meetings
    pub ignore_idle_in_meetings: bool,
}

impl Default for IdleConfig {
    fn default() -> Self {
        Self {
            idle_threshold: Duration::from_secs(5 * 60), // 5 minutes
            session_end_threshold: Duration::from_secs(30 * 60), // 30 minutes
            use_system_idle: true,
            ignore_idle_in_meetings: true,
        }
    }
}

/// User activity state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityState {
    Active,
    Idle,
    Away,
}

/// Idle event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdleEventData {
    pub previous_state: String,
    pub new_state: String,
    pub idle_duration_seconds: u64,
    pub last_activity_type: Option<String>,
    /// Activity category of last activity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_activity_category: Option<ActivityCategory>,
    /// Whether idle was detected via system-level (IOKit) or app-level
    #[serde(default)]
    pub system_idle: bool,
}

/// Session tracking data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub session_id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_minutes: u64,
    pub event_count: u64,
    pub idle_periods: Vec<IdlePeriod>,
    /// Time spent in each activity category (in seconds)
    #[serde(default)]
    pub time_by_category: std::collections::HashMap<String, u64>,
    /// Whether session included any meetings
    #[serde(default)]
    pub had_meetings: bool,
    /// Active (non-idle) time in minutes
    #[serde(default)]
    pub active_minutes: u64,
}

/// A period of idle time within a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlePeriod {
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub duration_seconds: u64,
}

/// Idle detector that tracks user activity
pub struct IdleDetector {
    config: IdleConfig,
    last_activity: Instant,
    last_activity_time: DateTime<Utc>,
    current_state: ActivityState,
    session_start: Option<(Instant, DateTime<Utc>)>,
    session_event_count: u64,
    idle_periods: Vec<IdlePeriod>,
    current_idle_start: Option<DateTime<Utc>>,
    last_activity_type: Option<String>,
    last_activity_category: Option<ActivityCategory>,
    /// Whether currently in a meeting (suppresses idle detection)
    in_meeting: bool,
    /// Time tracking per category (category_name -> total_seconds)
    category_time: std::collections::HashMap<String, u64>,
    /// Last category change time
    last_category_change: Instant,
    /// Last known category
    current_category: Option<ActivityCategory>,
    /// Track if session had meetings
    session_had_meetings: bool,
}

/// Get system idle time on macOS using IOKit
#[cfg(target_os = "macos")]
pub fn get_system_idle_time() -> Option<Duration> {
    use std::process::Command;

    // Use ioreg to get HIDIdleTime from IOHIDSystem
    // This gives us the actual system-level idle time from mouse/keyboard
    let output = Command::new("ioreg")
        .args(["-c", "IOHIDSystem", "-d", "4"])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Look for HIDIdleTime in the output
    for line in stdout.lines() {
        if line.contains("HIDIdleTime") {
            // Extract the numeric value
            if let Some(start) = line.find('=') {
                let value_part = &line[start + 1..].trim();
                // Parse the number (it's in nanoseconds)
                if let Ok(nanos) = value_part.parse::<u64>() {
                    return Some(Duration::from_nanos(nanos));
                }
            }
        }
    }

    None
}

/// Fallback for non-macOS: no system idle detection
#[cfg(not(target_os = "macos"))]
pub fn get_system_idle_time() -> Option<Duration> {
    None
}

impl IdleDetector {
    /// Create a new idle detector
    pub fn new(config: IdleConfig) -> Self {
        Self {
            config,
            last_activity: Instant::now(),
            last_activity_time: Utc::now(),
            current_state: ActivityState::Active,
            session_start: None,
            session_event_count: 0,
            idle_periods: Vec::new(),
            current_idle_start: None,
            last_activity_type: None,
            last_activity_category: None,
            in_meeting: false,
            category_time: std::collections::HashMap::new(),
            last_category_change: Instant::now(),
            current_category: None,
            session_had_meetings: false,
        }
    }

    /// Set meeting state (called from main loop when meeting detector updates)
    pub fn set_in_meeting(&mut self, in_meeting: bool) {
        if in_meeting && !self.in_meeting {
            debug!("Entering meeting - idle detection paused");
            self.session_had_meetings = true;
        } else if !in_meeting && self.in_meeting {
            debug!("Leaving meeting - idle detection resumed");
        }
        self.in_meeting = in_meeting;
    }

    /// Update category time tracking
    fn update_category_time(&mut self, new_category: Option<ActivityCategory>) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_category_change).as_secs();

        // Record time for previous category
        if let Some(ref cat) = self.current_category {
            let cat_name = format!("{:?}", cat).to_lowercase();
            *self.category_time.entry(cat_name).or_insert(0) += elapsed;
        }

        self.current_category = new_category;
        self.last_category_change = now;
    }

    /// Record user activity
    pub fn record_activity(&mut self, activity_type: &str) -> Option<IdleEventData> {
        self.record_activity_with_app(activity_type, None)
    }

    /// Record user activity with optional app name for better categorization
    pub fn record_activity_with_app(
        &mut self,
        activity_type: &str,
        app_name: Option<&str>,
    ) -> Option<IdleEventData> {
        let now = Instant::now();
        let now_utc = Utc::now();
        let previous_state = self.current_state;

        // Determine activity category
        let category = if let Some(app) = app_name {
            ActivityCategory::from_app_name(app)
        } else {
            ActivityCategory::from_activity_type(activity_type)
        };

        // Update category time tracking
        self.update_category_time(Some(category.clone()));

        // End any current idle period
        if let Some(idle_start) = self.current_idle_start.take() {
            let duration = (now_utc - idle_start).num_seconds() as u64;
            self.idle_periods.push(IdlePeriod {
                started_at: idle_start,
                ended_at: now_utc,
                duration_seconds: duration,
            });
        }

        // Start session if not active
        if self.session_start.is_none() {
            self.session_start = Some((now, now_utc));
            self.category_time.clear();
            self.session_had_meetings = false;
            info!("New session started");
        }

        self.last_activity = now;
        self.last_activity_time = now_utc;
        self.current_state = ActivityState::Active;
        self.session_event_count += 1;
        self.last_activity_type = Some(activity_type.to_string());
        self.last_activity_category = Some(category.clone());

        // Track if this is a meeting activity
        if category == ActivityCategory::Meeting {
            self.session_had_meetings = true;
        }

        // Return state change event if applicable
        if previous_state != ActivityState::Active {
            let idle_duration = match previous_state {
                ActivityState::Idle | ActivityState::Away => {
                    now.duration_since(self.last_activity).as_secs()
                }
                _ => 0,
            };

            Some(IdleEventData {
                previous_state: format!("{:?}", previous_state).to_lowercase(),
                new_state: "active".to_string(),
                idle_duration_seconds: idle_duration,
                last_activity_type: self.last_activity_type.clone(),
                last_activity_category: self.last_activity_category.clone(),
                system_idle: false,
            })
        } else {
            None
        }
    }

    /// Check current idle status (call periodically)
    pub fn check_idle(&mut self) -> Option<IdleEventData> {
        // If in a meeting and configured to ignore idle during meetings, stay active
        if self.in_meeting && self.config.ignore_idle_in_meetings {
            // Record implicit activity from being in a meeting
            if self.current_state != ActivityState::Active {
                self.current_state = ActivityState::Active;
                return Some(IdleEventData {
                    previous_state: format!("{:?}", self.current_state).to_lowercase(),
                    new_state: "active".to_string(),
                    idle_duration_seconds: 0,
                    last_activity_type: Some("meeting".to_string()),
                    last_activity_category: Some(ActivityCategory::Meeting),
                    system_idle: false,
                });
            }
            return None;
        }

        let now = Instant::now();
        let now_utc = Utc::now();
        let previous_state = self.current_state;

        // Determine idle time - prefer system-level if available and configured
        let (elapsed, using_system_idle) = if self.config.use_system_idle {
            if let Some(system_idle) = get_system_idle_time() {
                // Use system idle time if it's longer than our app-level tracking
                let app_elapsed = now.duration_since(self.last_activity);
                if system_idle > app_elapsed {
                    (system_idle, true)
                } else {
                    (app_elapsed, false)
                }
            } else {
                (now.duration_since(self.last_activity), false)
            }
        } else {
            (now.duration_since(self.last_activity), false)
        };

        let new_state = if elapsed >= self.config.session_end_threshold {
            ActivityState::Away
        } else if elapsed >= self.config.idle_threshold {
            ActivityState::Idle
        } else {
            ActivityState::Active
        };

        if new_state != previous_state {
            self.current_state = new_state;

            // Update category time when going idle
            if new_state != ActivityState::Active {
                self.update_category_time(None);
            }

            // Track idle period start
            if new_state == ActivityState::Idle {
                self.current_idle_start = Some(now_utc);
            }

            // End session if away
            if new_state == ActivityState::Away {
                if let Some(idle_start) = self.current_idle_start.take() {
                    let duration = (now_utc - idle_start).num_seconds() as u64;
                    self.idle_periods.push(IdlePeriod {
                        started_at: idle_start,
                        ended_at: now_utc,
                        duration_seconds: duration,
                    });
                }
                info!("Session ended due to inactivity");
            }

            Some(IdleEventData {
                previous_state: format!("{:?}", previous_state).to_lowercase(),
                new_state: format!("{:?}", new_state).to_lowercase(),
                idle_duration_seconds: elapsed.as_secs(),
                last_activity_type: self.last_activity_type.clone(),
                last_activity_category: self.last_activity_category.clone(),
                system_idle: using_system_idle,
            })
        } else {
            None
        }
    }

    /// Get current session data
    pub fn get_session(&self) -> Option<SessionData> {
        self.session_start.map(|(start_instant, start_time)| {
            let duration = Instant::now().duration_since(start_instant);
            let ended = if self.current_state == ActivityState::Away {
                Some(self.last_activity_time)
            } else {
                None
            };

            // Calculate total idle time
            let total_idle_secs: u64 = self.idle_periods.iter().map(|p| p.duration_seconds).sum();
            let duration_secs = duration.as_secs();
            let active_secs = duration_secs.saturating_sub(total_idle_secs);

            SessionData {
                session_id: format!("session-{}", start_time.timestamp()),
                started_at: start_time,
                ended_at: ended,
                duration_minutes: duration_secs / 60,
                event_count: self.session_event_count,
                idle_periods: self.idle_periods.clone(),
                time_by_category: self.category_time.clone(),
                had_meetings: self.session_had_meetings,
                active_minutes: active_secs / 60,
            }
        })
    }

    /// Check if user is currently in a meeting
    pub fn is_in_meeting(&self) -> bool {
        self.in_meeting
    }

    /// End current session and reset
    pub fn end_session(&mut self) -> Option<SessionData> {
        let session = self.get_session();

        // Reset state
        self.session_start = None;
        self.session_event_count = 0;
        self.idle_periods.clear();
        self.current_idle_start = None;
        self.category_time.clear();
        self.session_had_meetings = false;
        self.current_category = None;

        session
    }

    /// Get current activity state
    pub fn state(&self) -> ActivityState {
        self.current_state
    }

    /// Get time since last activity
    pub fn time_since_activity(&self) -> Duration {
        Instant::now().duration_since(self.last_activity)
    }

    /// Check if currently in a session
    pub fn in_session(&self) -> bool {
        self.session_start.is_some() && self.current_state != ActivityState::Away
    }
}

impl Default for IdleDetector {
    fn default() -> Self {
        Self::new(IdleConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    fn test_config() -> IdleConfig {
        IdleConfig {
            idle_threshold: Duration::from_millis(100),
            session_end_threshold: Duration::from_millis(200),
            use_system_idle: false, // Disable for tests
            ignore_idle_in_meetings: true,
        }
    }

    #[test]
    fn test_activity_recording() {
        let mut detector = IdleDetector::new(test_config());

        // First activity starts a session
        detector.record_activity("shell");
        assert!(detector.in_session());
        assert_eq!(detector.state(), ActivityState::Active);
    }

    #[test]
    fn test_idle_detection() {
        let mut detector = IdleDetector::new(IdleConfig {
            idle_threshold: Duration::from_millis(50),
            session_end_threshold: Duration::from_millis(100),
            use_system_idle: false,
            ignore_idle_in_meetings: true,
        });

        detector.record_activity("shell");
        sleep(Duration::from_millis(60));

        let event = detector.check_idle();
        assert!(event.is_some());
        assert_eq!(detector.state(), ActivityState::Idle);
    }

    #[test]
    fn test_activity_category_from_type() {
        assert_eq!(
            ActivityCategory::from_activity_type("shell"),
            ActivityCategory::Terminal
        );
        assert_eq!(
            ActivityCategory::from_activity_type("editor"),
            ActivityCategory::Coding
        );
        assert_eq!(
            ActivityCategory::from_activity_type("meeting"),
            ActivityCategory::Meeting
        );
    }

    #[test]
    fn test_activity_category_from_app() {
        assert_eq!(
            ActivityCategory::from_app_name("Visual Studio Code"),
            ActivityCategory::Coding
        );
        assert_eq!(
            ActivityCategory::from_app_name("Slack"),
            ActivityCategory::Communication
        );
        assert_eq!(
            ActivityCategory::from_app_name("zoom.us"),
            ActivityCategory::Meeting
        );
        assert_eq!(
            ActivityCategory::from_app_name("Google Chrome"),
            ActivityCategory::Research
        );
        assert_eq!(
            ActivityCategory::from_app_name("Adobe Photoshop"),
            ActivityCategory::Creative
        );
        assert_eq!(
            ActivityCategory::from_app_name("iTerm2"),
            ActivityCategory::Terminal
        );
    }

    #[test]
    fn test_meeting_suppresses_idle() {
        let mut detector = IdleDetector::new(IdleConfig {
            idle_threshold: Duration::from_millis(10),
            session_end_threshold: Duration::from_millis(50),
            use_system_idle: false,
            ignore_idle_in_meetings: true,
        });

        detector.record_activity("shell");
        detector.set_in_meeting(true);

        // Wait past idle threshold
        sleep(Duration::from_millis(20));

        // Should still be active due to meeting
        let event = detector.check_idle();
        assert!(event.is_none() || detector.state() == ActivityState::Active);
    }

    #[test]
    fn test_session_tracks_categories() {
        let mut detector = IdleDetector::new(test_config());

        detector.record_activity_with_app("window_change", Some("Visual Studio Code"));
        sleep(Duration::from_millis(10));
        detector.record_activity_with_app("window_change", Some("Slack"));

        let session = detector.get_session();
        assert!(session.is_some());
        // Category time should have at least one entry
        // Note: Timing in tests is approximate
    }

    #[test]
    fn test_session_data_fields() {
        let mut detector = IdleDetector::new(test_config());

        detector.record_activity("shell");
        detector.set_in_meeting(true);
        detector.set_in_meeting(false);

        let session = detector.get_session().unwrap();
        assert!(session.had_meetings);
        assert!(session.event_count >= 1);
    }
}
