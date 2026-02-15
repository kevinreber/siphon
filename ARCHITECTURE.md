# Architecture

This document describes the system architecture of Siphon — how the pieces fit together, why they're structured this way, and how data flows through the system.

## System Overview

Siphon is a two-layer system: a **capture layer** that collects development events, and an **analysis layer** that turns those events into content ideas. The capture layer can operate in two modes — on-demand (CLI) or continuous (daemon).

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Event Sources                                │
├─────────────┬──────────────┬──────────────┬────────────────────────┤
│ Shell Hook  │  VS Code Ext │  File Watch  │  Browser (planned)     │
│ (zsh/bash)  │  (TypeScript)│  (notify)    │                        │
└──────┬──────┴──────┬───────┴──────┬───────┴───────┬────────────────┘
       │             │              │               │
       ▼             ▼              ▼               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     Ingestion (HTTP API)                             │
│                     POST /events/shell                               │
│                     POST /events/editor                              │
│                     Internal file watcher                            │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     Event Storage (SQLite)                           │
│                     ~/.siphon/events.db                              │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │ events table                                                 │    │
│  │ id | timestamp | source | event_type | event_data | project  │    │
│  └─────────────────────────────────────────────────────────────┘    │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                    ┌──────────┼──────────┐
                    ▼                     ▼
          ┌─────────────────┐   ┌─────────────────┐
          │  Query API      │   │  Analysis API   │
          │  GET /events    │   │  GET /clusters  │
          │  GET /recent    │   │  GET /ideas     │
          └─────────────────┘   └─────────────────┘
               │    │                     │
               │    │                     ▼
               │    │           ┌─────────────────┐
               │    │           │  Content        │
               │    │           │  Generator (TS) │
               │    │           └─────────────────┘
               ▼    ▼
    ┌────────────┐ ┌─────────────────┐
    │siphon-ctl  │ │  Web Dashboard  │
    │(Rust CLI)  │ │  (Browser UI)   │
    └────────────┘ └─────────────────┘
```

## Components

### Rust Daemon (`siphon-daemon`)

The daemon is the core of the continuous capture system. It runs in the background with minimal resource usage and is responsible for:

**Event collection** from multiple sources simultaneously. The file system watcher uses the `notify` crate for OS-native file events (kqueue on macOS, inotify on Linux). Shell events arrive via HTTP from the zsh hook. Editor events arrive via HTTP from the VS Code extension.

**Event storage** in a local SQLite database. We chose SQLite because the write patterns are append-heavy with occasional reads, the data is inherently local, and there's no need for network database complexity. The `rusqlite` crate with bundled SQLite means zero external dependencies.

**Event clustering** via a lightweight topic extraction algorithm. Events are grouped by inferred topic (based on commands, file extensions, URLs visited) and scored by intensity (event count, time span, source diversity). This is intentionally simple — the heavy analysis lives in the TypeScript layer where LLM integration is easier.

**HTTP API** using Axum for both event ingestion and querying. The API runs on `127.0.0.1:9847` (localhost only) so external access isn't possible. CORS is enabled for the VS Code extension's webview requests.

**Web dashboard serving** via `tower-http`'s `ServeDir`. The daemon serves the web dashboard as static files on the same port. API routes take priority; unmatched paths fall back to static file serving from `~/.siphon/ui/` (or the bundled `siphon-ui/` directory during development). This means the dashboard requires zero additional processes or configuration.

#### Daemon Process Model

```
Main Thread
├── Axum HTTP Server (async, tokio)
├── File System Watcher (sync notify → async bridge)
├── Shell History Importer (startup, one-shot)
└── Event Broadcast Channel (tokio::sync::broadcast)
    └── Storage Writer (receives from broadcast, writes to SQLite)
```

The broadcast channel decouples event sources from storage. Any component can emit events, and the storage writer processes them independently. This means a slow disk write won't block the API from accepting new events.

### TypeScript CLI (`siphon-cli`)

The CLI is the original on-demand tool. It reads directly from shell history files, browser SQLite databases, git logs, and file modification times — no daemon required. This is useful for quick analysis after a work session without needing the daemon running.

**Collectors** read from native data sources:
- Shell: Parses `~/.zsh_history` or `~/.bash_history` with timestamp support
- Browser: Reads Chrome/Firefox SQLite history databases (opens a copy to avoid locking)
- Git: Runs `git log` with timestamp filters
- Files: Uses `fs.stat` for modification times

**Analyzer** clusters events by topic using keyword extraction, categorization by tool/language, and temporal proximity. It identifies "learning signals" — patterns that indicate the developer was learning or debugging rather than just executing known tasks.

**Generator** produces three output formats:
- Summary: Quick list of content ideas with confidence scores
- Detailed: Full timeline breakdown with event evidence
- Script: Complete video storyboard with hook, sections, talking points, and SEO keywords

### Shell Hook (`siphon-hook.zsh`)

A lightweight zsh hook that sends commands to the daemon in real-time. Uses `preexec` and `precmd` hooks to capture:
- The command string
- Exit code
- Execution duration (millisecond precision via `$EPOCHREALTIME`)
- Current working directory (used to infer project)

Events are sent via `curl` in a background subshell (`&!`) so there's zero impact on shell responsiveness. If the daemon isn't running, the curl fails silently.

### VS Code Extension

A TypeScript extension that tracks editor-level activity:
- File opens and closes
- File saves
- Significant edits (5+ lines changed, to avoid noise from minor edits)
- Debug session starts and stops

Events are sent to the daemon's HTTP API. The extension includes a status bar item showing connection state and commands for viewing activity and ideas directly in VS Code.

### Web Dashboard (`siphon-ui`)

A lightweight web interface served directly by the daemon at `http://localhost:9847`. Built with plain HTML, CSS, and vanilla JavaScript — no build step, no framework, no npm dependencies.

