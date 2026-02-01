//! Session summary module
//!
//! Aggregates events from a time period into structured summaries.
//! Provides the foundation for AI-powered insights.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

use crate::storage::{Event, EventStore};

/// Summary of a work session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    /// When the session started
    pub start_time: DateTime<Utc>,
    /// When the session ended
    pub end_time: DateTime<Utc>,
    /// Duration in minutes
    pub duration_minutes: u32,
    /// Total events in the session
    pub total_events: usize,
    /// Breakdown by event source
    pub events_by_source: HashMap<String, usize>,
    /// Projects worked on
    pub projects: Vec<ProjectSummary>,
    /// Applications used
    pub applications: Vec<AppUsageSummary>,
    /// Key activities detected
    pub key_activities: Vec<ActivitySummary>,
    /// Meetings attended
    pub meetings: Vec<MeetingSummary>,
    /// Focus score (0-100)
    pub focus_score: u32,
    /// Suggested summary text (for AI to enhance)
    pub summary_text: String,
}

/// Summary of work on a specific project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub name: String,
    pub event_count: usize,
    pub duration_minutes: u32,
    pub file_changes: usize,
    pub commands_run: usize,
}

/// Summary of application usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppUsageSummary {
    pub app_name: String,
    pub duration_minutes: u32,
    pub window_switches: usize,
}

/// Summary of a detected activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivitySummary {
    pub activity_type: String,
    pub description: String,
    pub timestamp: DateTime<Utc>,
}

/// Summary of a meeting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingSummary {
    pub platform: String,
    pub title: Option<String>,
    pub duration_minutes: u32,
    pub started_at: DateTime<Utc>,
}

/// Configuration for summary generation
#[derive(Debug, Clone)]
pub struct SummaryConfig {
    /// Minimum session duration to generate summary (minutes)
    pub min_session_duration: u32,
    /// Include clipboard content in summary
    pub include_clipboard: bool,
    /// Include browser URLs in summary
    pub include_urls: bool,
}

impl Default for SummaryConfig {
    fn default() -> Self {
        Self {
            min_session_duration: 15,
            include_clipboard: false, // Privacy by default
            include_urls: true,
        }
    }
}

/// Generates session summaries from event data
pub struct SummaryGenerator {
    config: SummaryConfig,
}

impl SummaryGenerator {
    /// Create a new summary generator
    pub fn new(config: SummaryConfig) -> Self {
        Self { config }
    }

