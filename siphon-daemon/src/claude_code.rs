//! Claude Code session tracking module
//!
//! Monitors ~/.claude/ for Claude Code activity across all projects.
//! Tracks prompts, sessions, and tool usage by tailing JSONL files.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tracing::{debug, info};

/// Maximum prompt preview length to store
const MAX_PROMPT_PREVIEW: usize = 200;

/// Configuration for Claude Code tracking
#[derive(Debug, Clone)]
pub struct ClaudeCodeConfig {
    /// Path to ~/.claude directory
    pub claude_dir: PathBuf,
    /// How often to poll for changes
    pub poll_interval: Duration,
    /// Whether to track individual tool_use events from conversations
    pub track_tool_use: bool,
}

impl Default for ClaudeCodeConfig {
    fn default() -> Self {
        let claude_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".claude");
        Self {
            claude_dir,
            poll_interval: Duration::from_secs(5),
            track_tool_use: true,
        }
    }
}

/// A Claude Code event ready to be stored
#[derive(Debug, Clone)]
pub struct ClaudeCodeEvent {
    pub event_type: String,
    pub data: serde_json::Value,
    pub project: Option<String>,
}

/// Data for a claude_prompt event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudePromptData {
    pub session_id: String,
    pub project: String,
    pub prompt_preview: String,
    pub has_pasted_content: bool,
    pub timestamp_ms: u64,
}

/// Data for a claude_session_start event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeSessionData {
    pub session_id: String,
    pub pid: u64,
    pub cwd: String,
    pub kind: String,
    pub entrypoint: String,
    pub started_at_ms: u64,
}

/// Data for a claude_session_end event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeSessionEndData {
    pub session_id: String,
    pub pid: u64,
    pub cwd: String,
    pub duration_minutes: u32,
    pub started_at_ms: u64,
    pub ended_at_ms: u64,
}

/// Data for a claude_tool_use event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeToolUseData {
    pub session_id: String,
    pub tool_name: String,
    pub project: String,
}

/// Tracks state for an active Claude Code session
#[derive(Debug, Clone)]
struct ActiveSession {
    session_id: String,
    pid: u64,
    cwd: String,
    started_at_ms: u64,
    /// Byte offset into the conversation JSONL file
    conversation_offset: u64,
    /// Path to the conversation JSONL file (if found)
    conversation_path: Option<PathBuf>,
}

/// Monitors Claude Code activity by polling ~/.claude/ files
pub struct ClaudeCodeTracker {
    config: ClaudeCodeConfig,
    /// Byte offset of last read position in history.jsonl
    history_offset: u64,
    /// Set of session IDs we've already emitted session_start for
    known_sessions: HashSet<String>,
    /// Active sessions keyed by session_id
    active_sessions: HashMap<String, ActiveSession>,
    /// Last poll time
    last_check_time: Instant,
}

impl ClaudeCodeTracker {
    /// Create a new tracker, starting from the current end of history.jsonl
    pub fn new(config: ClaudeCodeConfig) -> Self {
        // Start from the end of history.jsonl so we don't ingest old data
        let history_path = config.claude_dir.join("history.jsonl");
        let history_offset = std::fs::metadata(&history_path)
            .map(|m| m.len())
            .unwrap_or(0);

        debug!(
            "Claude Code tracker initialized, history offset: {}",
            history_offset
        );

        Self {
            config,
            history_offset,
            known_sessions: HashSet::new(),
            active_sessions: HashMap::new(),
            last_check_time: Instant::now() - Duration::from_secs(60), // trigger immediate first poll
        }
    }

    /// Poll for new Claude Code events. Returns a vec of events to store.
    pub fn poll(&mut self) -> Vec<ClaudeCodeEvent> {
        if self.last_check_time.elapsed() < self.config.poll_interval {
            return Vec::new();
        }
        self.last_check_time = Instant::now();

        let mut events = Vec::new();

        // 1. Check for new prompts in history.jsonl
        events.extend(self.check_history());

        // 2. Check for new/ended sessions
        events.extend(self.check_sessions());

        // 3. Check conversations for tool_use (if enabled)
        if self.config.track_tool_use {
            events.extend(self.check_conversations());
        }

        events
    }

