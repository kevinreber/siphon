//! Idle Detection
//!
//! Tracks user activity and detects idle periods.
//! Useful for understanding work sessions and breaks.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::info;

/// Idle detection configuration
#[derive(Debug, Clone)]
pub struct IdleConfig {
    /// Duration of inactivity before considered idle (default: 5 minutes)
    pub idle_threshold: Duration,
    /// Duration of inactivity before session ends (default: 30 minutes)
    pub session_end_threshold: Duration,
}

impl Default for IdleConfig {
    fn default() -> Self {
        Self {
            idle_threshold: Duration::from_secs(5 * 60),        // 5 minutes
            session_end_threshold: Duration::from_secs(30 * 60), // 30 minutes
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
        }
    }

    /// Record user activity
    pub fn record_activity(&mut self, activity_type: &str) -> Option<IdleEventData> {
        let now = Instant::now();
        let now_utc = Utc::now();
        let previous_state = self.current_state;

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
            info!("New session started");
        }

        self.last_activity = now;
        self.last_activity_time = now_utc;
        self.current_state = ActivityState::Active;
        self.session_event_count += 1;
        self.last_activity_type = Some(activity_type.to_string());

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
            })
        } else {
            None
        }
    }

    /// Check current idle status (call periodically)
    pub fn check_idle(&mut self) -> Option<IdleEventData> {
        let now = Instant::now();
        let now_utc = Utc::now();
        let elapsed = now.duration_since(self.last_activity);
        let previous_state = self.current_state;

        let new_state = if elapsed >= self.config.session_end_threshold {
            ActivityState::Away
        } else if elapsed >= self.config.idle_threshold {
            ActivityState::Idle
        } else {
            ActivityState::Active
        };

        if new_state != previous_state {
            self.current_state = new_state;

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

            SessionData {
                session_id: format!("session-{}", start_time.timestamp()),
                started_at: start_time,
                ended_at: ended,
                duration_minutes: duration.as_secs() / 60,
                event_count: self.session_event_count,
                idle_periods: self.idle_periods.clone(),
            }
        })
    }

    /// End current session and reset
    pub fn end_session(&mut self) -> Option<SessionData> {
        let session = self.get_session();

        // Reset state
        self.session_start = None;
        self.session_event_count = 0;
        self.idle_periods.clear();
        self.current_idle_start = None;

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

    #[test]
    fn test_activity_recording() {
        let mut detector = IdleDetector::new(IdleConfig {
            idle_threshold: Duration::from_millis(100),
            session_end_threshold: Duration::from_millis(200),
        });

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
        });

        detector.record_activity("shell");
        sleep(Duration::from_millis(60));

        let event = detector.check_idle();
        assert!(event.is_some());
        assert_eq!(detector.state(), ActivityState::Idle);
    }
}
