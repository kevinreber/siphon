/**
 * Weekly Digest Command
 *
 * Generates a summary of the week's development activity
 * including top content opportunities and patterns.
 */

import dayjs from "dayjs";
import { Analyzer } from "../analyzer.js";
import { ShellHistoryCollector } from "../collectors/shell.js";
import { GitCollector } from "../collectors/git.js";
import { Event } from "../types.js";
import {
  isClaudeAvailable,
  generateWithClaude,
  displayGeneratedContent,
} from "../generate.js";

export interface DigestOptions {
  days?: string;
  generate?: boolean;
  verbose?: boolean;
}

export async function digestCommand(options: DigestOptions): Promise<void> {
  const days = parseInt(options.days || "7", 10);
  const endTime = new Date();
  const startTime = dayjs().subtract(days, "day").toDate();

  console.log(`\nGenerating ${days}-day digest...`);
  console.log(`Period: ${dayjs(startTime).format("YYYY-MM-DD")} to ${dayjs(endTime).format("YYYY-MM-DD")}\n`);

  // Collect events
  const shellCollector = new ShellHistoryCollector();
  const gitCollector = new GitCollector();

  const shellEvents = await shellCollector.collect(startTime, endTime);
  const gitEvents = await gitCollector.collect(startTime, endTime);

  const allEvents: Event[] = [...shellEvents, ...gitEvents];

  if (allEvents.length === 0) {
    console.log("No activity found in this period.");
    return;
  }

  // Analyze
  const analyzer = new Analyzer();
  const result = analyzer.analyze(allEvents, { start: startTime, end: endTime });

  // Display digest
  displayDigest(result, days, options.verbose);

  // Generate with Claude if requested
  if (options.generate) {
    if (!isClaudeAvailable()) {
      console.log("\n" + "=".repeat(60));
      console.log("CLAUDE API NOT CONFIGURED");
      console.log("=".repeat(60));
      console.log("\nTo use --generate, set your API key:");
      console.log("  export ANTHROPIC_API_KEY=your-key-here\n");
      return;
    }

    try {
      const generatedContent = await generateWithClaude(result);
      console.log();
      displayGeneratedContent(generatedContent);
    } catch (err) {
      console.error("\nFailed to generate content with Claude:");
      console.error(err instanceof Error ? err.message : String(err));
    }
  }
}

function displayDigest(
  result: ReturnType<Analyzer["analyze"]>,
  days: number,
  verbose?: boolean
): void {
  const { summary, sessions, ideas, clusters } = result;

  // Header
  console.log("=".repeat(60));
  console.log(`${days}-DAY DEVELOPMENT DIGEST`);
  console.log("=".repeat(60));
  console.log();

  // Overview stats
  console.log("OVERVIEW");
  console.log("-".repeat(40));
  console.log(`  Total Events: ${summary.totalEvents}`);
  console.log(`  Commands Run: ${summary.totalCommands} (${summary.failedCommands} failed)`);
  console.log(`  Work Sessions: ${summary.sessionCount}`);
  console.log(`  Avg Session Length: ${summary.averageSessionMinutes} min`);
  console.log(`  Overall Struggle Score: ${summary.struggleScore}%`);
  console.log();

  // Daily breakdown
  console.log("DAILY ACTIVITY");
  console.log("-".repeat(40));
  const dailyEvents = getDailyBreakdown(result.events);
  for (const [date, count] of dailyEvents) {
    const bar = "█".repeat(Math.min(Math.round(count / 5), 30));
    console.log(`  ${date}: ${bar} ${count}`);
  }
  console.log();

  // Top topics
  if (summary.topTopics.length > 0) {
    console.log("TOP TOPICS THIS WEEK");
    console.log("-".repeat(40));
    for (const topic of summary.topTopics) {
      console.log(`  ${topic.topic}: ${topic.count} events (${topic.timeMinutes} min)`);
    }
    console.log();
  }

  // Sessions breakdown
  if (sessions.length > 0) {
    console.log("WORK SESSIONS");
    console.log("-".repeat(40));
    for (const session of sessions.slice(0, 10)) {
      const date = dayjs(session.startTime).format("MM/DD HH:mm");
      const duration = session.durationMinutes;
      console.log(`  ${date} - ${duration} min: ${session.description}`);
    }
    if (sessions.length > 10) {
      console.log(`  ... and ${sessions.length - 10} more sessions`);
    }
    console.log();
  }

  // Aha moments
  if (summary.ahaMonments.length > 0) {
    console.log("BREAKTHROUGH MOMENTS");
    console.log("-".repeat(40));
    for (const aha of summary.ahaMonments) {
      const date = dayjs(aha.timestamp).format("MM/DD HH:mm");
      console.log(`  ${date}: ${aha.description}`);
    }
    console.log();
  }

  // Content opportunities
  if (ideas.length > 0) {
    console.log("TOP CONTENT OPPORTUNITIES");
    console.log("-".repeat(40));
    const topIdeas = ideas.slice(0, 5);
    for (let i = 0; i < topIdeas.length; i++) {
      const idea = topIdeas[i];
      console.log(`  ${i + 1}. ${idea.title}`);
      console.log(`     "${idea.hook}"`);
      console.log(`     Format: ${idea.suggestedFormat} | Confidence: ${idea.confidence}`);
      console.log();
    }
  }

  // Patterns and insights
  console.log("WEEKLY PATTERNS");
  console.log("-".repeat(40));

  // Most productive day
  const [mostProductiveDay] = dailyEvents.sort((a, b) => b[1] - a[1]);
  if (mostProductiveDay) {
    console.log(`  Most Active Day: ${mostProductiveDay[0]} (${mostProductiveDay[1]} events)`);
  }

  // High struggle clusters (potential content gold)
  const struggleClusters = clusters.filter((c) => c.struggleScore >= 50);
  if (struggleClusters.length > 0) {
    console.log(`  Debugging Sessions: ${struggleClusters.length} (high content potential)`);
  }

  // Learning signals
  const explorationSignals = clusters.filter((c) =>
    c.signals.some((s) => s.type === "exploration")
  );
  if (explorationSignals.length > 0) {
    console.log(`  Exploration Sessions: ${explorationSignals.length}`);
  }

  console.log();

  // Verbose mode: show all clusters
  if (verbose) {
    console.log("ALL CLUSTERS (VERBOSE)");
    console.log("-".repeat(40));
    for (const cluster of clusters) {
      const start = dayjs(cluster.startTime).format("MM/DD HH:mm");
      console.log(`  ${cluster.topic} (${start}, ${cluster.durationMinutes} min)`);
      console.log(`    Events: ${cluster.events.length}, Struggle: ${cluster.struggleScore}%, Aha: ${cluster.ahaIndex}%`);
      if (cluster.signals.length > 0) {
        console.log(`    Signals: ${cluster.signals.map((s) => s.type).join(", ")}`);
      }
    }
    console.log();
  }
}

function getDailyBreakdown(events: Event[]): [string, number][] {
  const dailyCounts = new Map<string, number>();

  for (const event of events) {
    const date = dayjs(event.timestamp).format("YYYY-MM-DD");
    dailyCounts.set(date, (dailyCounts.get(date) || 0) + 1);
  }

  return [...dailyCounts.entries()].sort((a, b) => a[0].localeCompare(b[0]));
}
