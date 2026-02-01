//! Clipboard tracking module
//!
//! Monitors the system clipboard for changes and captures text content.
//! Implements redaction for sensitive data like passwords, API keys, etc.

use arboard::Clipboard;
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

/// Maximum text length to store (prevent huge clipboard entries)
const MAX_TEXT_LENGTH: usize = 10_000;

/// Minimum text length to track (skip tiny snippets)
const MIN_TEXT_LENGTH: usize = 2;

/// Configuration for clipboard tracking
#[derive(Debug, Clone)]
pub struct ClipboardConfig {
    /// Minimum time between clipboard checks
    pub poll_interval: Duration,
    /// Whether to redact sensitive content
    pub redact_sensitive: bool,
    /// Maximum text length to store
    pub max_text_length: usize,
}

impl Default for ClipboardConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(1),
            redact_sensitive: true,
            max_text_length: MAX_TEXT_LENGTH,
        }
    }
}

/// Types of clipboard content
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ClipboardContentType {
    Text,
    Code,
    Url,
    Path,
    Command,
}

/// Information about clipboard content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardContent {
    /// The text content (may be redacted)
    pub text: String,
    /// Type of content detected
    pub content_type: ClipboardContentType,
    /// Length of original text (before any truncation)
    pub original_length: usize,
    /// Whether any redaction was applied
    pub was_redacted: bool,
    /// Hash of the original content (for deduplication)
    pub content_hash: u64,
}

/// Event emitted when clipboard content changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardChangeEvent {
    /// The new clipboard content
    pub content: ClipboardContent,
    /// Timestamp of the change
    pub timestamp: DateTime<Utc>,
    /// Source hint (which app might have set it, if known)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_app: Option<String>,
}

/// Tracks clipboard content and emits events on changes
pub struct ClipboardTracker {
    config: ClipboardConfig,
    clipboard: Option<Clipboard>,
    last_content_hash: Option<u64>,
    last_check_time: Instant,
}

impl ClipboardTracker {
    /// Create a new clipboard tracker
    pub fn new(config: ClipboardConfig) -> Self {
        let clipboard = match Clipboard::new() {
            Ok(cb) => Some(cb),
            Err(e) => {
                warn!("Failed to initialize clipboard: {}", e);
                None
            }
        };

        Self {
            config,
            clipboard,
            last_content_hash: None,
            last_check_time: Instant::now(),
        }
    }

    /// Check the clipboard and return an event if content changed
    pub fn check_clipboard(&mut self, source_app: Option<String>) -> Option<ClipboardChangeEvent> {
        // Respect poll interval
        if self.last_check_time.elapsed() < self.config.poll_interval {
            return None;
        }
        self.last_check_time = Instant::now();

        let clipboard = self.clipboard.as_mut()?;

        // Try to get text content
        let text = match clipboard.get_text() {
            Ok(t) => t,
            Err(_) => return None, // No text content or error
        };

        // Skip empty or too short text
        if text.len() < MIN_TEXT_LENGTH {
            return None;
        }

        // Calculate hash of content
        let content_hash = Self::hash_content(&text);

        // Check if content changed
        if self.last_content_hash == Some(content_hash) {
            return None;
        }

        self.last_content_hash = Some(content_hash);

        // Detect content type
        let content_type = Self::detect_content_type(&text);

        // Apply redaction if needed
        let (processed_text, was_redacted) = if self.config.redact_sensitive {
            redact_sensitive_content(&text)
        } else {
            (text.clone(), false)
        };

        // Truncate if too long
        let original_length = text.len();
        let final_text = if processed_text.len() > self.config.max_text_length {
            format!(
                "{}... [truncated, {} chars total]",
                &processed_text[..self.config.max_text_length],
                original_length
            )
        } else {
            processed_text
        };

        debug!(
            "Clipboard changed: {} chars, type: {:?}, redacted: {}",
            original_length, content_type, was_redacted
        );

        Some(ClipboardChangeEvent {
            content: ClipboardContent {
                text: final_text,
                content_type,
                original_length,
                was_redacted,
                content_hash,
            },
            timestamp: Utc::now(),
            source_app,
        })
    }

