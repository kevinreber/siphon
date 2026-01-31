//! Sensitive data redaction
//!
//! This module provides utilities for redacting sensitive information
//! from shell commands before storage. This ensures that API keys,
//! passwords, and other secrets are never persisted.

use regex::Regex;
use std::sync::LazyLock;

/// Patterns that indicate sensitive data
static SENSITIVE_PATTERNS: LazyLock<Vec<SensitivePattern>> = LazyLock::new(|| {
    vec![
        // API keys and tokens (common naming patterns)
        SensitivePattern::new(
            r"(?i)(api[_-]?key|api[_-]?token|auth[_-]?token|access[_-]?token|secret[_-]?key|bearer)\s*[=:]\s*['\x22]?([a-zA-Z0-9_.+=/\-]{8,})['\x22]?",
            "$1=[REDACTED]",
        ),
        // Environment variable assignments with sensitive names
        SensitivePattern::new(
            r"(?i)(ANTHROPIC_API_KEY|OPENAI_API_KEY|AWS_SECRET_ACCESS_KEY|AWS_ACCESS_KEY_ID|GITHUB_TOKEN|GH_TOKEN|NPM_TOKEN|DOCKER_PASSWORD|DATABASE_URL|DB_PASSWORD|REDIS_URL|STRIPE_SECRET_KEY|STRIPE_API_KEY|SENDGRID_API_KEY|TWILIO_AUTH_TOKEN|SLACK_TOKEN|DISCORD_TOKEN)\s*=\s*['\x22]?([^'\x22\s]+)['\x22]?",
            "$1=[REDACTED]",
        ),
        // Password flags in commands
        SensitivePattern::new(
            r"(?i)(-p|--password|--passwd|--pass)\s*[=\s]?\s*['\x22]?([^'\x22\s]+)['\x22]?",
            "$1 [REDACTED]",
        ),
        // mysql/psql with inline password
        SensitivePattern::new(
            r"(?i)(mysql|psql|mongosh?)\s+.*-p\s*([^'\x22\s]+)",
            "$1 ... -p[REDACTED]",
        ),
        // curl with Authorization header
        SensitivePattern::new(
            r"(?i)(-H|--header)\s*['\x22]?(Authorization:\s*(Bearer\s+)?)['\x22]?([a-zA-Z0-9_.+=/\-]+)['\x22]?",
            r#"$1 "Authorization: [REDACTED]""#,
        ),
        // curl with -u for basic auth
        SensitivePattern::new(
            r"(?i)(curl\s+.*)-u\s*['\x22]?([^'\x22\s:]+:[^'\x22\s]+)['\x22]?",
            "$1 -u [REDACTED]",
        ),
        // SSH/SCP with password (shouldn't happen but just in case)
        SensitivePattern::new(
            r"(?i)sshpass\s+-p\s*['\x22]?([^'\x22\s]+)['\x22]?",
            "sshpass -p [REDACTED]",
        ),
        // Docker login
        SensitivePattern::new(
            r"(?i)(docker\s+login\s+.*)-p\s*['\x22]?([^'\x22\s]+)['\x22]?",
            "$1 -p [REDACTED]",
        ),
        // npm/yarn with auth token inline
        SensitivePattern::new(
            r"(?i)(npm|yarn)\s+.*--_authToken\s*[=\s]?\s*['\x22]?([^'\x22\s]+)['\x22]?",
            "$1 ... --_authToken [REDACTED]",
        ),
        // Generic secret patterns (long random strings that look like secrets)
        SensitivePattern::new(
            r"(?i)(sk-|pk_live_|pk_test_|sk_live_|sk_test_|ghp_|gho_|ghu_|ghs_|ghr_|xoxb-|xoxp-|xoxa-|xoxr-)([a-zA-Z0-9_\-]{20,})",
            "[REDACTED_SECRET]",
        ),
        // AWS credentials patterns
        SensitivePattern::new(
            r"(?i)(AKIA|ABIA|ACCA|AGPA|AIDA|AIPA|ANPA|ANVA|APKA|AROA|ASCA|ASIA)[A-Z0-9]{16}",
            "[REDACTED_AWS_KEY]",
        ),
        // JWT tokens (3 base64 segments separated by dots)
        SensitivePattern::new(
            r"eyJ[a-zA-Z0-9_\-]*\.eyJ[a-zA-Z0-9_\-]*\.[a-zA-Z0-9_\-]+",
            "[REDACTED_JWT]",
        ),
        // Private keys inline (shouldn't happen but catch it)
        SensitivePattern::new(
            r"(?i)(-----BEGIN\s+(RSA\s+)?PRIVATE\s+KEY-----)",
            "[REDACTED_PRIVATE_KEY]",
        ),
        // Heroku API key pattern
        SensitivePattern::new(
            r"(?i)heroku[_-]?api[_-]?key\s*[=:]\s*['\x22]?([a-f0-9\-]{36})['\x22]?",
            "heroku_api_key=[REDACTED]",
        ),
    ]
});

