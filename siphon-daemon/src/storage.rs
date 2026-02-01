//! SQLite event storage

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// Event source types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSource {
    Shell,
    Editor,
    Filesystem,
    Git,
    Browser,
    Window,
    Clipboard,
    Hotkey,
    Meeting,
}

impl std::fmt::Display for EventSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventSource::Shell => write!(f, "shell"),
            EventSource::Editor => write!(f, "editor"),
            EventSource::Filesystem => write!(f, "filesystem"),
            EventSource::Git => write!(f, "git"),
            EventSource::Browser => write!(f, "browser"),
            EventSource::Window => write!(f, "window"),
            EventSource::Clipboard => write!(f, "clipboard"),
            EventSource::Hotkey => write!(f, "hotkey"),
            EventSource::Meeting => write!(f, "meeting"),
        }
    }
}

/// Shell command event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellEventData {
    pub command: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub cwd: String,
    #[serde(default)]
    pub git_branch: Option<String>,
}

/// Editor event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorEventData {
    pub action: String, // file_open, file_save, file_close, edit
    pub file_path: String,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub lines_changed: Option<i32>,
}

/// Generic event structure stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub event_type: String,
    pub event_data: String, // JSON blob
    #[serde(default)]
    pub project: Option<String>,
}

/// Event store backed by SQLite
pub struct EventStore {
    conn: Connection,
    db_path: PathBuf,
}

impl EventStore {
    /// Create a new event store, initializing the database if needed
    pub fn new() -> Result<Self> {
        let db_path = Self::default_db_path();

        // Create parent directory if it doesn't exist
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(&db_path)?;
        let store = Self { conn, db_path };
        store.init_schema()?;
        Ok(store)
    }

    /// Get the default database path (~/.siphon/events.db)
    fn default_db_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".siphon")
            .join("events.db")
    }

    /// Get the database path
    pub fn db_path(&self) -> &PathBuf {
        &self.db_path
    }

    /// Initialize database schema
    fn init_schema(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                source TEXT NOT NULL,
                event_type TEXT NOT NULL,
                event_data TEXT NOT NULL,
                project TEXT,
                metadata TEXT
            )",
            [],
        )?;

        // Create indexes
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_source ON events(source)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_project ON events(project)",
            [],
        )?;

        Ok(())
    }

    /// Insert a new event
    pub fn insert_event(
        &self,
        source: EventSource,
        event_type: &str,
        event_data: &str,
        project: Option<&str>,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT INTO events (id, timestamp, source, event_type, event_data, project)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                timestamp,
                source.to_string(),
                event_type,
                event_data,
                project
            ],
        )?;

        Ok(id)
    }

    /// Get events within a time range
    pub fn get_events_since(&self, since: DateTime<Utc>, limit: Option<u32>) -> Result<Vec<Event>> {
        let limit = limit.unwrap_or(1000);
        let since_str = since.to_rfc3339();

        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, source, event_type, event_data, project
             FROM events
             WHERE timestamp >= ?1
             ORDER BY timestamp DESC
             LIMIT ?2",
        )?;

        let events = stmt
            .query_map(params![since_str, limit], |row| {
                Ok(Event {
                    id: row.get(0)?,
                    timestamp: row
                        .get::<_, String>(1)?
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                    source: row.get(2)?,
                    event_type: row.get(3)?,
                    event_data: row.get(4)?,
                    project: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(events)
    }

    /// Get recent events (last N hours)
    pub fn get_recent_events(&self, hours: u32) -> Result<Vec<Event>> {
        let since = Utc::now() - chrono::Duration::hours(hours as i64);
        self.get_events_since(since, None)
    }

    /// Get event count by source
    pub fn get_stats(&self) -> Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT source, COUNT(*) as count
             FROM events
             GROUP BY source
             ORDER BY count DESC",
        )?;

        let stats = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>>>()?;

        Ok(stats)
    }

    /// Get total event count
    pub fn get_total_count(&self) -> Result<i64> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))?;
        Ok(count)
    }

    /// Delete events older than the specified number of days
    /// Returns the number of events deleted
    pub fn cleanup_old_events(&self, retention_days: u32) -> Result<usize> {
        let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);
        let cutoff_str = cutoff.to_rfc3339();

        let deleted = self.conn.execute(
            "DELETE FROM events WHERE timestamp < ?1",
            params![cutoff_str],
        )?;

        Ok(deleted)
    }

    /// Vacuum the database to reclaim space after deletions
    pub fn vacuum(&self) -> Result<()> {
        self.conn.execute("VACUUM", [])?;
        Ok(())
    }

    /// Get the database file size in bytes
    pub fn get_db_size(&self) -> Result<u64> {
        let metadata = std::fs::metadata(&self.db_path)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        Ok(metadata.len())
    }

    /// Get the oldest and newest event timestamps
    pub fn get_event_time_range(&self) -> Result<Option<(DateTime<Utc>, DateTime<Utc>)>> {
        let result: Option<(String, String)> = self.conn.query_row(
            "SELECT MIN(timestamp), MAX(timestamp) FROM events",
            [],
            |row| {
                let min: Option<String> = row.get(0)?;
                let max: Option<String> = row.get(1)?;
                match (min, max) {
                    (Some(min), Some(max)) => Ok(Some((min, max))),
                    _ => Ok(None),
                }
            },
        )?;

        match result {
            Some((min, max)) => {
                let min_dt = min.parse().unwrap_or_else(|_| Utc::now());
                let max_dt = max.parse().unwrap_or_else(|_| Utc::now());
                Ok(Some((min_dt, max_dt)))
            }
            None => Ok(None),
        }
    }

    /// Get event counts grouped by day for the last N days
    pub fn get_daily_counts(&self, days: u32) -> Result<Vec<(String, i64)>> {
        let since = Utc::now() - chrono::Duration::days(days as i64);
        let since_str = since.to_rfc3339();

        let mut stmt = self.conn.prepare(
            "SELECT DATE(timestamp) as day, COUNT(*) as count
             FROM events
             WHERE timestamp >= ?1
             GROUP BY day
             ORDER BY day DESC",
        )?;

        let counts = stmt
            .query_map(params![since_str], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>>>()?;

        Ok(counts)
    }
}
