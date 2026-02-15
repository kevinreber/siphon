//! Meeting detection module
//!
//! Detects when the user is in a video conference meeting based on active window.
//! Supports Zoom, Google Meet, Microsoft Teams, and other common meeting apps.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::{debug, info};

use crate::window::WindowInfo;

/// Known meeting application patterns
const MEETING_APPS: &[MeetingAppPattern] = &[
    MeetingAppPattern {
        app_name_contains: &["zoom.us", "Zoom"],
        title_patterns: &["Zoom Meeting", "Zoom Webinar"],
        platform: MeetingPlatform::Zoom,
    },
    MeetingAppPattern {
        app_name_contains: &[
            "Google Chrome",
            "Arc",
            "Safari",
            "Firefox",
            "Microsoft Edge",
        ],
        title_patterns: &["Meet -", "Google Meet", "meet.google.com"],
        platform: MeetingPlatform::GoogleMeet,
    },
    MeetingAppPattern {
        app_name_contains: &["Microsoft Teams", "Teams"],
        title_patterns: &["Meeting", "| Microsoft Teams"],
        platform: MeetingPlatform::Teams,
    },
    MeetingAppPattern {
        app_name_contains: &["Slack"],
        title_patterns: &["Huddle", "Call"],
        platform: MeetingPlatform::Slack,
    },
    MeetingAppPattern {
        app_name_contains: &["Discord"],
        title_patterns: &["Voice Connected", "Video Call"],
        platform: MeetingPlatform::Discord,
    },
    MeetingAppPattern {
        app_name_contains: &["FaceTime"],
        title_patterns: &["FaceTime"],
        platform: MeetingPlatform::FaceTime,
    },
    MeetingAppPattern {
        app_name_contains: &["Webex", "Cisco Webex"],
        title_patterns: &["Meeting", "Webex"],
        platform: MeetingPlatform::Webex,
    },
    MeetingAppPattern {
        app_name_contains: &["Around"],
        title_patterns: &["Around"],
        platform: MeetingPlatform::Around,
    },
    MeetingAppPattern {
        app_name_contains: &["Loom"],
        title_patterns: &["Recording", "Loom"],
        platform: MeetingPlatform::Loom,
    },
];

/// Pattern for matching meeting applications
struct MeetingAppPattern {
    /// Substrings to match in app name
    app_name_contains: &'static [&'static str],
    /// Substrings to match in window title
    title_patterns: &'static [&'static str],
    /// The platform this pattern matches
    platform: MeetingPlatform,
}

/// Meeting platforms we can detect
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MeetingPlatform {
    Zoom,
    GoogleMeet,
    Teams,
    Slack,
    Discord,
    FaceTime,
    Webex,
    Around,
    Loom,
    Unknown,
}

impl std::fmt::Display for MeetingPlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeetingPlatform::Zoom => write!(f, "Zoom"),
            MeetingPlatform::GoogleMeet => write!(f, "Google Meet"),
            MeetingPlatform::Teams => write!(f, "Microsoft Teams"),
            MeetingPlatform::Slack => write!(f, "Slack"),
            MeetingPlatform::Discord => write!(f, "Discord"),
            MeetingPlatform::FaceTime => write!(f, "FaceTime"),
            MeetingPlatform::Webex => write!(f, "Webex"),
            MeetingPlatform::Around => write!(f, "Around"),
            MeetingPlatform::Loom => write!(f, "Loom"),
            MeetingPlatform::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Current meeting state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MeetingState {
    /// Whether currently in a meeting
    pub in_meeting: bool,
    /// The platform being used
    pub platform: Option<MeetingPlatform>,
    /// When the meeting started
    pub started_at: Option<DateTime<Utc>>,
    /// Meeting title (from window title, may contain participant info)
    pub title: Option<String>,
}

/// Event emitted when meeting state changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingEvent {
    /// Type of event
    pub event_type: MeetingEventType,
    /// The platform
    pub platform: MeetingPlatform,
    /// Meeting title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Duration in minutes (for meeting_end events)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_minutes: Option<u32>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Types of meeting events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MeetingEventType {
    MeetingStart,
    MeetingEnd,
}