The dashboard displays:
- **Session state** — active/idle/offline status badge
- **Stats overview** — total events, session duration, focus score, database size
- **Daily activity chart** — bar chart of events per day over the last 14 days
- **Events by source** — horizontal bar breakdown (shell, editor, filesystem, etc.)
- **Session summary** — projects, applications, key activities, meetings
- **Recent events** — scrollable list of the last 50 events with source indicators
- **Active window** — current focused application

The dashboard auto-refreshes every 15 seconds by polling the daemon's existing API endpoints (`/stats`, `/events/recent`, `/summary`, `/session`, `/storage`, `/window`). No WebSocket connection is needed.

UI files are resolved in priority order: `~/.siphon/ui/` > binary-adjacent `ui/` directory > `siphon-ui/` relative to CWD (for development).

## Data Flow

### Real-time capture (daemon mode)

```
1. Developer runs: kubectl get pods --all-namespaces
2. Zsh preexec hook fires, records start time
3. Command completes, precmd hook fires
4. Hook sends POST to daemon: { command, exit_code, duration_ms, cwd }
5. Daemon stores event in SQLite with auto-detected project
6. Event broadcast to any active subscribers
```

### On-demand capture (CLI mode)

```
1. Developer finishes a 2-hour debugging session
2. Runs: siphon capture --time 2h
3. CLI reads last 2 hours from:
   - ~/.zsh_history (parsed with timestamps)
   - Chrome history SQLite (copied, not locked)
   - git log for repos in current directory
   - File modification times in current directory
4. Events are clustered by topic
5. Learning signals are detected
6. Content ideas are generated and printed
```

### Analysis pipeline

```
Raw Events
    │
    ▼
Topic Extraction
    │  (command → "kubernetes", file extension → "typescript",
    │   URL → "documentation", etc.)
    ▼
Temporal Clustering
    │  (events within 30 minutes on same topic → cluster)
    ▼
Learning Signal Detection
    │  (multiple searches = research phase,
    │   Stack Overflow visits = troubleshooting,
    │   trial-and-error commands = debugging)
    ▼
Confidence Scoring
    │  (high: 10+ events AND 60+ minutes,
    │   medium: 5+ events,
    │   low: everything else)
    ▼
Content Generation
    │  (idea title, hook, angle, storyboard)
    ▼
Output
```

## Storage Schema

```sql
CREATE TABLE events (
    id TEXT PRIMARY KEY,           -- UUID v4
    timestamp TEXT NOT NULL,       -- RFC 3339
    source TEXT NOT NULL,          -- shell, editor, filesystem, git
    event_type TEXT NOT NULL,      -- command, file_save, page_visit, etc.
    event_data TEXT NOT NULL,      -- JSON blob with event-specific data
    project TEXT,                  -- Auto-detected project name
    metadata TEXT                  -- Optional additional context (JSON)
);

CREATE INDEX idx_events_timestamp ON events(timestamp);
CREATE INDEX idx_events_source ON events(source);
CREATE INDEX idx_events_project ON events(project);
```

The `event_data` column stores the full event payload as JSON, using Rust's serde for serialization. This means we can add new event types without schema migrations — the event_type column tells us how to interpret the JSON.

## Network Model

The daemon listens only on `127.0.0.1:9847`. There is no outbound network traffic. All data stays local.

```
┌─────────────────────────────────────────┐
│                 localhost                 │
│                                          │
│  Shell Hook ──POST──► Daemon :9847       │
│  VS Code Ext ─POST──►   │               │
│  siphon-ctl ──GET───►   │               │
│  Browser ────GET────►   │  (dashboard)  │
│                          │               │
│                     SQLite File          │
│                  ~/.siphon/              │
└─────────────────────────────────────────┘
```

## Scaling Considerations

This is a single-user, single-machine tool. The design optimizes for:

**Low resource usage over throughput.** The daemon should be unnoticeable. Target: <10MB memory, <0.1% CPU while idle.

**Write-heavy, read-occasional.** Events arrive frequently but queries happen when the developer wants ideas — maybe a few times per day. SQLite handles this pattern well.

**Eventual consistency is fine.** If a file event is recorded 100ms after it happened, that's totally acceptable. The analysis looks at multi-minute time windows anyway.

**Data retention is bounded.** Events older than 90 days are periodically cleaned up. The database should stay under 100MB for normal usage.
