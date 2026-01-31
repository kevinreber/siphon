# Vision

## The Insight

Every developer is sitting on a goldmine of content. The debugging sessions, the late-night Stack Overflow rabbit holes, the "aha" moments when something finally clicks — these are exactly the stories that other developers want to hear. But there's a gap between doing the work and recognizing it as content.

The gap exists for three reasons:

**1. Flow state is anti-reflective.** When you're deep in a problem, your brain is fully allocated to solving it. You're not thinking "oh, this would make a great video." By the time you surface, the details have faded.

**2. The curse of knowledge.** Once you understand something, it feels obvious. The struggle that got you there — which is the interesting part — becomes invisible. You think "everyone knows this" when actually, you just learned it an hour ago.

**3. Imposter syndrome scales.** The more skilled you become, the more you take for granted. A senior developer debugging a complex distributed systems issue doesn't realize that thousands of people would watch a video about exactly that problem.

## What We're Building

Siphon is the tool that closes this gap. It watches what you do (passively, privately, locally) and tells you "hey, that thing you just spent 2 hours on? That's a video."

The core value proposition is turning **unconscious expertise into conscious content**.

## Design Principles

### 1. Invisible Until Useful

The tool should be undetectable during work. No notifications, no pop-ups, no "would you like to save this?" prompts. The daemon should use less memory than a browser tab. The shell hook should add zero perceptible latency.

Content ideas surface when the developer asks for them, not when the tool decides to interrupt.

### 2. Your Data, Your Machine

Developer activity data is deeply personal. It reveals what you're working on, what you struggle with, what you search for, and how you spend your time. None of this should ever leave your machine unless you explicitly choose to share it.

No accounts. No cloud sync. No telemetry. The SQLite database is yours. You can query it, export it, or delete it at any time.

### 3. Useful Without AI, Better With AI

The tool must generate usable content ideas with zero external dependencies. Simple pattern matching and template-based generation gets you 70% of the way there. But for the other 30% — creative angles, catchy titles, narrative structure — the tool should make it easy to leverage LLMs by generating rich context prompts.

### 4. Gradual Adoption

Don't ask developers to change their workflow. Day one: install the CLI and run it after a session. Week one: add the shell hook. Month one: install the daemon for continuous capture. Each step adds value without requiring the previous one.

### 5. Developer-Friendly

This is a tool built by a developer, for developers. It should respect the things developers care about: open source, local-first, no vendor lock-in, sensible defaults, good documentation, and a clean CLI interface.

## Content Types We Support

### YouTube Videos

The primary use case. Siphon generates storyboards that map your debugging/building journey into a narrative arc:

- **Hook:** The attention-grabbing opening (usually the problem or the surprising outcome)
- **Setup:** Context that viewers need to follow along
- **Journey:** The debugging/building process, condensed from hours to minutes
- **Resolution:** The fix/solution/outcome
- **Lesson:** The transferable insight

### Blog Posts / Newsletters

Shorter-form content that can be derived from the same activity data:

- "Today I Learned" posts from individual discoveries
- "Weekly Dev Digest" from a week of accumulated activity
- Deep-dive tutorials from extended learning sessions
- Tool comparison posts from trying multiple approaches

### Future: Social/Short-Form

Short-form content from individual moments:

- "The one command that saved me 3 hours"
- Quick tips from patterns detected in your workflow
- Before/after comparisons from refactoring sessions

## Roadmap

### Phase 1: Capture (Current)

Get the data collection right. The TypeScript CLI captures on-demand from existing data sources. The Rust daemon captures continuously with minimal overhead.

- [x] Shell history parser (zsh, bash)
- [x] Browser history reader (Chrome, Firefox)
- [x] Git activity collector
- [x] File modification tracker
- [x] Rust daemon with HTTP API
- [x] Zsh shell hook
- [x] VS Code extension
- [ ] Browser extension for real-time search capture
- [ ] Fish shell hook
- [ ] Neovim plugin

### Phase 2: Analyze

Make the clustering and pattern detection smarter. Reduce noise, increase signal.

- [ ] Improved topic extraction (multi-word topics, context awareness)
- [ ] Session detection (gaps between work sessions)
- [ ] Frustration detection (repeated commands, rapid file switches)
- [ ] Learning curve detection (progression from searches to implementation)
- [ ] Multi-day narrative detection (projects that evolve over time)

### Phase 3: Generate

Richer content output formats and optional AI integration.

- [ ] Claude API integration (opt-in, BYO key) for enhanced ideas
- [ ] Newsletter draft generation
- [ ] Blog post outline generation
- [ ] Thumbnail concept generation
- [ ] SEO keyword suggestion
- [ ] Weekly/monthly digest generation

### Phase 4: Integrate

Connect to the content creation workflow.

- [ ] Export to Notion/Obsidian
- [ ] Companion screen recording triggers
- [ ] Integration with video editing tools (chapter markers)
- [ ] RSS feed of ideas
- [ ] Scheduled digest emails

## Who This Is For

**Developers who want to create content but don't know where to start.** You have the knowledge. You have the experiences. You just need something to tap you on the shoulder and say "that thing you just did? Write about it."

**Developers who want to document their learning journey.** Even if you never publish anything, having a log of what you learned and how you learned it is valuable. Siphon is a smarter, automated version of a developer journal.

**Developer advocates and educators.** People whose job involves creating developer content can use this to find real-world examples and scenarios from their actual work, rather than contriving artificial examples.

## What This Is Not

**Not a time tracker.** We don't care how many hours you worked or which apps you used. We care about what you learned and built.

**Not a productivity tool.** We're not here to optimize your workflow or tell you you're spending too much time on Reddit. We're here to find the stories in your work.

**Not a surveillance tool.** This runs on your machine, for you. There's no management dashboard, no team analytics, no "employee monitoring" use case. If anyone asks for that, the answer is no.
