//! Event Deduplication
//!
//! Prevents duplicate events from being stored when the same action
//! occurs multiple times in quick succession.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

/// Configuration for event deduplication
#[derive(Debug, Clone)]
pub struct DedupConfig {
    /// Time window for considering events as duplicates
    pub window: Duration,
    /// Maximum number of entries in the cache
    pub max_entries: usize,
    /// Interval for cleaning up expired entries
    pub cleanup_interval: Duration,
}

impl Default for DedupConfig {
    fn default() -> Self {
        Self {
            window: Duration::from_secs(2),       // 2 second window
            max_entries: 10000,                    // Max 10k entries
            cleanup_interval: Duration::from_secs(60), // Cleanup every minute
        }
    }
}

/// A deduplication key for events
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventKey {
    pub source: String,
    pub event_type: String,
    pub content_hash: u64,
}

impl EventKey {
    /// Create a new event key
    pub fn new(source: &str, event_type: &str, content: &str) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        content.hash(&mut hasher);
        let content_hash = hasher.finish();

        Self {
            source: source.to_string(),
            event_type: event_type.to_string(),
            content_hash,
        }
    }

    /// Create a key from shell command
    pub fn from_shell(command: &str, exit_code: i32) -> Self {
        Self::new("shell", "command", &format!("{}:{}", command, exit_code))
    }

    /// Create a key from editor event
    pub fn from_editor(action: &str, file_path: &str) -> Self {
        Self::new("editor", action, file_path)
    }

    /// Create a key from file system event
    pub fn from_filesystem(action: &str, file_path: &str) -> Self {
        Self::new("filesystem", action, file_path)
    }
}

/// Entry in the deduplication cache
#[derive(Debug, Clone)]
struct CacheEntry {
    timestamp: Instant,
    count: u32,
}

/// Event deduplicator
pub struct Deduplicator {
    config: DedupConfig,
    cache: HashMap<EventKey, CacheEntry>,
    last_cleanup: Instant,
}

impl Deduplicator {
    /// Create a new deduplicator
    pub fn new(config: DedupConfig) -> Self {
        Self {
            config,
            cache: HashMap::new(),
            last_cleanup: Instant::now(),
        }
    }

    /// Check if an event should be processed or is a duplicate
    /// Returns true if the event should be processed (not a duplicate)
    pub fn should_process(&mut self, key: &EventKey) -> bool {
        self.maybe_cleanup();

        let now = Instant::now();

        if let Some(entry) = self.cache.get_mut(key) {
            // Check if within dedup window
            if now.duration_since(entry.timestamp) < self.config.window {
                // Still within window, this is a duplicate
                entry.count += 1;
                return false;
            }

            // Outside window, update timestamp and allow
            entry.timestamp = now;
            entry.count = 1;
            true
        } else {
            // New event, add to cache
            if self.cache.len() >= self.config.max_entries {
                // Cache is full, remove oldest entries
                self.evict_oldest(self.config.max_entries / 4);
            }

            self.cache.insert(
                key.clone(),
                CacheEntry {
                    timestamp: now,
                    count: 1,
                },
            );
            true
        }
    }

    /// Get the duplicate count for an event
    pub fn get_dup_count(&self, key: &EventKey) -> u32 {
        self.cache.get(key).map(|e| e.count).unwrap_or(0)
    }

    /// Perform cleanup of expired entries if needed
    fn maybe_cleanup(&mut self) {
        if self.last_cleanup.elapsed() < self.config.cleanup_interval {
            return;
        }

        let now = Instant::now();
        let window = self.config.window;

        // Remove entries outside the window
        self.cache
            .retain(|_, entry| now.duration_since(entry.timestamp) < window * 10);

        self.last_cleanup = now;
    }

    /// Evict the oldest entries from the cache
    fn evict_oldest(&mut self, count: usize) {
        // Collect entries with timestamps
        let mut entries: Vec<_> = self
            .cache
            .iter()
            .map(|(k, v)| (k.clone(), v.timestamp))
            .collect();

        // Sort by timestamp (oldest first)
        entries.sort_by(|a, b| a.1.cmp(&b.1));

        // Remove oldest entries
        for (key, _) in entries.into_iter().take(count) {
            self.cache.remove(&key);
        }
    }

    /// Clear the entire cache
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> DedupStats {
        let total_dups: u32 = self.cache.values().map(|e| e.count.saturating_sub(1)).sum();

        DedupStats {
            cache_size: self.cache.len(),
            total_duplicates_prevented: total_dups,
        }
    }
}

/// Deduplication statistics
#[derive(Debug, Clone)]
pub struct DedupStats {
    pub cache_size: usize,
    pub total_duplicates_prevented: u32,
}

impl Default for Deduplicator {
    fn default() -> Self {
        Self::new(DedupConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_basic_dedup() {
        let mut dedup = Deduplicator::new(DedupConfig {
            window: Duration::from_millis(100),
            ..Default::default()
        });

        let key = EventKey::from_shell("ls", 0);

        // First event should be processed
        assert!(dedup.should_process(&key));

        // Immediate duplicate should be blocked
        assert!(!dedup.should_process(&key));

        // After window expires, should be processed
        sleep(Duration::from_millis(150));
        assert!(dedup.should_process(&key));
    }

    #[test]
    fn test_different_events() {
        let mut dedup = Deduplicator::default();

        let key1 = EventKey::from_shell("ls", 0);
        let key2 = EventKey::from_shell("pwd", 0);

        // Different events should both be processed
        assert!(dedup.should_process(&key1));
        assert!(dedup.should_process(&key2));
    }

    #[test]
    fn test_exit_code_differentiation() {
        let mut dedup = Deduplicator::default();

        let key_success = EventKey::from_shell("npm test", 0);
        let key_failure = EventKey::from_shell("npm test", 1);

        // Same command with different exit codes are different events
        assert!(dedup.should_process(&key_success));
        assert!(dedup.should_process(&key_failure));
    }
}