impl std::fmt::Display for MeetingEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeetingEventType::MeetingStart => write!(f, "meeting_start"),
            MeetingEventType::MeetingEnd => write!(f, "meeting_end"),
        }
    }
}

/// Configuration for meeting detection
#[derive(Debug, Clone)]
pub struct MeetingConfig {
    /// Minimum time in a meeting app to count as "in meeting" (prevents false positives)
    pub min_meeting_duration_secs: u64,
    /// Grace period after leaving meeting window before counting as "meeting ended"
    pub grace_period_secs: u64,
}

impl Default for MeetingConfig {
    fn default() -> Self {
        Self {
            min_meeting_duration_secs: 30,
            grace_period_secs: 60,
        }
    }
}

/// Detects when user is in a meeting
pub struct MeetingDetector {
    config: MeetingConfig,
    current_state: MeetingState,
    /// When we first detected meeting-like activity
    potential_meeting_start: Option<Instant>,
    /// When we last saw meeting activity
    last_meeting_activity: Option<Instant>,
    /// Track if we've emitted a start event for current meeting
    emitted_start: bool,
}

impl MeetingDetector {
    /// Create a new meeting detector
    pub fn new(config: MeetingConfig) -> Self {
        Self {
            config,
            current_state: MeetingState::default(),
            potential_meeting_start: None,
            last_meeting_activity: None,
            emitted_start: false,
        }
    }

    /// Check window info and return any meeting events
    pub fn check_window(&mut self, window: Option<&WindowInfo>) -> Vec<MeetingEvent> {
        let mut events = Vec::new();

        match window {
            Some(w) => {
                if let Some((platform, title)) = self.detect_meeting(w) {
                    // We're in a meeting window
                    self.last_meeting_activity = Some(Instant::now());

                    if let Some(start) = self.potential_meeting_start {
                        if !self.current_state.in_meeting {
                            // Check if we've been in meeting long enough
                            let elapsed = start.elapsed();
                            if elapsed.as_secs() >= self.config.min_meeting_duration_secs {
                                // Confirmed meeting
                                self.current_state = MeetingState {
                                    in_meeting: true,
                                    platform: Some(platform.clone()),
                                    started_at: Some(
                                        Utc::now() - Duration::seconds(elapsed.as_secs() as i64),
                                    ),
                                    title: title.clone(),
                                };
                                self.emitted_start = true;

                                info!("Meeting started: {} - {:?}", platform, title);
                                events.push(MeetingEvent {
                                    event_type: MeetingEventType::MeetingStart,
                                    platform,
                                    title,
                                    duration_minutes: None,
                                    timestamp: self.current_state.started_at.unwrap(),
                                });
                            }
                        }
                    } else {
                        // First time seeing meeting activity
                        self.potential_meeting_start = Some(Instant::now());
                        debug!("Potential meeting detected: {:?}", platform);
                    }
                } else {
                    // Not in a meeting window
                    self.check_meeting_end(&mut events);
                }
            }
            None => {
                // No window info available
                self.check_meeting_end(&mut events);
            }
        }

        events
    }

    /// Check if meeting has ended (grace period expired)
    fn check_meeting_end(&mut self, events: &mut Vec<MeetingEvent>) {
        if let Some(last_activity) = self.last_meeting_activity {
            if last_activity.elapsed().as_secs() >= self.config.grace_period_secs {
                // Meeting ended
                if self.current_state.in_meeting && self.emitted_start {
                    let duration = self
                        .current_state
                        .started_at
                        .map(|start| (Utc::now() - start).num_minutes() as u32);

                    info!(
                        "Meeting ended: {:?} (duration: {:?} min)",
                        self.current_state.platform, duration
                    );

                    events.push(MeetingEvent {
                        event_type: MeetingEventType::MeetingEnd,
                        platform: self
                            .current_state
                            .platform
                            .clone()
                            .unwrap_or(MeetingPlatform::Unknown),
                        title: self.current_state.title.clone(),
                        duration_minutes: duration,
                        timestamp: Utc::now(),
                    });
                }

                // Reset state
                self.current_state = MeetingState::default();
                self.potential_meeting_start = None;
                self.last_meeting_activity = None;
                self.emitted_start = false;
            }
        }
    }

