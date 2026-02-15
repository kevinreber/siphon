# UI Options for Siphon

This document evaluates three approaches for adding a visual interface to Siphon, aimed at making activity tracking and content ideas more accessible to non-technical users.

---

## Option 1: Web UI served by the existing daemon (Selected)

Serve a single-page app directly from the Rust daemon on `localhost:9847`. Users open a browser tab to see their dashboard.

**Why this fits Siphon:**
- The daemon already runs Axum (a web framework) with HTTP endpoints for events, stats, summaries, and session info. Adding static file serving is a small change.
- No new process or install step. The UI comes for free when the daemon is running.
- Non-technical users know how to open a browser tab.
- Keeps the "zero cloud, local-only" promise. Everything stays on localhost.
- Can be built incrementally — start with a simple dashboard, add complexity over time.
- The API already has CORS enabled, so the frontend can call endpoints directly.

**Tech choices:**
- Plain HTML + CSS + vanilla JavaScript (no build step, no npm dependencies)
- Served as static files from the daemon binary or from `~/.siphon/ui/`
- Calls existing API endpoints (`/stats`, `/events`, `/summary`, `/session`, `/storage`)

**Tradeoffs:**
- Requires a browser to be open (but so does most of a developer's day)
- No native OS integration (system tray, notifications)
- UI capabilities limited to what browsers support

---

## Option 2: Terminal UI (TUI)

A Rust-based TUI using the `ratatui` crate. Runs directly in the terminal with interactive panels, charts, and navigation.

**Why it could work:**
- No browser needed. Stays entirely in the terminal workflow.
- Very lightweight — adds zero external dependencies beyond what Siphon already uses.
- Natural fit for the developer audience. Tools like `htop`, `lazygit`, and `k9s` prove TUIs can be great.
- Rust ecosystem has strong TUI libraries (`ratatui`, `crossterm`).

**Why we didn't choose it (for now):**
- The stated goal was "non-technical users." TUIs are still intimidating for people who don't live in the terminal.
- Limited visualization capabilities — no rich charts, no clickable links, no images.
- Harder to iterate on design. CSS changes are instant; TUI layout changes require recompilation.
- Can't easily share or screenshot a TUI dashboard.

**When to reconsider:**
- If the primary audience shifts back to power users / developers
- As a complementary interface alongside the web UI (e.g., `siphon-ctl dashboard`)

---

## Option 3: Tauri desktop app

A standalone desktop application using Tauri (Rust backend + system webview). Similar to Electron but uses the OS-native webview instead of bundling Chromium.

**Why it could work:**
- Native app feel: system tray icon, OS notifications, global shortcuts.
- Tiny binary (~5-10MB) compared to Electron (~200MB+). Uses the system's built-in WebKit/WebView2.
- Siphon is already a Rust project, so Tauri's Rust backend is a natural fit.
- Could bundle the daemon directly into the app — one process instead of two.

**Why we didn't choose it (for now):**
- Adds significant build complexity (Tauri CLI, platform-specific signing, installer generation).
- Another artifact to distribute — binary downloads or a package manager entry.
- The daemon already provides everything the UI needs via HTTP. A desktop wrapper is an extra layer with minimal added value at this stage.
- WebView rendering varies across OS versions, which can cause subtle UI inconsistencies.

**When to reconsider:**
- If Siphon needs native OS features (system tray always-on indicator, native notifications for content ideas)
- If the project moves toward a more polished "product" distribution model
- If users want a one-click install with no terminal interaction

---

## Decision

**We chose Option 1** for the initial implementation. It adds the most value with the least complexity, reuses existing infrastructure, and aligns with Siphon's philosophy of being lightweight and local-first.

The web UI is served directly by the daemon — no extra processes, no build tools, no additional dependencies. If we later need native desktop features, Option 3 (Tauri) is a natural evolution since the web frontend would carry over directly.
