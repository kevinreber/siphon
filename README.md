# Siphon

> Turn your daily development work into video, newsletter, and blog content — automatically.

You just spent 3 hours debugging a Kubernetes networking issue. You searched Stack Overflow, tried 15 different `kubectl` commands, read the docs twice, and finally fixed it. That debugging journey would make an excellent 10-minute YouTube video — but by the time you're done, the last thing on your mind is content creation.

**Siphon** solves this by passively tracking your development activity and surfacing content ideas based on what you actually did. It detects learning sessions, troubleshooting patterns, and deep-dive moments, then generates video storyboards, blog outlines, and newsletter hooks from your real work.

## How It Works

```
┌─────────────────────────────────────────────────────────────────────┐
│  Your normal workflow (no changes needed)                           │
│                                                                     │
│  Shell commands → Browser searches → File edits → Git commits       │
└──────────────────────────┬──────────────────────────────────────────┘
                           │
                    ┌──────▼──────┐
                    │   Capture   │  Siphon Daemon (Rust) runs in
                    │   Layer     │  the background, or CLI captures
                    └──────┬──────┘  on-demand after a session
                           │
                    ┌──────▼──────┐
                    │   Analyze   │  Clusters events by topic, detects
                    │   Layer     │  learning signals, finds patterns
                    └──────┬──────┘
                           │
                    ┌──────▼──────┐
                    │   Generate  │  Video storyboards, blog outlines,
                    │   Layer     │  newsletter hooks, Claude prompts
                    └─────────────┘
```

## The Problem

Developers learn valuable things every day but rarely turn that knowledge into content because:

1. **You don't notice it in the moment.** When you're deep in a problem, the last thing you're thinking about is "this would make a great video."
2. **You forget the journey.** After solving something, you remember the answer but not the struggle that got you there — and the struggle is what makes good content.
3. **Imposter syndrome wins.** "Who would care about this?" turns out to be wrong most of the time. The knowledge you take for granted is gold to someone else.

## Components

This project has two main components that work together:

### 📋 CLI Tool (`siphon-cli/`)

On-demand analysis. Run it after a work session to capture and analyze your activity.

```bash
cd siphon-cli
npm install && npm run build

# Capture last 2 hours and get content ideas
siphon capture --time 2h

# Generate a video storyboard
siphon script --time 2h --topic kubernetes

# Generate a prompt for Claude to enhance ideas
siphon capture --time 2h --prompt
```

**Tech:** TypeScript, Node.js

### 🦀 Siphon Daemon (`siphon-daemon/`)

Background service that tracks continuously. Low resource usage, always-on capture.

```bash
cd siphon-daemon
cargo build --release

# Start the daemon
siphon-daemon start --foreground

# Check what you've been doing
siphon-ctl ideas --hours 4
```

**Tech:** Rust, SQLite, Axum

### 🔗 Integrations

| Integration | Location | Purpose |
|-------------|----------|---------|
| Zsh Hook | `siphon-daemon/hooks/siphon-hook.zsh` | Real-time shell command capture |
| VS Code Extension | `siphon-daemon/vscode-extension/` | Editor activity tracking |
| Browser Extension | _Planned_ | Search and browsing capture |

## What Gets Captured

| Source | Examples | Why It Matters |
|--------|----------|---------------|
| **Shell** | `kubectl get pods`, `docker build`, `npm install` | Shows what tools you used and how |
| **Browser** | Stack Overflow, docs, GitHub browsing | Reveals research and learning phases |
| **Git** | Commits, branch switches, push/pull | Maps the arc of building something |
| **Files** | Code edits, new files, deletions | Shows what you were working on |

## What Gets Generated

**Content Ideas** with confidence scores:
```
🔥 "Kubernetes Networking Mistakes I Made (So You Don't Have To)" [high]
   You spent 45 minutes debugging, searched 8 times, visited 3 docs pages.

✨ "Docker Multi-Stage Builds: The Setup Nobody Shows You" [medium]
   You modified 4 Dockerfiles and ran 12 build commands.
```

**Video Storyboards** with hooks, sections, and talking points:
```
HOOK: "I spent 3 hours on a problem that turned out to be one line..."
SECTION 1: The Setup (show the error)
SECTION 2: What I Tried (the debugging journey)
SECTION 3: The Fix (the actual solution)
SECTION 4: Why This Happens (the lesson)
```

## Privacy

All data stays local. Nothing leaves your machine. The daemon stores events in a local SQLite database at `~/.siphon/events.db`. The CLI reads shell and browser history directly from your filesystem. No cloud, no telemetry, no accounts.

## Project Structure

```
siphon/
├── siphon-cli/                # TypeScript CLI tool
│   ├── src/
│   │   ├── cli.ts             # CLI entry point
│   │   ├── analyzer.ts        # Pattern detection and clustering
│   │   ├── generator.ts       # Content idea and storyboard generation
│   │   └── collectors/        # Shell, browser, git, file collectors
│   ├── package.json
│   └── README.md
│
├── siphon-daemon/             # Rust background daemon
│   ├── src/
│   │   ├── main.rs            # Daemon entry point
│   │   ├── ctl.rs             # Control CLI (siphon-ctl)
│   │   ├── api.rs             # HTTP API server
│   │   ├── storage.rs         # SQLite persistence
│   │   └── collectors/        # File system watcher, shell collector
│   ├── hooks/                 # Shell integration hooks
│   ├── vscode-extension/      # VS Code activity tracker
│   ├── Cargo.toml
│   └── README.md
│
└── docs/
    ├── ARCHITECTURE.md        # System design deep dive
    ├── DECISIONS.md           # Why we made key technical choices
    └── VISION.md              # Where this project is headed
```

## Quick Install

Install everything (CLI, daemon, database, shell hooks) with a single command:

```bash
./install.sh
```

That's it! The installer will:
- Build the CLI and daemon
- Install binaries to `~/.local/bin`
- Set up the database at `~/.siphon/events.db`
- Configure the daemon to start automatically (systemd/launchd)
- Add shell hooks for real-time command tracking

### Installation Options

```bash
# Full installation (recommended)
./install.sh

# CLI only (no background tracking)
./install.sh --cli-only

# Skip daemon auto-start service
./install.sh --no-service

# Skip shell hook configuration
./install.sh --no-hooks

# Custom install location
./install.sh --prefix /usr/local/bin

# Uninstall completely
./install.sh --uninstall
```

### Using Make

```bash
make install          # Full installation
make build            # Build without installing
make start            # Start the daemon
make stop             # Stop the daemon
make status           # Check if daemon is running
make uninstall        # Remove Siphon
```

### Manual Installation

See the individual READMEs for detailed manual setup:

- [CLI Tool Setup](./siphon-cli/README.md)
- [Daemon Setup](./siphon-daemon/README.md)

### Requirements

- **Node.js** (v18+) and **npm** for the CLI
- **Rust** (via [rustup](https://rustup.rs/)) for the daemon

### After Installation

```bash
# Restart your terminal (or source your shell config)
source ~/.zshrc  # or ~/.bashrc

# Verify everything is working
siphon-ctl status     # Should show "Daemon is running"
siphon status         # Quick overview of recent activity

# After working for a while...
siphon capture        # Analyze your session and get content ideas
```

## Contributing

This is an early-stage project. If you're interested in contributing, start by reading [ARCHITECTURE.md](./docs/ARCHITECTURE.md) and [DECISIONS.md](./docs/DECISIONS.md) to understand the design philosophy.

## License

MIT