    /// Hash content for change detection
    fn hash_content(text: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }

    /// Detect the type of clipboard content
    fn detect_content_type(text: &str) -> ClipboardContentType {
        let trimmed = text.trim();

        // Check for URL
        if trimmed.starts_with("http://")
            || trimmed.starts_with("https://")
            || trimmed.starts_with("file://")
        {
            return ClipboardContentType::Url;
        }

        // Check for file path
        if trimmed.starts_with('/')
            || trimmed.starts_with('~')
            || (trimmed.len() > 2 && trimmed.chars().nth(1) == Some(':'))
        {
            // Unix path or Windows path
            return ClipboardContentType::Path;
        }

        // Check for shell command patterns
        if trimmed.starts_with("$ ")
            || trimmed.starts_with("# ")
            || trimmed.starts_with("sudo ")
            || trimmed.starts_with("cd ")
            || trimmed.starts_with("git ")
            || trimmed.starts_with("npm ")
            || trimmed.starts_with("cargo ")
            || trimmed.starts_with("docker ")
        {
            return ClipboardContentType::Command;
        }

        // Check for code patterns
        if trimmed.contains("function ")
            || trimmed.contains("fn ")
            || trimmed.contains("def ")
            || trimmed.contains("class ")
            || trimmed.contains("const ")
            || trimmed.contains("let ")
            || trimmed.contains("var ")
            || trimmed.contains("import ")
            || trimmed.contains("export ")
            || trimmed.starts_with("//")
            || trimmed.starts_with("/*")
            || trimmed.starts_with('#')
            || trimmed.contains("=> ")
            || trimmed.contains("->")
            || (trimmed.contains('{') && trimmed.contains('}'))
        {
            return ClipboardContentType::Code;
        }

        ClipboardContentType::Text
    }

    /// Check if clipboard tracking is available
    pub fn is_available(&self) -> bool {
        self.clipboard.is_some()
    }
}

// Regex patterns for sensitive content
static PASSWORD_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(password|passwd|pwd|secret|token|key|api[_-]?key|auth|bearer|credential)\s*[:=]\s*\S+").unwrap()
});

static API_KEY_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    // Common API key formats
    Regex::new(r"(?i)(sk|pk|api|key|token|secret|auth)[_-]?[a-z0-9]{20,}").unwrap()
});

static JWT_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*").unwrap()
});

static GITHUB_TOKEN_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"gh[pousr]_[a-zA-Z0-9]{36,}").unwrap()
});

static AWS_KEY_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"AKIA[0-9A-Z]{16}").unwrap()
});

static PRIVATE_KEY_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"-----BEGIN\s+(RSA\s+)?PRIVATE\s+KEY-----").unwrap()
});

static CREDIT_CARD_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    // Basic credit card pattern (not comprehensive, but catches common formats)
    Regex::new(r"\b[0-9]{4}[\s-]?[0-9]{4}[\s-]?[0-9]{4}[\s-]?[0-9]{4}\b").unwrap()
});

static SSN_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b[0-9]{3}-[0-9]{2}-[0-9]{4}\b").unwrap()
});

