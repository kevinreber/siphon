# Getting Started with Siphon

Welcome to Siphon! This guide will help you install, configure, and start using Siphon to turn your daily development work into content ideas.

## What is Siphon?

Siphon is a smart activity tracker that passively captures your development activity—shell commands, file edits, browser searches, git commits—and surfaces content ideas based on what you actually did. It bridges the gap between doing interesting technical work and recognizing it as potential content.

**The core idea:** Turn your unconscious expertise into conscious content.

---

## Quick Install

```bash
# Clone the repository
git clone https://github.com/kevinreber/siphon.git
cd siphon

# Run the installer
./install.sh
```

That's it! The installer will set up both components and configure your shell.

### Installation Options

```bash
./install.sh --cli-only      # CLI only (no background tracking)
./install.sh --no-service    # Skip daemon auto-start
./install.sh --no-hooks      # Skip shell hook configuration
./install.sh --uninstall     # Remove Siphon completely
```

### Requirements

- **Node.js v18+** and npm (for the CLI)
- **Rust** via [rustup.rs](https://rustup.rs/) (for the daemon)

---

## Your First 5 Minutes

### 1. Restart Your Shell

After installation, restart your terminal or run:

```bash
source ~/.zshrc   # or ~/.bashrc for bash users
```

### 2. Check Installation

```bash
# Verify the daemon is running
siphon-ctl status

# You should see something like:
# Siphon daemon is running (PID: 12345)
# Uptime: 2 minutes
# Events collected: 0
```

### 3. Open the Dashboard (Optional)

Open `http://localhost:9847` in your browser to see the web dashboard. It shows real-time session state, event stats, and activity charts. The dashboard auto-refreshes every 15 seconds.

If the dashboard isn't available, install it with:

```bash
make install-ui
```

### 4. Do Some Work

Use your terminal normally for a few minutes:

```bash
# Example: navigate around, run commands
cd ~/projects/my-app
git status
npm run test
```

### 5. Capture Your Activity

```bash
# See what Siphon captured
siphon status

# Get a detailed analysis with content ideas
siphon capture --time 30m
```

---

## Example Workflows

### Example 1: End of Day Content Review

You've been coding all day. Before wrapping up, see what content-worthy moments happened:

```bash
# Analyze the last 4 hours of work
siphon capture --time 4h

# Output might show:
#
# Content Ideas (3 found):
#
# [HIGH] "Debugging Docker Container Network Issues"
#   Evidence: 12 shell commands, 3 Stack Overflow searches
#   Hook: "Why your containers can't talk to each other"
#
# [MEDIUM] "Setting Up TypeScript Path Aliases"
#   Evidence: 5 config file edits, 2 documentation searches
#   Hook: "Stop using ugly relative imports"
```

### Example 2: Quick Status Check

Want a quick overview without the full analysis?

```bash
siphon status --time 2h

# Output:
# Last 2 hours:
#   Shell commands: 47
#   Files modified: 12
#   Git commits: 3
#   Browser searches: 8
#
# Top topics: kubernetes, docker, networking
```

### Example 3: Focus on a Specific Topic

You know you did something interesting with Kubernetes today:

```bash
siphon capture --time 4h --topic kubernetes

# Filters results to only Kubernetes-related activity
```

### Example 4: AI-Enhanced Content Ideas

Get Claude to help generate polished content ideas (requires API key):

```bash
# Set your API key
export ANTHROPIC_API_KEY="sk-ant-..."

# Generate AI-enhanced content
siphon capture --time 2h --generate

# Output includes:
# - Refined titles and hooks
# - Video storyboard suggestions
# - Blog post outlines
```

### Example 5: Verbose Mode for Debugging

See exactly what Siphon captured:

```bash
siphon capture --time 1h --verbose

# Shows detailed breakdown:
# - Individual commands with timestamps
# - Each file change
# - Browser history entries
# - Git operations
```

### Example 6: Using the Web Dashboard

Open the visual dashboard in your browser for a real-time overview:

```bash
# Open the dashboard (daemon must be running)
open http://localhost:9847

# The dashboard shows:
# - Session state (active/idle/offline)
# - Total events, focus score, database size
# - Daily activity chart
# - Events by source breakdown
# - Session summary with projects and apps
# - Recent events feed
```

The dashboard auto-refreshes every 15 seconds. Use the time range selector on the session summary card to adjust the analysis window.

### Example 7: Using the Daemon Control CLI

Query the daemon directly for real-time data:

```bash
# View recent events
siphon-ctl events --hours 1

# Get statistics
siphon-ctl stats

# Get content ideas directly from daemon
siphon-ctl ideas --hours 4
```

---

## What Gets Captured

| Source | What's Tracked | Content Signal |
|--------|----------------|----------------|
| **Shell** | Commands you run | Tools used, problem-solving patterns |
| **Browser** | Searches, docs visited | Research phase, learning moments |
| **Git** | Commits, branches, pushes | Building narrative, milestones |
| **Files** | Code edits, new files | What you worked on |
| **VS Code** | Files opened, edit patterns | Editor activity |
| **Clipboard** | Commands copied (redacted) | Workflow patterns |

### Privacy Notes

- All data stays **local** on your machine (`~/.siphon/events.db`)
- Sensitive data (API keys, passwords) is **automatically redacted**
- No cloud sync, no telemetry, no accounts required
- You own your data—explore it with any SQLite client

---

## Common Commands Reference

### CLI Commands (`siphon`)

| Command | Description |
|---------|-------------|
| `siphon status` | Quick overview of recent activity |
| `siphon capture --time 2h` | Analyze activity and get content ideas |
| `siphon capture --generate` | AI-enhanced content generation |
| `siphon summary --time 4h` | Formatted summary of work session |

### Daemon Control (`siphon-ctl`)

| Command | Description |
|---------|-------------|
| `siphon-ctl status` | Check if daemon is running |
| `siphon-ctl events --hours 2` | View recent events |
| `siphon-ctl stats` | Event statistics |
| `siphon-ctl ideas --hours 4` | Get content ideas from daemon |

### Web Dashboard

| URL/Command | Description |
|-------------|-------------|
| `http://localhost:9847` | Open the web dashboard |
| `make install-ui` | Install dashboard to `~/.siphon/ui/` |

### Shell Utilities

| Command | Description |
|---------|-------------|
| `siphon-pause` | Temporarily pause tracking |
| `siphon-resume` | Resume tracking |
| `siphon-status` | Quick daemon status check |

---

## Troubleshooting

### Daemon not running

```bash
# Check status
siphon-ctl status

# Start manually
make start

# Or use the daemon binary directly
~/.local/bin/siphon-daemon &
```

### No events being captured

1. Make sure your shell is configured:
   ```bash
   # Check if hook is loaded
   type siphon_preexec  # Should show function definition
   ```

2. Restart your shell after installation:
   ```bash
   source ~/.zshrc
   ```

3. Verify daemon is accepting events:
   ```bash
   curl http://127.0.0.1:9847/health
   # Should return: {"status":"ok"}
   ```

### Shell hooks not working

Re-run installation with hooks:
```bash
./install.sh
source ~/.zshrc
```

Or manually add to your shell config:
```bash
# For zsh, add to ~/.zshrc:
source ~/.local/share/siphon/siphon-hook.zsh
```

### Reset everything

```bash
# Uninstall
./install.sh --uninstall

# Remove data
rm -rf ~/.siphon

# Reinstall fresh
./install.sh
```

---

## Next Steps

1. **Use Siphon for a week** - Let it passively collect data as you work normally
2. **Check the dashboard** - Open `http://localhost:9847` to see your activity visualized
3. **Run `siphon capture` daily** - Check in at the end of each work session
4. **Try AI generation** - Set up your Anthropic API key and use `--generate`
5. **Explore your data** - Open `~/.siphon/events.db` in a SQLite browser
6. **Read the architecture** - Check out [ARCHITECTURE.md](./ARCHITECTURE.md) to understand the system

---

## Getting Help

- **Documentation**: See [README.md](./README.md), [ARCHITECTURE.md](./ARCHITECTURE.md)
- **Issues**: [GitHub Issues](https://github.com/kevinreber/siphon/issues)
- **Vision**: Read [VISION.md](./VISION.md) for the project roadmap

Happy content creating!
