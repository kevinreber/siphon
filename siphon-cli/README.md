# Siphon CLI

Command-line tool for analyzing developer activity and generating content ideas.

## Features

- **Capture** - Collect and analyze recent activity from shell history and git
- **Status** - Quick overview of what you've been working on
- **Summary** - Human-readable session summaries
- **Struggle Score** - Detect debugging intensity for content potential
- **Aha Moments** - Identify breakthrough moments after failures
- **Claude Integration** - Generate polished content ideas using Claude API

## Installation

```bash
npm install
npm run build
```

## Usage

### Capture and Analyze

```bash
# Analyze last 2 hours of activity
siphon capture --time 2h

# Generate a Claude prompt for enhanced analysis
siphon capture --time 2h --prompt

# Focus on a specific topic
siphon capture --time 4h --topic kubernetes

# Verbose mode with detailed breakdown
siphon capture --time 2h --verbose

# Generate polished content ideas using Claude API
siphon capture --time 2h --generate
```

### Claude API Integration

The `--generate` flag uses Claude to create polished content ideas from your session data:

```bash
# Set your API key
export ANTHROPIC_API_KEY=your-key-here

# Generate content ideas
siphon capture --time 2h --generate
```

This will produce:
- 3-5 enhanced content ideas with titles, hooks, and outlines
- Target audience suggestions
- Key takeaways for each idea
- Weekly theme suggestions (if patterns detected)
- Series potential for ongoing content

### Quick Status

```bash
# See what you've been doing in the last hour
siphon status

# Last 4 hours
siphon status --time 4h
```

### Session Summary

```bash
# Get a formatted summary of your work session
siphon summary --time 4h
```

## Output Example

```
SUMMARY
----------------------------------------
Time Range: 14:00 - 16:30 (150 min)
Total Events: 87
Commands: 72 (12 failed)
Struggle Score: 45%

TOP TOPICS
----------------------------------------
  kubectl: 23 events (45 min)
  docker: 18 events (30 min)
  git: 15 events (20 min)

CONTENT IDEAS
----------------------------------------
1. Debugging Kubernetes: A Developer's Journey
   Hook: "I spent 45 minutes debugging kubernetes. Here's what I learned."
   Format: video

2. The kubernetes Bug That Took Me Hours (And the Simple Fix)
   Hook: "After multiple failed attempts, I finally figured out the solution."
   Format: blog
```

## Data Sources

The CLI reads from:
- Shell history (`~/.zsh_history` or `~/.bash_history`)
- Git log (commits and branch switches)
- Siphon daemon (if running) for real-time events

## How It Works

1. **Collection** - Reads events from shell history, git, and the daemon
2. **Clustering** - Groups events by topic and temporal proximity (30-min windows)
3. **Signal Detection** - Identifies learning patterns (failures, retries, searches)
4. **Scoring** - Calculates struggle score and aha moment intensity
5. **Generation** - Produces content ideas based on detected patterns
