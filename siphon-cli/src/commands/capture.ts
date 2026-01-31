/**
 * Capture Command
 *
 * Captures developer activity from various sources and
 * generates content ideas.
 */

import { Analyzer } from "../analyzer.js";
import { ShellHistoryCollector } from "../collectors/shell.js";
import { GitCollector } from "../collectors/git.js";
import { Event } from "../types.js";

interface CaptureOptions {
  time: string;
  prompt?: boolean;
  topic?: string;
  verbose?: boolean;
}

/**
 * Parse time duration string (e.g., "2h", "30m") to milliseconds
 */
function parseTimeDuration(duration: string): number {
  const match = duration.match(/^(\d+)(h|m|s)?$/);
  if (!match) {
    throw new Error(`Invalid duration: ${duration}. Use format like "2h", "30m", or "90"`);
  }

  const value = parseInt(match[1], 10);
  const unit = match[2] || "m";

  switch (unit) {
    case "h":
      return value * 60 * 60 * 1000;
    case "m":
      return value * 60 * 1000;
    case "s":
      return value * 1000;
    default:
      return value * 60 * 1000;
  }
}

export async function captureCommand(options: CaptureOptions): Promise<void> {
  const durationMs = parseTimeDuration(options.time);
  const endTime = new Date();
  const startTime = new Date(endTime.getTime() - durationMs);

  console.log(`\nCapturing activity from the last ${options.time}...\n`);

  // Collect events from all sources
  const events: Event[] = [];

  // Collect shell history
  const shellCollector = new ShellHistoryCollector();
  try {
    const shellEvents = await shellCollector.collect(startTime, endTime);
    events.push(...shellEvents);
    if (options.verbose) {
      console.log(`  Shell: ${shellEvents.length} commands`);
    }
  } catch (err) {
    console.log(`  Shell: Could not read history`);
  }

  // Collect git activity
  const gitCollector = new GitCollector();
  try {
    const gitEvents = await gitCollector.collect(startTime, endTime);
    events.push(...gitEvents);
    if (options.verbose) {
      console.log(`  Git: ${gitEvents.length} commits`);
    }
  } catch (err) {
    console.log(`  Git: Could not read git log`);
  }

  if (events.length === 0) {
    console.log("No events found in the specified time range.");
    console.log("\nTips:");
    console.log("  - Make sure you have shell history enabled");
    console.log("  - Run the Siphon daemon for real-time capture");
    console.log("  - Try a longer time window with --time 4h");
    return;
  }

  console.log(`Found ${events.length} events\n`);

  // Analyze events
  const analyzer = new Analyzer();
  const result = analyzer.analyze(events, { start: startTime, end: endTime });

  // Display results
  displayResults(result, options);

  // Generate Claude prompt if requested
  if (options.prompt) {
    console.log("\n" + "=".repeat(60));
    console.log("CLAUDE PROMPT");
    console.log("=".repeat(60) + "\n");
    console.log(generateClaudePrompt(result));
  }
}

function displayResults(
  result: ReturnType<Analyzer["analyze"]>,
  options: CaptureOptions
): void {
  const { summary, clusters, ideas } = result;

  // Summary section
  console.log("SUMMARY");
  console.log("-".repeat(40));
  console.log(`Time Range: ${formatTimeRange(result.timeRange)}`);
  console.log(`Total Events: ${summary.totalEvents}`);
  console.log(`Commands: ${summary.totalCommands} (${summary.failedCommands} failed)`);
  console.log(`Struggle Score: ${summary.struggleScore}%`);
  console.log();

  // Top topics
  if (summary.topTopics.length > 0) {
    console.log("TOP TOPICS");
    console.log("-".repeat(40));
    for (const topic of summary.topTopics) {
      console.log(`  ${topic.topic}: ${topic.count} events (${topic.timeMinutes} min)`);
    }
    console.log();
  }

  // Aha moments
  if (summary.ahaMonments.length > 0) {
    console.log("AHA MOMENTS");
    console.log("-".repeat(40));
    for (const aha of summary.ahaMonments) {
      console.log(`  ${aha.description} at ${formatTime(aha.timestamp)}`);
    }
    console.log();
  }

  // Content ideas
  if (ideas.length > 0) {
    console.log("CONTENT IDEAS");
    console.log("-".repeat(40));
    for (let i = 0; i < Math.min(ideas.length, 5); i++) {
      const idea = ideas[i];
      const badge = idea.confidence === "high" ? "" : idea.confidence === "medium" ? "" : "";
      console.log(`\n${i + 1}. ${badge} ${idea.title}`);
      console.log(`   Hook: "${idea.hook}"`);
      console.log(`   Format: ${idea.suggestedFormat}`);
      if (options.verbose) {
        console.log(`   Evidence: ${idea.evidence.join("; ")}`);
      }
    }
    console.log();
  } else {
    console.log("No strong content ideas detected.");
    console.log("Try working on something challenging and run capture again!");
  }

  // Clusters (verbose mode)
  if (options.verbose && clusters.length > 0) {
    console.log("\nCLUSTERS (Detailed)");
    console.log("-".repeat(40));
    for (const cluster of clusters) {
      console.log(`\n[${cluster.topic}] ${cluster.durationMinutes} min, ${cluster.events.length} events`);
      console.log(`  Confidence: ${cluster.confidence}`);
      console.log(`  Struggle: ${cluster.struggleScore}% | Aha: ${cluster.ahaIndex}%`);
      if (cluster.signals.length > 0) {
        console.log(`  Signals: ${cluster.signals.map((s) => s.type).join(", ")}`);
      }
    }
  }
}

function formatTimeRange(range: { start: Date; end: Date; durationMinutes: number }): string {
  return `${formatTime(range.start)} - ${formatTime(range.end)} (${range.durationMinutes} min)`;
}

function formatTime(date: Date): string {
  return date.toLocaleTimeString("en-US", {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
}

function generateClaudePrompt(result: ReturnType<Analyzer["analyze"]>): string {
  const { summary, clusters, ideas } = result;

  return `I just completed a ${result.timeRange.durationMinutes}-minute development session. Here's what I worked on:

## Session Summary
- Total commands: ${summary.totalCommands} (${summary.failedCommands} failed)
- Struggle score: ${summary.struggleScore}%
- Top topics: ${summary.topTopics.map((t) => `${t.topic} (${t.count} events)`).join(", ")}

## Detected Clusters
${clusters
  .map(
    (c) => `- **${c.topic}** (${c.durationMinutes} min): ${c.events.length} events, struggle: ${c.struggleScore}%, aha: ${c.ahaIndex}%`
  )
  .join("\n")}

## Initial Content Ideas
${ideas.map((i, idx) => `${idx + 1}. ${i.title}\n   - Hook: "${i.hook}"\n   - Format: ${i.suggestedFormat}`).join("\n")}

Based on this session data, please:
1. Suggest 3-5 content ideas (video, blog post, or Twitter thread)
2. For each idea, provide a hook, outline, and target audience
3. Identify the most compelling narrative from my session
4. Suggest ways to make the content more engaging

Focus on ideas that would resonate with other developers who face similar challenges.`;
}
