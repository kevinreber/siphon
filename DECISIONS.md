# Technical Decisions

This document records the key technical decisions made during the development of Siphon, along with the reasoning and tradeoffs behind each one. Written as a reference for contributors and for our future selves.

---

## 001: Two-tool architecture (CLI + Daemon)

**Date:** January 2026

**Context:** We started with an on-demand CLI tool that reads shell history, browser history, and git logs after a work session. It works, but it only captures what's already recorded by other tools. Shell history doesn't track duration or exit codes. Browser history might be cleared. There's no way to capture VS Code activity or real-time file changes.

**Decision:** Build a separate background daemon for continuous capture while keeping the CLI for quick on-demand analysis.

**Rationale:**
- The CLI has zero setup cost — run it after a session, get ideas. Valuable on day one.
- The daemon captures richer data (timing, exit codes, editor events) but requires installation.
- Users can start with just the CLI and add the daemon later.
- The daemon's API can also feed the CLI's analyzer for the best of both worlds.

**Tradeoff:** Two codebases to maintain. But the concerns are genuinely different — the CLI is about reading existing data sources, while the daemon is about real-time event collection.

---

## 002: Rust for the daemon, TypeScript for the CLI

**Date:** January 2026

**Context:** The CLI was already written in TypeScript. We needed to choose a language for the background daemon that would run 24/7 on developer machines.

**Decision:** Rust for the daemon, keep TypeScript for the CLI and VS Code extension.

**Rationale for Rust:**
- **Memory:** Rust's ownership model means no garbage collector. The daemon should use <10MB of memory. A Node.js daemon would use 50-100MB minimum just for the runtime.
- **CPU:** No GC pauses means predictable, low CPU usage. A background tool that randomly spikes CPU would get uninstalled fast.
- **Single binary:** `cargo build --release` produces one binary with no runtime dependencies. No "make sure you have Node 18 installed" for a background service.
- **File system hooks:** The `notify` crate provides native OS-level file watching (kqueue on macOS, inotify on Linux). No polling.
- **System integration:** Rust is natural for writing launchd/systemd services, PID file management, signal handling.

**Rationale for keeping TypeScript:**
- The CLI reads data and generates text — TypeScript is fine for this. No performance concern.
- VS Code extensions must be TypeScript/JavaScript. No choice there.
- LLM prompt generation and natural language output is easier in a high-level language.
- The TypeScript ecosystem has good libraries for parsing browser SQLite databases and git output.

**Tradeoff:** Two languages in the repo. But each is used where it genuinely fits best, not just for variety.

---

## 003: SQLite for event storage

**Date:** January 2026

**Context:** The daemon needs to persist events across restarts. Options considered: SQLite, flat JSON files, embedded key-value stores (sled, RocksDB), or no persistence (in-memory only).

**Decision:** SQLite via `rusqlite` with the `bundled` feature flag.

**Rationale:**
- **Write pattern fits perfectly.** Events are appended sequentially with occasional time-range queries. This is exactly what SQLite is optimized for.
- **Zero deployment complexity.** The `bundled` feature compiles SQLite directly into the binary. No external database to install or manage.
- **Queryable.** Users can open `~/.siphon/events.db` with any SQLite client to explore their data directly. This supports the "your data is yours" philosophy.
- **Proven at this scale.** We're talking about thousands of events per day, not millions. SQLite handles this without breaking a sweat.
- **Indexes are sufficient.** Timestamp, source, and project indexes cover all our query patterns.

**Why not flat files:** Hard to query by time range without reading everything. JSON append files work for write-only logs but are slow for filtered reads.

**Why not sled/RocksDB:** Overkill. These are designed for high-throughput scenarios. We'd add complexity for no benefit.

**Why not in-memory only:** Events should survive daemon restarts. The whole point is to accumulate data over days and weeks.

---

## 004: HTTP API over Unix sockets

**Date:** January 2026

**Context:** The shell hook and VS Code extension need to send events to the daemon. Options: Unix domain socket, TCP socket with custom protocol, or HTTP API.

**Decision:** HTTP API on `127.0.0.1:9847` using Axum.