/// Redact sensitive content from clipboard text
fn redact_sensitive_content(text: &str) -> (String, bool) {
    let mut result = text.to_string();
    let mut was_redacted = false;

    // Check if entire content looks like a password/secret (single line, no spaces, alphanumeric+special)
    let trimmed = text.trim();
    if !trimmed.contains('\n')
        && !trimmed.contains(' ')
        && trimmed.len() >= 16
        && trimmed.len() <= 128
    {
        // Looks like a raw password or token
        let has_mixed = trimmed.chars().any(|c| c.is_ascii_lowercase())
            && trimmed.chars().any(|c| c.is_ascii_uppercase())
            && trimmed.chars().any(|c| c.is_ascii_digit());
        let has_special = trimmed.chars().any(|c| !c.is_alphanumeric());

        if has_mixed || has_special {
            return ("[REDACTED: possible password/token]".to_string(), true);
        }
    }

    // Apply pattern-based redaction
    let patterns: &[(&LazyLock<Regex>, &str)] = &[
        (&PASSWORD_PATTERN, "[REDACTED: password]"),
        (&API_KEY_PATTERN, "[REDACTED: api_key]"),
        (&JWT_PATTERN, "[REDACTED: jwt]"),
        (&GITHUB_TOKEN_PATTERN, "[REDACTED: github_token]"),
        (&AWS_KEY_PATTERN, "[REDACTED: aws_key]"),
        (&PRIVATE_KEY_PATTERN, "[REDACTED: private_key]"),
        (&CREDIT_CARD_PATTERN, "[REDACTED: card_number]"),
        (&SSN_PATTERN, "[REDACTED: ssn]"),
    ];

    for (pattern, replacement) in patterns {
        if pattern.is_match(&result) {
            result = pattern.replace_all(&result, *replacement).to_string();
            was_redacted = true;
        }
    }

    (result, was_redacted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_config_default() {
        let config = ClipboardConfig::default();
        assert_eq!(config.poll_interval, Duration::from_secs(1));
        assert!(config.redact_sensitive);
    }

    #[test]
    fn test_content_type_detection() {
        assert_eq!(
            ClipboardTracker::detect_content_type("https://example.com"),
            ClipboardContentType::Url
        );
        assert_eq!(
            ClipboardTracker::detect_content_type("/home/user/file.txt"),
            ClipboardContentType::Path
        );
        assert_eq!(
            ClipboardTracker::detect_content_type("git commit -m 'test'"),
            ClipboardContentType::Command
        );
        assert_eq!(
            ClipboardTracker::detect_content_type("function foo() { return 1; }"),
            ClipboardContentType::Code
        );
        assert_eq!(
            ClipboardTracker::detect_content_type("Hello, world!"),
            ClipboardContentType::Text
        );
    }

    #[test]
    fn test_redact_password() {
        let (result, redacted) = redact_sensitive_content("password=mysecret123");
        assert!(redacted);
        assert!(result.contains("[REDACTED"));
    }

    #[test]
    fn test_redact_api_key() {
        let (result, redacted) =
            redact_sensitive_content("api_key_abcdef123456789012345");
        assert!(redacted);
        assert!(result.contains("[REDACTED"));
    }

    #[test]
    fn test_redact_jwt() {
        let (result, redacted) = redact_sensitive_content(
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U"
        );
        assert!(redacted);
        assert!(result.contains("[REDACTED"));
    }

    #[test]
    fn test_redact_github_token() {
        let (result, redacted) =
            redact_sensitive_content("ghp_abcdefghijklmnopqrstuvwxyz123456");
        assert!(redacted);
        assert!(result.contains("[REDACTED"));
    }

    #[test]
    fn test_redact_credit_card() {
        let (result, redacted) = redact_sensitive_content("Card: 4111-1111-1111-1111");
        assert!(redacted);
        assert!(result.contains("[REDACTED"));
    }

    #[test]
    fn test_no_redaction_needed() {
        let text = "This is just normal text with no secrets.";
        let (result, redacted) = redact_sensitive_content(text);
        assert!(!redacted);
        assert_eq!(result, text);
    }

    #[test]
    fn test_raw_password_detection() {
        // A string that looks like a password (mixed case + special chars, no spaces, 16+ chars)
        let (result, redacted) = redact_sensitive_content("MyP@ssw0rd!2024X");
        assert!(redacted);
        assert!(result.contains("[REDACTED"));
    }

    #[test]
    fn test_hash_consistency() {
        let hash1 = ClipboardTracker::hash_content("hello");
        let hash2 = ClipboardTracker::hash_content("hello");
        let hash3 = ClipboardTracker::hash_content("world");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
