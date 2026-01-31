//! Screenshot and Recording Triggers
//!
//! Detects key moments during development that would be good
//! to capture for content creation (screenshots, recordings).

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Trigger configuration
#[derive(Debug, Clone)]
pub struct TriggerConfig {
    /// Number of failures before triggering a "struggle" capture
    pub failure_threshold: u32,
    /// Time window for counting failures (seconds)
    pub failure_window_secs: u64,
    /// Enable OBS integration
    pub obs_integration: bool,
    /// OBS WebSocket URL
    pub obs_websocket_url: String,
    /// Enable screenshot command
    pub screenshot_enabled: bool,
    /// Screenshot command to execute
    pub screenshot_command: String,
}

impl Default for TriggerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 3,
            failure_window_secs: 300, // 5 minutes
            obs_integration: false,
            obs_websocket_url: "ws://localhost:4455".to_string(),
            screenshot_enabled: false,
            screenshot_command: "screencapture -x ~/siphon-screenshots/$(date +%Y%m%d_%H%M%S).png".to_string(),
        }
    }
}

/// Types of capture triggers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    /// Multiple failures in a row - potential debugging moment
    StruggleMoment,
    /// Success after multiple failures - breakthrough!
    Breakthrough,
    /// Long command execution - something significant happening
    LongOperation,
    /// First success with a new tool/command
    FirstSuccess,
    /// Git commit - milestone moment
    Commit,
    /// Test passing after failures
    TestFixed,
    /// Manual trigger from user
    Manual,
}

/// A triggered capture event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerEvent {
    pub trigger_type: TriggerType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub description: String,
    pub context: TriggerContext,
    pub action_taken: Option<String>,
}

/// Context for what triggered the capture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerContext {
    pub recent_commands: Vec<String>,
    pub failure_count: u32,
    pub topic: Option<String>,
    pub project: Option<String>,
}

/// Command result for tracking
#[derive(Debug, Clone)]
struct CommandResult {
    command: String,
    exit_code: i32,
    timestamp: Instant,
    duration_ms: u64,
}

/// Trigger detector
pub struct TriggerDetector {
    config: TriggerConfig,
    recent_commands: VecDeque<CommandResult>,
    known_commands: std::collections::HashSet<String>,
    last_trigger: Option<Instant>,
    trigger_cooldown: Duration,
}

impl TriggerDetector {
    /// Create a new trigger detector
    pub fn new(config: TriggerConfig) -> Self {
        Self {
            config,
            recent_commands: VecDeque::with_capacity(50),
            known_commands: std::collections::HashSet::new(),
            last_trigger: None,
            trigger_cooldown: Duration::from_secs(60), // 1 minute between triggers
        }
    }

