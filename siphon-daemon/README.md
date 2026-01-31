# Siphon Daemon

Background service for continuous activity capture. Runs on `localhost:9847` and stores events in SQLite.

## Features

- **Event ingestion** via HTTP API from shell hooks and editor extensions
- **SQLite storage** with automatic project detection
- **Control CLI** (`siphon-ctl`) for querying events and stats
- **Sensitive data redaction** - API keys, passwords, and secrets are automatically redacted before storage
- **Multi-shell support** - Hooks for both Zsh and Bash

## Building

```bash
cargo build --release
```

This produces two binaries:
- `siphon-daemon` - The background service
- `siphon-ctl` - Control CLI for interacting with the daemon

## Running

```bash
# Start the daemon
./target/release/siphon-daemon

# Check status
./target/release/siphon-ctl status

# View recent events
./target/release/siphon-ctl events --hours 2

# View statistics
./target/release/siphon-ctl stats

# Get content ideas
./target/release/siphon-ctl ideas --hours 4
```

## Shell Integration

### Zsh

Add to your `~/.zshrc`:

```bash
source /path/to/siphon-daemon/hooks/siphon-hook.zsh
```

### Bash

Add to your `~/.bashrc`:

```bash
source /path/to/siphon-daemon/hooks/siphon-hook.bash
```

### Utility Commands

Both hooks provide these utility functions:

- `siphon-pause` - Temporarily pause tracking
- `siphon-resume` - Resume tracking
- `siphon-status` - Check if daemon is running

## Security

The daemon automatically redacts sensitive information from commands before storage:

- API keys and tokens (e.g., `ANTHROPIC_API_KEY`, `GITHUB_TOKEN`)
- Passwords in command flags (`-p`, `--password`)
- Authorization headers in curl commands
- AWS credentials, JWTs, and other secrets

Password manager commands (e.g., `pass`, `1password`) are skipped entirely and not stored.

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Health check |
| POST | `/events/shell` | Ingest shell command event |
| POST | `/events/editor` | Ingest editor event |
| GET | `/events` | Query events (params: `hours`, `source`, `project`) |
| GET | `/events/recent` | Get events from last 2 hours |
| GET | `/stats` | Get event statistics |

## Event Format

### Shell Event

```json
{
  "command": "kubectl get pods",
  "exit_code": 0,
  "duration_ms": 1234,
  "cwd": "/home/user/project",
  "git_branch": "main"
}
```

### Editor Event

```json
{
  "action": "file_save",
  "file_path": "/home/user/project/src/main.rs",
  "language": "rust",
  "lines_changed": 15
}
```

## Data Storage

Events are stored in `~/.siphon/events.db` (SQLite).

```sql
SELECT * FROM events
WHERE timestamp > datetime('now', '-2 hours')
ORDER BY timestamp DESC;
```