    /// Detect if the current window is a meeting
    fn detect_meeting(&self, window: &WindowInfo) -> Option<(MeetingPlatform, Option<String>)> {
        let app_name = window.app_name.to_lowercase();
        let title = window.title.to_lowercase();

        for pattern in MEETING_APPS {
            // Check app name
            let app_matches = pattern
                .app_name_contains
                .iter()
                .any(|p| app_name.contains(&p.to_lowercase()));

            if !app_matches {
                continue;
            }

            // Check title patterns
            let title_matches = pattern
                .title_patterns
                .iter()
                .any(|p| title.contains(&p.to_lowercase()));

            // For browser-based meetings (Google Meet), require title match
            // For native apps (Zoom, Teams), app name alone might be enough
            let is_browser = ["chrome", "arc", "safari", "firefox", "edge"]
                .iter()
                .any(|b| app_name.contains(b));

            if title_matches || (!is_browser && app_matches) {
                return Some((pattern.platform.clone(), Some(window.title.clone())));
            }
        }

        None
    }

    /// Get current meeting state
    pub fn state(&self) -> &MeetingState {
        &self.current_state
    }

    /// Check if currently in a meeting
    pub fn in_meeting(&self) -> bool {
        self.current_state.in_meeting
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meeting_config_default() {
        let config = MeetingConfig::default();
        assert_eq!(config.min_meeting_duration_secs, 30);
        assert_eq!(config.grace_period_secs, 60);
    }

    #[test]
    fn test_meeting_platform_display() {
        assert_eq!(MeetingPlatform::Zoom.to_string(), "Zoom");
        assert_eq!(MeetingPlatform::GoogleMeet.to_string(), "Google Meet");
        assert_eq!(MeetingPlatform::Teams.to_string(), "Microsoft Teams");
    }

    #[test]
    fn test_meeting_event_type_display() {
        assert_eq!(MeetingEventType::MeetingStart.to_string(), "meeting_start");
        assert_eq!(MeetingEventType::MeetingEnd.to_string(), "meeting_end");
    }

    #[test]
    fn test_detect_zoom_meeting() {
        let detector = MeetingDetector::new(MeetingConfig::default());

        let window = WindowInfo {
            app_name: "zoom.us".to_string(),
            title: "Zoom Meeting".to_string(),
            process_id: 1234,
            bundle_id: None,
            url: None,
            bounds: None,
        };

        let result = detector.detect_meeting(&window);
        assert!(result.is_some());
        let (platform, _) = result.unwrap();
        assert_eq!(platform, MeetingPlatform::Zoom);
    }

    #[test]
    fn test_detect_google_meet() {
        let detector = MeetingDetector::new(MeetingConfig::default());

        let window = WindowInfo {
            app_name: "Google Chrome".to_string(),
            title: "Meet - abc-defg-hij".to_string(),
            process_id: 1234,
            bundle_id: None,
            url: Some("https://meet.google.com/abc-defg-hij".to_string()),
            bounds: None,
        };

        let result = detector.detect_meeting(&window);
        assert!(result.is_some());
        let (platform, _) = result.unwrap();
        assert_eq!(platform, MeetingPlatform::GoogleMeet);
    }

    #[test]
    fn test_no_meeting_detected() {
        let detector = MeetingDetector::new(MeetingConfig::default());

        let window = WindowInfo {
            app_name: "Visual Studio Code".to_string(),
            title: "main.rs - siphon".to_string(),
            process_id: 1234,
            bundle_id: None,
            url: None,
            bounds: None,
        };

        let result = detector.detect_meeting(&window);
        assert!(result.is_none());
    }
}