    /// Record a command execution and check for triggers
    pub fn record_command(
        &mut self,
        command: &str,
        exit_code: i32,
        duration_ms: u64,
        project: Option<&str>,
    ) -> Option<TriggerEvent> {
        let now = Instant::now();
        let command_base = extract_command_base(command);

        // Add to recent commands
        self.recent_commands.push_back(CommandResult {
            command: command.to_string(),
            exit_code,
            timestamp: now,
            duration_ms,
        });

        // Keep only recent commands within window
        let cutoff = now - Duration::from_secs(self.config.failure_window_secs);
        while let Some(front) = self.recent_commands.front() {
            if front.timestamp < cutoff {
                self.recent_commands.pop_front();
            } else {
                break;
            }
        }

        // Check for triggers (with cooldown)
        if let Some(last) = self.last_trigger {
            if now.duration_since(last) < self.trigger_cooldown {
                return None;
            }
        }

        // Check for breakthrough (success after failures)
        if exit_code == 0 {
            let recent_failures = self
                .recent_commands
                .iter()
                .rev()
                .skip(1) // Skip current command
                .take_while(|c| c.exit_code != 0)
                .count();

            if recent_failures >= self.config.failure_threshold as usize {
                self.last_trigger = Some(now);
                return Some(self.create_trigger_event(
                    TriggerType::Breakthrough,
                    format!(
                        "Success after {} failures with {}",
                        recent_failures, command_base
                    ),
                    project,
                ));
            }

            // Check for first success with new command
            if !self.known_commands.contains(&command_base) {
                self.known_commands.insert(command_base.clone());
                // Only trigger for "significant" commands
                if is_significant_command(&command_base) {
                    self.last_trigger = Some(now);
                    return Some(self.create_trigger_event(
                        TriggerType::FirstSuccess,
                        format!("First successful use of {}", command_base),
                        project,
                    ));
                }
            }
        }

        // Check for struggle moment (multiple failures)
        let recent_failure_count = self
            .recent_commands
            .iter()
            .filter(|c| c.exit_code != 0)
            .count();

        if recent_failure_count >= self.config.failure_threshold as usize {
            // Only trigger if this is a new failure (not already in struggle)
            let prev_failure_count = self
                .recent_commands
                .iter()
                .rev()
                .skip(1)
                .filter(|c| c.exit_code != 0)
                .count();

            if prev_failure_count < self.config.failure_threshold as usize {
                self.last_trigger = Some(now);
                return Some(self.create_trigger_event(
                    TriggerType::StruggleMoment,
                    format!("{} failures in the last {} minutes",
                        recent_failure_count,
                        self.config.failure_window_secs / 60
                    ),
                    project,
                ));
            }
        }

        // Check for long operation
        if duration_ms > 30000 && exit_code == 0 {
            // 30+ seconds
            self.last_trigger = Some(now);
            return Some(self.create_trigger_event(
                TriggerType::LongOperation,
                format!("{} completed after {:.1}s", command_base, duration_ms as f64 / 1000.0),
                project,
            ));
        }

        // Check for git commit
        if command.starts_with("git commit") && exit_code == 0 {
            self.last_trigger = Some(now);
            return Some(self.create_trigger_event(
                TriggerType::Commit,
                "Git commit created".to_string(),
                project,
            ));
        }

        // Check for test fix
        if is_test_command(command) && exit_code == 0 {
            let prev_test_failures = self
                .recent_commands
                .iter()
                .rev()
                .skip(1)
                .filter(|c| is_test_command(&c.command) && c.exit_code != 0)
                .count();

            if prev_test_failures >= 2 {
                self.last_trigger = Some(now);
                return Some(self.create_trigger_event(
                    TriggerType::TestFixed,
                    format!("Tests passing after {} failures", prev_test_failures),
                    project,
                ));
            }
        }

        None
    }

    /// Create a trigger event
    fn create_trigger_event(
        &self,
        trigger_type: TriggerType,
        description: String,
        project: Option<&str>,
    ) -> TriggerEvent {
        let recent_cmds: Vec<String> = self
            .recent_commands
            .iter()
            .rev()
            .take(10)
            .map(|c| c.command.clone())
            .collect();

        let failure_count = self
            .recent_commands
            .iter()
            .filter(|c| c.exit_code != 0)
            .count() as u32;

        TriggerEvent {
            trigger_type,
            timestamp: chrono::Utc::now(),
            description,
            context: TriggerContext {
                recent_commands: recent_cmds,
                failure_count,
                topic: detect_topic_from_commands(&self.recent_commands),
                project: project.map(String::from),
            },
            action_taken: None,
        }
    }

    /// Execute a capture action based on trigger
    pub fn execute_capture(&self, trigger: &TriggerEvent) -> Result<String, String> {
        if self.config.screenshot_enabled {
            // Execute screenshot command
            let output = std::process::Command::new("sh")
                .arg("-c")
                .arg(&self.config.screenshot_command)
                .output();

            match output {
                Ok(out) if out.status.success() => {
                    return Ok("Screenshot captured".to_string());
                }
                Ok(out) => {
                    return Err(format!(
                        "Screenshot failed: {}",
                        String::from_utf8_lossy(&out.stderr)
                    ));
                }
                Err(e) => {
                    return Err(format!("Failed to execute screenshot: {}", e));
                }
            }
        }

        // TODO: OBS integration would go here
        // if self.config.obs_integration {
        //     // Send WebSocket message to OBS to start recording or take screenshot
        // }

        Ok("No capture action configured".to_string())
    }

