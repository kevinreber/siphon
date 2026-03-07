# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Siphon** is a developer content generation tool that passively tracks your development activity and surfaces content ideas (YouTube videos, blog posts, newsletters) from what you actually built and learned.

It has three components:

- **siphon-cli** — TypeScript/Node.js CLI for on-demand session analysis
- **siphon-daemon** — Rust background daemon for continuous activity capture (SQLite, Axum REST API)
- **siphon-ui** — Vanilla HTML/CSS/JS web dashboard served by the daemon at `http://localhost:9847`

## Project Structure

```
siphon/
├── siphon-cli/                # TypeScript CLI tool
│   ├── src/
│   │   ├── cli.ts             # CLI entry point
│   │   ├── analyzer.ts        # Pattern detection and clustering
│   │   ├── generator.ts       # Content idea and storyboard generation
│   │   └── collectors/        # Shell, browser, git, file collectors
│   ├── biome.json             # Biome linter/formatter config
│   ├── package.json
│   └── tsconfig.json
│
├── siphon-daemon/             # Rust background daemon
│   ├── src/
│   │   ├── main.rs            # Daemon entry point + static file serving
│   │   ├── ctl.rs             # Control CLI (siphon-ctl)
│   │   ├── api.rs             # HTTP API (Axum)
│   │   ├── storage.rs         # SQLite persistence
│   │   ├── watcher.rs         # File system watcher
│   │   └── triggers.rs        # Activity detection
│   ├── hooks/                 # Shell integration (siphon-hook.zsh)
│   ├── vscode-extension/      # VS Code activity tracker
│   └── Cargo.toml
│
├── siphon-ui/                 # Web dashboard (served by daemon)
│   ├── index.html
│   ├── style.css
│   └── app.js
│
├── Makefile                   # Build and install targets
└── install.sh                 # Full installer script
```

## Development Commands

### Using Make (recommended)

```bash
make build          # Build both CLI and daemon
make build-cli      # Build only the TypeScript CLI
make build-daemon   # Build only the Rust daemon
make check          # Run all linters and type checks
make test           # Run all tests (CLI + daemon + install)
make install        # Full install (build + install + setup service)
make install-ui     # Install web dashboard to ~/.siphon/ui/
make start          # Start daemon service
make stop           # Stop daemon service
make status         # Check if daemon is running
make clean          # Remove build artifacts
make uninstall      # Remove Siphon completely
```

### CLI-specific (from siphon-cli/)

```bash
npm install         # Install dependencies
npm run build       # Compile TypeScript to dist/
npm run typecheck   # TypeScript type checking
npx biome check src # Lint and format check
npx biome format src --write  # Auto-format
```

### Daemon-specific (from siphon-daemon/)

```bash
cargo build --release    # Build release binary
cargo test               # Run tests
cargo clippy             # Lint
cargo fmt                # Format
cargo fmt --check        # Check formatting
```

## Code Conventions

### TypeScript (siphon-cli)

- **Formatter/Linter**: Biome (replaces ESLint + Prettier)
- **Style**: Single quotes, 2-space indent, trailing commas
- Run `npx biome check src` before committing
- Strict TypeScript mode enabled

### Rust (siphon-daemon)

- Follow standard Rust idioms and clippy recommendations
- Use `anyhow` for error handling in application code
- Use `thiserror` for library/public error types
- Format with `cargo fmt` before committing
- Zero clippy warnings (`cargo clippy -- -D warnings`)

### HTML/CSS/JS (siphon-ui)

- No build step — plain vanilla JS
- No frameworks or bundlers
- Served directly by the Rust daemon as static files

## Data Storage

The daemon stores events in SQLite at `~/.siphon/events.db`. The CLI reads shell/browser history directly from the filesystem. No external services — all data is local.

## Testing

```bash
# All tests
make test

# CLI tests only
cd siphon-cli && npm test

# Daemon tests only
cd siphon-daemon && cargo test

# Install script test
./scripts/test-install.sh
./scripts/test-install.sh --quick  # Skip build
```

## Key Files

| File | Purpose |
|------|---------|
| `siphon-cli/src/cli.ts` | CLI commands entry point |
| `siphon-cli/src/analyzer.ts` | Activity pattern detection |
| `siphon-cli/src/generator.ts` | Content idea generation |
| `siphon-daemon/src/main.rs` | Daemon entry + file serving |
| `siphon-daemon/src/api.rs` | HTTP REST API |
| `siphon-daemon/src/storage.rs` | SQLite persistence |
| `siphon-daemon/hooks/siphon-hook.zsh` | Zsh shell hook |

## Environment

- Daemon runs on port `9847` by default
- Data dir: `~/.siphon/`
- Events DB: `~/.siphon/events.db`
- Binaries installed to: `~/.local/bin/`

## Pull Request Workflow

Before pushing, run all checks:

```bash
# CLI checks
cd siphon-cli && npm run typecheck && npx biome check src

# Daemon checks
cd siphon-daemon && cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

### Automated Hooks

Claude Code hooks (`.claude/settings.json`) automatically:
- Run TypeScript typecheck + Biome before committing CLI code
- Run `cargo fmt --check`, `cargo clippy`, and `cargo check` before committing daemon code
- Run `cargo fmt` and `npx biome format --write` after editing source files

## Architecture Notes

See `docs/ARCHITECTURE.md` for full system design details.
See `docs/DECISIONS.md` for key technical choices and rationale.
See `VISION.md` for where this project is headed.