/// Commands that should be completely skipped (not stored at all)
static SKIP_COMMANDS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        // Password/secret entry commands
        Regex::new(r"(?i)^(pass|gopass|1password|op)\s+").unwrap(),
        // GPG encryption with passphrase
        Regex::new(r"(?i)^gpg\s+.*--passphrase").unwrap(),
        // Commands that typically prompt for passwords
        Regex::new(r"(?i)^(sudo\s+-S|su\s+-c)").unwrap(),
    ]
});

struct SensitivePattern {
    regex: Regex,
    replacement: String,
}

impl SensitivePattern {
    fn new(pattern: &str, replacement: &str) -> Self {
        Self {
            regex: Regex::new(pattern).expect("Invalid regex pattern"),
            replacement: replacement.to_string(),
        }
    }
}

/// Result of redaction
#[derive(Debug)]
pub struct RedactionResult {
    /// The redacted command (None if command should be skipped entirely)
    pub command: Option<String>,
    /// Whether any redaction was performed
    pub was_redacted: bool,
    /// Number of redactions made
    pub redaction_count: usize,
}

/// Redact sensitive information from a command
pub fn redact_command(command: &str) -> RedactionResult {
    // Check if command should be skipped entirely
    for skip_regex in SKIP_COMMANDS.iter() {
        if skip_regex.is_match(command) {
            return RedactionResult {
                command: None,
                was_redacted: true,
                redaction_count: 1,
            };
        }
    }

    let mut result = command.to_string();
    let mut redaction_count = 0;

    // Apply each sensitive pattern
    for pattern in SENSITIVE_PATTERNS.iter() {
        if pattern.regex.is_match(&result) {
            let before = result.clone();
            result = pattern.regex.replace_all(&result, &pattern.replacement).to_string();
            if result != before {
                redaction_count += 1;
            }
        }
    }

    RedactionResult {
        command: Some(result.clone()),
        was_redacted: redaction_count > 0,
        redaction_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_redaction() {
        let result = redact_command("api_key=sk-abc123xyz-test");
        assert!(result.was_redacted);
        assert!(!result.command.unwrap().contains("sk-abc123xyz"));
    }

    #[test]
    fn test_env_var_redaction() {
        let result = redact_command("ANTHROPIC_API_KEY=sk-ant-abc123 npm start");
        assert!(result.was_redacted);
        assert!(result.command.unwrap().contains("[REDACTED]"));
    }

    #[test]
    fn test_password_flag_redaction() {
        let result = redact_command("mysql -u root -p secretpass123 mydb");
        assert!(result.was_redacted);
        assert!(!result.command.unwrap().contains("secretpass123"));
    }

    #[test]
    fn test_no_redaction_needed() {
        let result = redact_command("git status");
        assert!(!result.was_redacted);
        assert_eq!(result.command.unwrap(), "git status");
    }

    #[test]
    fn test_skip_password_manager() {
        let result = redact_command("pass show github/token");
        assert!(result.command.is_none());
    }

    #[test]
    fn test_jwt_redaction() {
        let result = redact_command("curl eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U");
        assert!(result.was_redacted);
        assert!(result.command.unwrap().contains("[REDACTED_JWT]"));
    }

    #[test]
    fn test_github_token_redaction() {
        let result = redact_command("GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx git push");
        assert!(result.was_redacted);
        assert!(!result.command.unwrap().contains("ghp_"));
    }
}