    /// Read new lines from history.jsonl and emit prompt events
    fn check_history(&mut self) -> Vec<ClaudeCodeEvent> {
        let history_path = self.config.claude_dir.join("history.jsonl");
        let mut events = Vec::new();

        let file = match File::open(&history_path) {
            Ok(f) => f,
            Err(_) => return events,
        };

        let file_len = file.metadata().map(|m| m.len()).unwrap_or(0);
        if file_len <= self.history_offset {
            return events;
        }

        let mut reader = BufReader::new(file);
        if reader.seek(SeekFrom::Start(self.history_offset)).is_err() {
            return events;
        }

        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    self.history_offset += n as u64;

                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    if let Ok(entry) = serde_json::from_str::<serde_json::Value>(trimmed) {
                        if let Some(event) = self.parse_history_entry(&entry) {
                            events.push(event);
                        }
                    }
                }
                Err(_) => break,
            }
        }

        events
    }

    /// Parse a single history.jsonl entry into a prompt event
    fn parse_history_entry(&self, entry: &serde_json::Value) -> Option<ClaudeCodeEvent> {
        let display = entry.get("display")?.as_str()?;
        let project_path = entry.get("project")?.as_str()?;
        let session_id = entry.get("sessionId")?.as_str()?;
        let timestamp = entry.get("timestamp")?.as_u64()?;

        // Skip slash commands and empty prompts
        if display.starts_with('/') || display.trim().is_empty() {
            return None;
        }

        // Extract project name from path (last component)
        let project_name = std::path::Path::new(project_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| project_path.to_string());

        let has_pasted_content = entry
            .get("pastedContents")
            .and_then(|v| v.as_object())
            .map(|o| !o.is_empty())
            .unwrap_or(false);

        // Truncate prompt preview
        let prompt_preview = if display.len() > MAX_PROMPT_PREVIEW {
            format!("{}...", &display[..MAX_PROMPT_PREVIEW])
        } else {
            display.to_string()
        };

        let data = ClaudePromptData {
            session_id: session_id.to_string(),
            project: project_name.clone(),
            prompt_preview,
            has_pasted_content,
            timestamp_ms: timestamp,
        };

        Some(ClaudeCodeEvent {
            event_type: "claude_prompt".to_string(),
            data: serde_json::to_value(data).ok()?,
            project: Some(project_name),
        })
    }

    /// Scan ~/.claude/sessions/ for new or ended sessions
    fn check_sessions(&mut self) -> Vec<ClaudeCodeEvent> {
        let sessions_dir = self.config.claude_dir.join("sessions");
        let mut events = Vec::new();

        let entries = match std::fs::read_dir(&sessions_dir) {
            Ok(e) => e,
            Err(_) => return events,
        };

        let mut current_pids: HashSet<String> = HashSet::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }

            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let session: serde_json::Value = match serde_json::from_str(&content) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let session_id = match session.get("sessionId").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => continue,
            };

            current_pids.insert(session_id.clone());

            // New session?
            if !self.known_sessions.contains(&session_id) {
                self.known_sessions.insert(session_id.clone());

                let pid = session.get("pid").and_then(|v| v.as_u64()).unwrap_or(0);
                let cwd = session
                    .get("cwd")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let started_at = session
                    .get("startedAt")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let kind = session
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .unwrap_or("interactive")
                    .to_string();
                let entrypoint = session
                    .get("entrypoint")
                    .and_then(|v| v.as_str())
                    .unwrap_or("cli")
                    .to_string();

                let project_name = std::path::Path::new(&cwd)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                // Find the conversation JSONL file
                let conversation_path = self.find_conversation_file(&cwd, &session_id);

                // Check if the process is actually still alive
                if is_pid_alive(pid as i32) {
                    info!(
                        "New Claude Code session: {} in {} (pid {})",
                        &session_id[..8],
                        project_name,
                        pid
                    );

                    let data = ClaudeSessionData {
                        session_id: session_id.clone(),
                        pid,
                        cwd: cwd.clone(),
                        kind,
                        entrypoint,
                        started_at_ms: started_at,
                    };

                    events.push(ClaudeCodeEvent {
                        event_type: "claude_session_start".to_string(),
                        data: serde_json::to_value(&data).unwrap_or_default(),
                        project: Some(project_name),
                    });

                    self.active_sessions.insert(
                        session_id,
                        ActiveSession {
                            session_id: data.session_id,
                            pid,
                            cwd,
                            started_at_ms: started_at,
                            conversation_offset: 0,
                            conversation_path,
                        },
                    );
                }
            }
        }

        // Check for ended sessions (PID no longer alive)
        let ended: Vec<String> = self
            .active_sessions
            .iter()
            .filter(|(_, s)| !is_pid_alive(s.pid as i32))
            .map(|(id, _)| id.clone())
            .collect();

        for session_id in ended {
            if let Some(session) = self.active_sessions.remove(&session_id) {
                let now_ms = Utc::now().timestamp_millis() as u64;
                let duration_minutes = if session.started_at_ms > 0 {
                    ((now_ms - session.started_at_ms) / 60_000) as u32
                } else {
                    0
                };

                let project_name = std::path::Path::new(&session.cwd)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                info!(
                    "Claude Code session ended: {} ({}m)",
                    &session_id[..8],
                    duration_minutes
                );

                let data = ClaudeSessionEndData {
                    session_id,
                    pid: session.pid,
                    cwd: session.cwd,
                    duration_minutes,
                    started_at_ms: session.started_at_ms,
                    ended_at_ms: now_ms,
                };

                events.push(ClaudeCodeEvent {
                    event_type: "claude_session_end".to_string(),
                    data: serde_json::to_value(&data).unwrap_or_default(),
                    project: Some(project_name),
                });
            }
        }

        events
    }

    /// Find the conversation JSONL file for a session
    fn find_conversation_file(&self, cwd: &str, session_id: &str) -> Option<PathBuf> {
        // Claude Code stores conversations at:
        // ~/.claude/projects/<slug>/<sessionId>.jsonl
        // where slug is the cwd with / replaced by -
        let slug = cwd.replace('/', "-");
        let path = self
            .config
            .claude_dir
            .join("projects")
            .join(&slug)
            .join(format!("{}.jsonl", session_id));

        if path.exists() {
            return Some(path);
        }

        // Fallback: scan projects dir for this session ID
        let projects_dir = self.config.claude_dir.join("projects");
        if let Ok(entries) = std::fs::read_dir(&projects_dir) {
            for entry in entries.flatten() {
                let candidate = entry.path().join(format!("{}.jsonl", session_id));
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }

        None
    }

    /// Check active session conversations for new tool_use events
    fn check_conversations(&mut self) -> Vec<ClaudeCodeEvent> {
        let mut events = Vec::new();

        // Collect session info needed for file lookup to avoid borrow issues
        let sessions_needing_path: Vec<(String, String, String)> = self
            .active_sessions
            .iter()
            .filter(|(_, s)| s.conversation_path.is_none())
            .map(|(id, s)| (id.clone(), s.cwd.clone(), s.session_id.clone()))
            .collect();

        // Resolve conversation paths for sessions that don't have them yet
        for (id, cwd, session_id) in sessions_needing_path {
            let path = self.find_conversation_file(&cwd, &session_id);
            if let Some(session) = self.active_sessions.get_mut(&id) {
                session.conversation_path = path;
            }
        }

        // Now process all active sessions
        let session_ids: Vec<String> = self.active_sessions.keys().cloned().collect();

        for session_id in session_ids {
            let (conv_path, offset, project_name) = {
                let session = match self.active_sessions.get(&session_id) {
                    Some(s) => s,
                    None => continue,
                };
                let conv_path = match &session.conversation_path {
                    Some(p) => p.clone(),
                    None => continue,
                };
                let project_name = std::path::Path::new(&session.cwd)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                (conv_path, session.conversation_offset, project_name)
            };

            let file = match File::open(&conv_path) {
                Ok(f) => f,
                Err(_) => continue,
            };

            let file_len = file.metadata().map(|m| m.len()).unwrap_or(0);
            if file_len <= offset {
                continue;
            }

            let mut reader = BufReader::new(file);
            if reader.seek(SeekFrom::Start(offset)).is_err() {
                continue;
            }

            let mut bytes_read: u64 = 0;
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(n) => {
                        bytes_read += n as u64;

                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }

                        if let Ok(entry) = serde_json::from_str::<serde_json::Value>(trimmed) {
                            if entry.get("type").and_then(|v| v.as_str()) != Some("assistant") {
                                continue;
                            }

                            let content = match entry
                                .get("message")
                                .and_then(|m| m.get("content"))
                                .and_then(|c| c.as_array())
                            {
                                Some(c) => c,
                                None => continue,
                            };

                            for block in content {
                                if block.get("type").and_then(|v| v.as_str()) == Some("tool_use") {
                                    if let Some(tool_name) =
                                        block.get("name").and_then(|v| v.as_str())
                                    {
                                        let data = ClaudeToolUseData {
                                            session_id: session_id.clone(),
                                            tool_name: tool_name.to_string(),
                                            project: project_name.clone(),
                                        };

                                        events.push(ClaudeCodeEvent {
                                            event_type: "claude_tool_use".to_string(),
                                            data: serde_json::to_value(&data).unwrap_or_default(),
                                            project: Some(project_name.clone()),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }

            // Update the offset after processing
            if let Some(session) = self.active_sessions.get_mut(&session_id) {
                session.conversation_offset += bytes_read;
            }
        }

        events
    }

    /// Check if Claude Code tracking is available
    pub fn is_available(&self) -> bool {
        self.config.claude_dir.exists() && self.config.claude_dir.join("history.jsonl").exists()
    }
}

/// Check if a process with the given PID is still running
fn is_pid_alive(pid: i32) -> bool {
    if pid <= 0 {
        return false;
    }
    // kill(pid, 0) checks if process exists without sending a signal
    unsafe { libc::kill(pid, 0) == 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = ClaudeCodeConfig::default();
        assert_eq!(config.poll_interval, Duration::from_secs(5));
        assert!(config.track_tool_use);
    }

    #[test]
    fn test_parse_history_entry() {
        let tracker = ClaudeCodeTracker::new(ClaudeCodeConfig::default());

        let entry = serde_json::json!({
            "display": "fix the login bug",
            "project": "/Users/dev/Projects/myapp",
            "sessionId": "abc-123",
            "timestamp": 1700000000000u64,
            "pastedContents": {}
        });

        let event = tracker.parse_history_entry(&entry).unwrap();
        assert_eq!(event.event_type, "claude_prompt");
        assert_eq!(event.project, Some("myapp".to_string()));

        let data: ClaudePromptData = serde_json::from_value(event.data).unwrap();
        assert_eq!(data.prompt_preview, "fix the login bug");
        assert_eq!(data.session_id, "abc-123");
        assert!(!data.has_pasted_content);
    }

    #[test]
    fn test_skip_slash_commands() {
        let tracker = ClaudeCodeTracker::new(ClaudeCodeConfig::default());

        let entry = serde_json::json!({
            "display": "/exit",
            "project": "/Users/dev/Projects/myapp",
            "sessionId": "abc-123",
            "timestamp": 1700000000000u64,
        });

        assert!(tracker.parse_history_entry(&entry).is_none());
    }

    #[test]
    fn test_prompt_preview_truncation() {
        let tracker = ClaudeCodeTracker::new(ClaudeCodeConfig::default());

        let long_prompt = "a".repeat(300);
        let entry = serde_json::json!({
            "display": long_prompt,
            "project": "/Users/dev/Projects/myapp",
            "sessionId": "abc-123",
            "timestamp": 1700000000000u64,
            "pastedContents": {}
        });

        let event = tracker.parse_history_entry(&entry).unwrap();
        let data: ClaudePromptData = serde_json::from_value(event.data).unwrap();
        assert!(data.prompt_preview.len() <= MAX_PROMPT_PREVIEW + 3); // +3 for "..."
    }

    #[test]
    fn test_pid_alive_invalid() {
        // PID 0 and negative should return false
        assert!(!is_pid_alive(0));
        assert!(!is_pid_alive(-1));
    }

    #[test]
    fn test_pid_alive_current_process() {
        // Our own process should be alive
        let pid = std::process::id() as i32;
        assert!(is_pid_alive(pid));
    }
}