**Rationale:**
- **Shell hook simplicity.** The zsh hook can use `curl`, which is available everywhere. A Unix socket would require `socat` or a custom client. An HTTP POST with JSON is the simplest possible integration.
- **VS Code extension.** The extension can use `fetch()` directly. No need for socket libraries.
- **Debuggability.** You can `curl http://localhost:9847/stats` to check the daemon. You can use Postman or any HTTP tool. Unix sockets are harder to inspect.
- **Language agnostic.** Anyone can write a new integration (Neovim plugin, JetBrains plugin, browser extension) with a simple HTTP POST. No protocol to learn.
- **Axum is lightweight.** The server adds minimal overhead. It's async and built on hyper/tokio, which are battle-tested.

**Why port 9847:** Picked to be unlikely to conflict with common services. Memorable as "9-8-4-7" (no specific meaning, just uncommon).

**Security:** Binding to `127.0.0.1` means only local processes can connect. No authentication is needed because there's no network exposure.

---

## 005: Fire-and-forget event ingestion from shell hooks

**Date:** January 2026

**Context:** The shell hook runs on every command. It cannot add perceptible latency to the terminal experience.

**Decision:** Send events using `curl` in a background subshell (`&!` in zsh) with no output or error handling.

**Rationale:**
- **Zero latency impact.** The `&!` operator disowns the background process immediately. The shell prompt returns before curl even starts.
- **Graceful degradation.** If the daemon isn't running, curl fails silently. The developer sees nothing. No "connection refused" errors cluttering the terminal.
- **Acceptable data loss.** If a few events are lost because the daemon was briefly down, that's fine. We're analyzing patterns over hours, not tracking individual events with audit-level precision.

**Tradeoff:** We can't guarantee delivery. But the alternative (synchronous calls, retry queues) would add latency and complexity that contradicts the "invisible" goal.

---

## 006: Event type schema as tagged JSON

**Date:** January 2026

**Context:** Different event sources produce different data. A shell command has `command`, `exit_code`, `duration_ms`. A file save has `path`. A browser visit has `url` and `title`. How do we store this?

**Decision:** Store event type and event data separately. The `event_type` column is a string tag (e.g., "command", "file_save"), and `event_data` is a JSON blob with the source-specific fields.

**Rationale:**
- **Extensible.** Adding a new event type doesn't require a schema migration. Just start storing events with a new tag and JSON shape.
- **Queryable enough.** We can filter by `event_type` in SQL, then deserialize the JSON in application code. We don't need to query inside the JSON blob.
- **Serde compatibility.** Rust's `serde` with tagged enum serialization maps directly to this pattern. The `EventType` enum serializes to exactly the right JSON shape.

**Tradeoff:** Can't do SQL queries like "find all commands containing 'docker'." But we query by time range and source, not by content. Content analysis happens in application code after loading events.

---

## 007: Keyword-based topic clustering (not ML)

**Date:** January 2026

**Context:** Events need to be grouped by topic for content idea generation. Options: ML-based clustering (embeddings + cosine similarity), keyword extraction, or manual tagging.

**Decision:** Simple keyword-based topic extraction with rule-based categorization.

**Rationale:**
- **No model dependencies.** An ML approach would require embedding models, adding hundreds of MB to the binary or requiring an API call. This contradicts the "local and lightweight" goal.
- **Good enough for the use case.** If someone runs 15 `kubectl` commands in 30 minutes, we don't need ML to figure out they were working on Kubernetes. The first word of a command, file extensions, and URL patterns cover 80% of cases.
- **Transparent.** Users can understand why something was categorized a certain way. "You ran 15 kubectl commands" is more useful than "cluster similarity score: 0.87."
- **LLM enhancement as opt-in.** The `--prompt` flag in the CLI generates a Claude prompt with the raw event data. Users who want smarter analysis can paste it into Claude. This keeps the tool simple while enabling advanced analysis for those who want it.

**Future consideration:** If we add an LLM integration, it would be as an optional enhancement layer, not a core dependency. The tool must work fully offline.

---

## 008: Local-only, no cloud, no accounts

**Date:** January 2026

**Context:** Developer activity data is sensitive. Shell history contains paths, server names, API calls. Browser history reveals what problems you're solving. This data should not leave the developer's machine.

**Decision:** All data stays local. No cloud sync, no accounts, no telemetry.