    /// Manual trigger
    pub fn manual_trigger(&mut self, description: &str, project: Option<&str>) -> TriggerEvent {
        self.last_trigger = Some(Instant::now());
        self.create_trigger_event(
            TriggerType::Manual,
            description.to_string(),
            project,
        )
    }
}

/// Extract base command (first word or two)
fn extract_command_base(command: &str) -> String {
    let parts: Vec<&str> = command.split_whitespace().collect();
    match parts.len() {
        0 => String::new(),
        1 => parts[0].to_string(),
        _ => {
            // For commands like "git commit", "npm run", etc.
            let first = parts[0];
            if ["git", "npm", "yarn", "cargo", "docker", "kubectl", "go"].contains(&first) {
                format!("{} {}", first, parts[1])
            } else {
                first.to_string()
            }
        }
    }
}

/// Check if command is a test command
fn is_test_command(command: &str) -> bool {
    let test_patterns = [
        "test", "jest", "pytest", "mocha", "cargo test", "go test",
        "npm test", "yarn test", "pnpm test", "vitest", "playwright",
    ];
    let cmd_lower = command.to_lowercase();
    test_patterns.iter().any(|p| cmd_lower.contains(p))
}

/// Check if command is significant (worth triggering on first use)
fn is_significant_command(command_base: &str) -> bool {
    let significant = [
        "docker", "kubectl", "terraform", "ansible", "helm",
        "aws", "gcloud", "az", "cargo build", "cargo run",
        "npm run", "yarn", "go build", "go run",
    ];
    significant.iter().any(|s| command_base.starts_with(s))
}

/// Detect topic from recent commands
fn detect_topic_from_commands(commands: &VecDeque<CommandResult>) -> Option<String> {
    let topics = [
        ("docker", "docker"),
        ("kubectl", "kubernetes"),
        ("npm", "node"),
        ("cargo", "rust"),
        ("go ", "go"),
        ("python", "python"),
        ("git", "git"),
        ("test", "testing"),
    ];

    let all_commands: String = commands.iter().map(|c| c.command.as_str()).collect::<Vec<_>>().join(" ");
    let cmd_lower = all_commands.to_lowercase();

    for (pattern, topic) in topics {
        if cmd_lower.contains(pattern) {
            return Some(topic.to_string());
        }
    }

    None
}

impl Default for TriggerDetector {
    fn default() -> Self {
        Self::new(TriggerConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_breakthrough_detection() {
        let mut detector = TriggerDetector::new(TriggerConfig {
            failure_threshold: 2,
            ..Default::default()
        });

        // Record some failures
        assert!(detector.record_command("npm test", 1, 100, None).is_none());
        assert!(detector.record_command("npm test", 1, 100, None).is_none());

        // Success after failures should trigger breakthrough
        let trigger = detector.record_command("npm test", 0, 100, None);
        assert!(trigger.is_some());
        assert!(matches!(trigger.unwrap().trigger_type, TriggerType::Breakthrough));
    }

    #[test]
    fn test_struggle_detection() {
        let mut detector = TriggerDetector::new(TriggerConfig {
            failure_threshold: 3,
            ..Default::default()
        });

        detector.record_command("npm test", 1, 100, None);
        detector.record_command("npm test", 1, 100, None);

        // Third failure should trigger struggle
        let trigger = detector.record_command("npm test", 1, 100, None);
        assert!(trigger.is_some());
        assert!(matches!(trigger.unwrap().trigger_type, TriggerType::StruggleMoment));
    }

    #[test]
    fn test_extract_command_base() {
        assert_eq!(extract_command_base("git commit -m 'test'"), "git commit");
        assert_eq!(extract_command_base("npm run build"), "npm run");
        assert_eq!(extract_command_base("ls -la"), "ls");
    }
}