    /// Generate a summary for a time range
    pub fn generate_summary(
        &self,
        store: &EventStore,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Option<SessionSummary> {
        // Get events in range
        let events = store.get_events_since(start_time, None).ok()?;
        let events: Vec<_> = events
            .into_iter()
            .filter(|e| e.timestamp <= end_time)
            .collect();

        if events.is_empty() {
            return None;
        }

        let duration_minutes = (end_time - start_time).num_minutes() as u32;

        if duration_minutes < self.config.min_session_duration {
            return None;
        }

        debug!(
            "Generating summary for {} events over {} minutes",
            events.len(),
            duration_minutes
        );

        // Aggregate by source
        let mut events_by_source: HashMap<String, usize> = HashMap::new();
        for event in &events {
            *events_by_source.entry(event.source.clone()).or_insert(0) += 1;
        }

        // Extract projects
        let projects = self.extract_projects(&events);

        // Extract application usage
        let applications = self.extract_app_usage(&events);

        // Extract key activities
        let key_activities = self.extract_key_activities(&events);

        // Extract meetings
        let meetings = self.extract_meetings(&events);

        // Calculate focus score
        let focus_score = self.calculate_focus_score(&events, &applications, duration_minutes);

        // Generate summary text
        let summary_text = self.generate_summary_text(
            &projects,
            &applications,
            &key_activities,
            &meetings,
            duration_minutes,
        );

        Some(SessionSummary {
            start_time,
            end_time,
            duration_minutes,
            total_events: events.len(),
            events_by_source,
            projects,
            applications,
            key_activities,
            meetings,
            focus_score,
            summary_text,
        })
    }

    /// Generate a summary for the last N hours
    pub fn generate_recent_summary(
        &self,
        store: &EventStore,
        hours: u32,
    ) -> Option<SessionSummary> {
        let end_time = Utc::now();
        let start_time = end_time - Duration::hours(hours as i64);
        self.generate_summary(store, start_time, end_time)
    }

    /// Extract project summaries from events
    fn extract_projects(&self, events: &[Event]) -> Vec<ProjectSummary> {
        let mut projects: HashMap<String, ProjectSummary> = HashMap::new();

        for event in events {
            if let Some(ref project) = event.project {
                let entry = projects.entry(project.clone()).or_insert(ProjectSummary {
                    name: project.clone(),
                    event_count: 0,
                    duration_minutes: 0,
                    file_changes: 0,
                    commands_run: 0,
                });

                entry.event_count += 1;

                if event.source == "filesystem" {
                    entry.file_changes += 1;
                }
                if event.source == "shell" {
                    entry.commands_run += 1;
                }
            }
        }

        // Estimate duration based on event distribution
        let total_events: usize = projects.values().map(|p| p.event_count).sum();
        for project in projects.values_mut() {
            if total_events > 0 {
                // Rough estimate: proportional to event count
                project.duration_minutes =
                    ((project.event_count as f64 / total_events as f64) * 60.0) as u32;
            }
        }

        let mut result: Vec<_> = projects.into_values().collect();
        result.sort_by(|a, b| b.event_count.cmp(&a.event_count));
        result
    }

    /// Extract application usage from window events
    fn extract_app_usage(&self, events: &[Event]) -> Vec<AppUsageSummary> {
        let mut apps: HashMap<String, AppUsageSummary> = HashMap::new();

        for event in events {
            if event.source == "window" {
                // Parse window event data
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&event.event_data) {
                    if let Some(current) = data.get("current") {
                        if let Some(app_name) = current.get("app_name").and_then(|v| v.as_str()) {
                            let entry = apps.entry(app_name.to_string()).or_insert(AppUsageSummary {
                                app_name: app_name.to_string(),
                                duration_minutes: 0,
                                window_switches: 0,
                            });
                            entry.window_switches += 1;

                            // Estimate duration from previous window duration
                            if let Some(duration) = data.get("previous_duration_ms").and_then(|v| v.as_u64()) {
                                entry.duration_minutes += (duration / 60000) as u32;
                            }
                        }
                    }
                }
            }
        }

        let mut result: Vec<_> = apps.into_values().collect();
        result.sort_by(|a, b| b.duration_minutes.cmp(&a.duration_minutes));
        result
    }

    /// Extract key activities from events
    fn extract_key_activities(&self, events: &[Event]) -> Vec<ActivitySummary> {
        let mut activities = Vec::new();

        for event in events {
            // Detect hotkey triggers
            if event.source == "hotkey" {
                activities.push(ActivitySummary {
                    activity_type: "mark".to_string(),
                    description: "Marked an important moment".to_string(),
                    timestamp: event.timestamp,
                });
            }

            // Detect git commits
            if event.source == "shell" && event.event_type == "command" {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&event.event_data) {
                    if let Some(cmd) = data.get("command").and_then(|v| v.as_str()) {
                        if cmd.contains("git commit") {
                            activities.push(ActivitySummary {
                                activity_type: "commit".to_string(),
                                description: "Made a git commit".to_string(),
                                timestamp: event.timestamp,
                            });
                        } else if cmd.contains("git push") {
                            activities.push(ActivitySummary {
                                activity_type: "push".to_string(),
                                description: "Pushed changes".to_string(),
                                timestamp: event.timestamp,
                            });
                        } else if cmd.starts_with("npm test") || cmd.starts_with("cargo test") || cmd.starts_with("pytest") {
                            activities.push(ActivitySummary {
                                activity_type: "test".to_string(),
                                description: "Ran tests".to_string(),
                                timestamp: event.timestamp,
                            });
                        }
                    }
                }
            }
        }