**Rationale:**
- **Trust.** Developers won't install a background daemon that sends their activity to a server. The bar for trust with an always-on tool is extremely high.
- **Simplicity.** No auth flows, no API keys, no rate limits, no "server is down" errors.
- **Speed.** Everything is a local file read or localhost HTTP call. Queries are instantaneous.

**Tradeoff:** No cross-device sync. No collaborative features. These could be added later as opt-in features, but the core tool must work fully offline. (Note: a local-only web dashboard was later added — see Decision 011. It runs on localhost and doesn't change the local-only commitment.)

---

## 009: Monorepo structure

**Date:** January 2026

**Context:** The project has multiple components (Rust daemon, TypeScript CLI, VS Code extension, shell hook). Should these be separate repos or a monorepo?

**Decision:** Single repository with directory-based separation.

**Rationale:**
- **Atomic changes.** A new event type requires changes in the daemon (Rust), the CLI (TypeScript), and possibly the VS Code extension. In a monorepo, this is one PR.
- **Shared documentation.** Architecture docs, decision records, and the vision doc apply to the whole system.
- **Discovery.** Someone finding the project on GitHub sees everything in one place.
- **No cross-repo versioning complexity.** We don't need to coordinate releases across repos.

**Tradeoff:** Different language toolchains in one repo (Cargo + npm). CI/CD is slightly more complex. But for a project this size, the benefits clearly outweigh the costs.

---

## 010: Content generation stays simple, delegates to LLMs

**Date:** January 2026

**Context:** The content idea generation could be very sophisticated (analyzing narrative arcs, estimating audience interest, generating SEO titles) or very simple (templated ideas based on event patterns).

**Decision:** Ship simple template-based generation with a `--prompt` mode that generates context for Claude/GPT to do the creative work.

**Rationale:**
- **The hard part is capture, not generation.** Getting the raw data organized and surfaced is 90% of the value. A developer looking at "you spent 45 minutes on Kubernetes networking, searched 8 times, visited 3 docs pages" can generate their own ideas.
- **LLMs are better at creative generation.** A template that says "Working with {topic} - A Deep Dive" is okay. Claude analyzing the full event context and generating "The One Kubernetes Setting That Broke My Entire Cluster" is much better.
- **No API key requirement.** The tool works fully without any AI service. The `--prompt` flag is an enhancement for users who want it.

**Future consideration:** An optional Claude API integration could be added behind a config flag, but it should never be required.

---

## 011: Web dashboard served by the daemon

**Date:** February 2026

**Context:** The project was CLI-only, which works well for developers but creates a barrier for non-technical users (content creators, advocates) who want to see what's being tracked. Decision 008 originally noted "no web dashboard" as a tradeoff. Three approaches were evaluated: embedded web UI, terminal UI (TUI), and a Tauri desktop app. See `docs/UI_OPTIONS.md` for the full evaluation.

**Decision:** Serve a lightweight web dashboard directly from the existing Axum daemon as static files. Plain HTML + CSS + vanilla JavaScript, no build step, no framework.

**Rationale:**
- **Zero new dependencies for users.** The daemon already runs Axum. Adding `tower-http`'s `ServeDir` is one line in `Cargo.toml`. Users just open a browser tab.
- **No new process.** The dashboard is served on the same `localhost:9847` port. No extra binary to install, no new service to manage.
- **Stays local-only.** The dashboard runs on localhost, reads from the same API endpoints that `siphon-ctl` uses. No data leaves the machine. This doesn't violate the spirit of Decision 008.
- **No build step.** Plain HTML/CSS/JS means anyone can edit the dashboard without needing Node.js, webpack, or any toolchain. The files are served as-is.
- **Incrementally adoptable.** The dashboard is optional. If `~/.siphon/ui/` doesn't exist, the daemon works exactly as before. `make install-ui` copies the files.

**Why not a TUI:** The stated goal was accessibility for non-technical users. TUIs are still intimidating for people outside the terminal.

**Why not Tauri:** Adds significant build complexity (platform-specific signing, installer generation) and another artifact to distribute. The daemon already provides everything the UI needs via HTTP. If we later need native OS features (system tray, notifications), Tauri is the natural evolution since the web frontend carries over directly.

**Tradeoff:** Requires a browser to be open. No native OS integration (system tray, notifications). But these are acceptable for an initial version, and the web UI foundation works with Tauri if we upgrade later.
