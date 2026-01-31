/**
 * Status Command
 *
 * Shows a quick overview of recent activity.
 */

import { ShellHistoryCollector } from "../collectors/shell.js";
import { Event, ShellEventData } from "../types.js";

interface StatusOptions {
  time: string;
}

function parseTimeDuration(duration: string): number {
  const match = duration.match(/^(\d+)(h|m|s)?$/);
  if (!match) {
    throw new Error(`Invalid duration: ${duration}`);
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

export async function statusCommand(options: StatusOptions): Promise<void> {
  const durationMs = parseTimeDuration(options.time);
  const endTime = new Date();
  const startTime = new Date(endTime.getTime() - durationMs);

  console.log(`\nActivity in the last ${options.time}:\n`);

  // Collect shell history
  const shellCollector = new ShellHistoryCollector();
  let events: Event[] = [];

  try {
    events = await shellCollector.collect(startTime, endTime);
  } catch (err) {
    console.log("Could not read shell history.");
    return;
  }

  if (events.length === 0) {
    console.log("No commands found in the specified time range.");
    return;
  }

  // Group by tool (first word of command)
  const toolCounts = new Map<string, number>();
  let failedCount = 0;

  for (const event of events) {
    const data = event.data as ShellEventData;
    const tool = data.command.split(/\s+/)[0];
    toolCounts.set(tool, (toolCounts.get(tool) || 0) + 1);
    if (data.exitCode !== 0) {
      failedCount++;
    }
  }

  // Sort by count
  const sortedTools = [...toolCounts.entries()].sort((a, b) => b[1] - a[1]);

  console.log(`Commands: ${events.length} (${failedCount} failed)`);
  console.log();
  console.log("Tools used:");
  for (const [tool, count] of sortedTools.slice(0, 10)) {
    const bar = "█".repeat(Math.min(Math.ceil(count / 2), 20));
    console.log(`  ${tool.padEnd(15)} ${bar} ${count}`);
  }

  // Show recent commands
  console.log();
  console.log("Recent commands:");
  const recentEvents = events.slice(-10).reverse();
  for (const event of recentEvents) {
    const data = event.data as ShellEventData;
    const status = data.exitCode === 0 ? "✓" : "✗";
    const cmd = data.command.length > 60 ? data.command.slice(0, 60) + "..." : data.command;
    const time = event.timestamp.toLocaleTimeString("en-US", {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    });
    console.log(`  ${time} ${status} ${cmd}`);
  }

  console.log();
  console.log(`Run 'siphon capture --time ${options.time}' for content ideas.`);
}