        // Limit to most recent activities
        activities.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        activities.truncate(20);
        activities
    }

    /// Extract meeting summaries
    fn extract_meetings(&self, events: &[Event]) -> Vec<MeetingSummary> {
        let mut meetings = Vec::new();

        for event in events {
            if event.source == "meeting" && event.event_type == "meeting_end" {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&event.event_data) {
                    let platform = data.get("platform")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let title = data.get("title")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let duration_minutes = data.get("duration_minutes")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32;

                    meetings.push(MeetingSummary {
                        platform,
                        title,
                        duration_minutes,
                        started_at: event.timestamp,
                    });
                }
            }
        }

        meetings
    }

    /// Calculate focus score (0-100)
    fn calculate_focus_score(
        &self,
        events: &[Event],
        apps: &[AppUsageSummary],
        duration_minutes: u32,
    ) -> u32 {
        if duration_minutes == 0 || events.is_empty() {
            return 0;
        }

        let mut score = 100u32;

        // Penalize excessive context switching
        let window_switches: usize = apps.iter().map(|a| a.window_switches).sum();
        let switches_per_hour = (window_switches as f64 / duration_minutes as f64) * 60.0;
        if switches_per_hour > 30.0 {
            score = score.saturating_sub(20);
        } else if switches_per_hour > 15.0 {
            score = score.saturating_sub(10);
        }

        // Penalize too many apps (fragmented attention)
        if apps.len() > 10 {
            score = score.saturating_sub(15);
        } else if apps.len() > 5 {
            score = score.saturating_sub(5);
        }

        // Bonus for productive activities
        let productive_events = events.iter().filter(|e| {
            e.source == "editor" ||
            (e.source == "shell" && e.event_type == "command") ||
            e.source == "filesystem"
        }).count();

        let productivity_ratio = productive_events as f64 / events.len() as f64;
        if productivity_ratio > 0.5 {
            score = (score + 10).min(100);
        }

        score
    }

    /// Generate human-readable summary text
    fn generate_summary_text(
        &self,
        projects: &[ProjectSummary],
        apps: &[AppUsageSummary],
        activities: &[ActivitySummary],
        meetings: &[MeetingSummary],
        duration_minutes: u32,
    ) -> String {
        let mut parts = Vec::new();

        // Duration
        let hours = duration_minutes / 60;
        let mins = duration_minutes % 60;
        if hours > 0 {
            parts.push(format!("Session duration: {}h {}m", hours, mins));
        } else {
            parts.push(format!("Session duration: {}m", mins));
        }

        // Projects
        if !projects.is_empty() {
            let project_names: Vec<_> = projects.iter()
                .take(3)
                .map(|p| p.name.clone())
                .collect();
            parts.push(format!("Projects: {}", project_names.join(", ")));
        }

        // Top apps
        if !apps.is_empty() {
            let app_names: Vec<_> = apps.iter()
                .take(3)
                .map(|a| a.app_name.clone())
                .collect();
            parts.push(format!("Main apps: {}", app_names.join(", ")));
        }

        // Meetings
        if !meetings.is_empty() {
            let meeting_count = meetings.len();
            let total_meeting_mins: u32 = meetings.iter().map(|m| m.duration_minutes).sum();
            parts.push(format!("{} meeting(s), {} minutes total", meeting_count, total_meeting_mins));
        }

        // Key activities
        let commits = activities.iter().filter(|a| a.activity_type == "commit").count();
        if commits > 0 {
            parts.push(format!("{} commit(s)", commits));
        }

        parts.join(". ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_config_default() {
        let config = SummaryConfig::default();
        assert_eq!(config.min_session_duration, 15);
        assert!(!config.include_clipboard);
        assert!(config.include_urls);
    }

    #[test]
    fn test_focus_score_calculation() {
        let generator = SummaryGenerator::new(SummaryConfig::default());

        // Empty events should return 0
        let score = generator.calculate_focus_score(&[], &[], 0);
        assert_eq!(score, 0);
    }
}
