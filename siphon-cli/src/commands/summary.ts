/**
 * Summary Command
 *
 * Generates a human-readable summary of a work session.
 */

import { Analyzer } from '../analyzer.js';
import { GitCollector } from '../collectors/git.js';
import { ShellHistoryCollector } from '../collectors/shell.js';
import type { Event } from '../types.js';

interface SummaryOptions {
  time: string;
}

function parseTimeDuration(duration: string): number {
  const match = duration.match(/^(\d+)(h|m|s)?$/);
  if (!match) {
    throw new Error(`Invalid duration: ${duration}`);
  }

  const value = Number.parseInt(match[1], 10);
  const unit = match[2] || 'm';

  switch (unit) {
    case 'h':
      return value * 60 * 60 * 1000;
    case 'm':
      return value * 60 * 1000;
    case 's':
      return value * 1000;
    default:
      return value * 60 * 1000;
  }
}

// biome-ignore lint/complexity/noExcessiveCognitiveComplexity: Complex command with multiple display paths
export async function summaryCommand(options: SummaryOptions): Promise<void> {
  const durationMs = parseTimeDuration(options.time);
  const endTime = new Date();
  const startTime = new Date(endTime.getTime() - durationMs);

  // Collect events
  const events: Event[] = [];

  const shellCollector = new ShellHistoryCollector();
  try {
    events.push(...(await shellCollector.collect(startTime, endTime)));
  } catch (_err) {
    // Ignore
  }

  const gitCollector = new GitCollector();
  try {
    events.push(...(await gitCollector.collect(startTime, endTime)));
  } catch (_err) {
    // Ignore
  }

  if (events.length === 0) {
    console.log('\nNo activity found in the specified time range.');
    return;
  }

  // Analyze
  const analyzer = new Analyzer();
  const result = analyzer.analyze(events, { start: startTime, end: endTime });

  // Generate summary
  console.log();
  console.log('╔════════════════════════════════════════════════════════════╗');
  console.log('║                     SESSION SUMMARY                         ║');
  console.log('╚════════════════════════════════════════════════════════════╝');
  console.log();

  // Time info
  const hours = Math.floor(result.timeRange.durationMinutes / 60);
  const mins = result.timeRange.durationMinutes % 60;
  const timeStr = hours > 0 ? `${hours}h ${mins}m` : `${mins}m`;

  console.log(`📅 ${formatDate(startTime)} | ⏱️  ${timeStr}`);
  console.log();

  // What you worked on
  if (result.summary.topTopics.length > 0) {
    console.log('📝 What you worked on:');
    for (const topic of result.summary.topTopics.slice(0, 3)) {
      const topicTime = topic.timeMinutes > 0 ? ` (${topic.timeMinutes} min)` : '';
      console.log(`   • ${capitalize(topic.topic)}${topicTime}`);
    }
    console.log();
  }

  // Struggle indicator
  if (result.summary.struggleScore > 30) {
    const emoji = result.summary.struggleScore > 60 ? '🔥' : '💪';
    console.log(`${emoji} Debugging intensity: ${result.summary.struggleScore}%`);
    console.log(
      `   ${result.summary.failedCommands} commands failed out of ${result.summary.totalCommands}`
    );
    console.log();
  }

  // Aha moments
  if (result.summary.ahaMonments.length > 0) {
    console.log('💡 Breakthrough moments:');
    for (const aha of result.summary.ahaMonments) {
      console.log(`   • ${aha.description}`);
    }
    console.log();
  }

  // Content potential
  if (result.ideas.length > 0) {
    const highConfidence = result.ideas.filter((i) => i.confidence === 'high');
    const medConfidence = result.ideas.filter((i) => i.confidence === 'medium');

    console.log('🎬 Content potential:');
    if (highConfidence.length > 0) {
      console.log(`   ${highConfidence.length} high-confidence idea(s)`);
    }
    if (medConfidence.length > 0) {
      console.log(`   ${medConfidence.length} medium-confidence idea(s)`);
    }
    console.log();
    console.log("   Run 'siphon capture' for detailed content ideas.");
  }

  console.log();
  console.log('─'.repeat(60));
}

function formatDate(date: Date): string {
  return date.toLocaleDateString('en-US', {
    weekday: 'short',
    month: 'short',
    day: 'numeric',
  });
}

function capitalize(str: string): string {
  return str.charAt(0).toUpperCase() + str.slice(1);
}
