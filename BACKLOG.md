# Feature Backlog

This document tracks planned features, ideas, and improvements for Siphon.

---

## 🚀 In Progress (Phase 1)

### Shell Hook
- [x] Basic zsh hook with preexec/precmd
- [x] Capture: command, exit code, duration, cwd
- [x] Fire-and-forget HTTP POST to daemon
- [ ] Bash support

### Rust Daemon
- [x] Basic Axum HTTP server on 127.0.0.1:9847
- [x] SQLite storage with events table
- [x] POST /events/shell endpoint
- [ ] File system watcher (notify crate)
- [ ] Health check endpoint (GET /health)
- [ ] Event query API (GET /events)

### TypeScript CLI
- [x] Basic CLI structure with commander
- [x] `siphon status` - show recent activity
- [x] `siphon capture --time 2h` - analyze recent work
- [ ] Shell history reader (~/.zsh_history)
- [ ] Git log integration
- [ ] Browser history reader (Chrome/Firefox)

### Web Dashboard
- [x] Dashboard HTML/CSS/JS (no build step)
- [x] Stats overview (total events, session duration, focus score, db size)
- [x] Daily activity bar chart (last 14 days)
- [x] Events by source breakdown
- [x] Session summary with projects, apps, key activities
- [x] Recent events list with source color coding
- [x] Active window display
- [x] Auto-refresh (15 second polling)
- [x] Static file serving from daemon (tower-http ServeDir)
- [x] `make install-ui` target
- [ ] Content ideas panel (display ideas from daemon)
- [ ] Time range picker for events view
- [ ] Project filter/selector

---

## 📋 Planned (Phase 2 - Analysis)

### Struggle Score Detection
Track the ratio of failed commands to successful ones, repeated searches, and time-between-saves. High struggle = high learning = high content potential.

**Implementation:**
- Track exit codes (non-zero = failure)
- Detect repeated similar commands (retry patterns)
- Calculate struggle_score per cluster
- Surface high-struggle sessions as content opportunities

### Aha Moment Detection
When a user has many failed attempts followed by a sudden success, flag this as a prime content opportunity.

**Implementation:**
- Detect sequences: [fail, fail, fail, ..., success]
- Record the "breakthrough" command/action
- Calculate aha_intensity based on failure count
- Include the full journey in content generation

### Git Branch Context Tagging
Parse branch names like `fix/auth-bug` or `feature/payment-integration` for automatic topic tagging.

**Implementation:**
- Extract branch name from git
- Parse common patterns: fix/, feature/, bug/, refactor/
- Use as topic hint in clustering

### Session Summaries
End-of-day summary: "Today you worked on Kubernetes for 2h, debugged TypeScript for 45min."

**Implementation:**
- `siphon summary` command
- Group by detected topics
- Calculate time spent per topic
- Optional: daily digest notification

---

## 💡 Backlog (Phase 3+)

### Content Generation Enhancements

- [ ] **Thread format output** - Generate Twitter/X thread format alongside video storyboards
- [ ] **Before/After code snippets** - Extract starting state and final state of key files
- [ ] **Difficulty estimation** - Tag content as "Beginner-friendly" vs "Advanced"
- [ ] **Content templates library** - Pre-built templates: "Debugging story", "Tool comparison", "TIL", "Deep dive"

### Data Collection Improvements

- [ ] **Screenshot capture on key moments** - Optionally capture screenshots when learning signals are detected
- [ ] **Idle detection** - Distinguish "actively working" vs "left for coffee" using system idle APIs
- [ ] **Incremental git diff analysis** - Capture actual changes, not just commits
- [ ] **Event deduplication** - Debounce duplicate events from multi-terminal setups

### Technical Improvements

- [ ] **tokio-rusqlite** - Non-blocking SQLite for async daemon
- [ ] **Sensitive command filtering** - Auto-redact API_KEY=, password, tokens
- [ ] **"Pause tracking" hotkey** - Global keyboard shortcut for sensitive work
- [ ] **Data retention cleanup** - Auto-delete events older than 90 days

### Integrations

- [ ] **Raycast/Alfred extension** - Quick "What did I work on?" from launcher
- [ ] **Git commit message enhancer** - Enrich commits with debugging journey context
- [ ] **Obsidian daily notes** - Auto-append work summaries to daily notes
- [ ] **"Record now" trigger** - Trigger OBS when high-value content detected
- [ ] **VS Code extension** - Editor activity tracking
- [ ] **Neovim plugin** - Editor events for Neovim users
- [ ] **Browser extension** - Real-time page visit tracking

### Analytics & Reporting

- [ ] **Weekly "content potential" report** - Scheduled summary of best content opportunities
- [ ] **Topic trends over time** - What topics are you working on most?
- [ ] **Productivity insights** - Peak hours, focus sessions, context switches

---

## ❌ Not Planned (Out of Scope)

These are explicitly out of scope per the project vision:

- Cloud sync / accounts
- Telemetry / analytics collection
- Time tracking for billing
- Team/enterprise features
- Mobile apps

---

## Notes

**Adding new ideas:** Add them to the appropriate section with a brief description. Major features should include an "Implementation" subsection.

**Moving items:** When work starts, move from Backlog → In Progress. When complete, check the box.

**Decision records:** Major architectural decisions should be documented in DECISIONS.md, not here.
