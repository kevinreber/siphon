/**
 * Export Command
 *
 * Exports analysis results to various formats including
 * Markdown, JSON, RSS, Obsidian, and Notion.
 */

import * as path from 'node:path';
import { Analyzer } from '../analyzer.js';
import { BrowserHistoryCollector } from '../collectors/browser.js';
import { GitCollector } from '../collectors/git.js';
import { ShellHistoryCollector } from '../collectors/shell.js';
import { exportResults, generateObsidianDailyEntry, type ExportOptions } from '../export.js';
import type { Event } from '../types.js';

interface ExportCommandOptions {
  time: string;
  format: 'markdown' | 'obsidian' | 'json' | 'rss' | 'notion';
  output?: string;
  daily?: boolean;
  verbose?: boolean;
}

/**
 * Parse time duration string to milliseconds
 */
function parseTimeDuration(duration: string): number {
  const match = duration.match(/^(\d+)(h|m|d)?$/);
  if (!match) {
    throw new Error(`Invalid duration: ${duration}`);
  }

  const value = Number.parseInt(match[1], 10);
  const unit = match[2] || 'h';

  switch (unit) {
    case 'd':
      return value * 24 * 60 * 60 * 1000;
    case 'h':
      return value * 60 * 60 * 1000;
    case 'm':
      return value * 60 * 1000;
    default:
      return value * 60 * 60 * 1000;
  }
}

export async function exportCommand(options: ExportCommandOptions): Promise<void> {
  const durationMs = parseTimeDuration(options.time);
  const endTime = new Date();
  const startTime = new Date(endTime.getTime() - durationMs);

  console.log(`\nCollecting activity from the last ${options.time}...\n`);

  // Collect events
  const events: Event[] = [];

  const shellCollector = new ShellHistoryCollector();
  try {
    const shellEvents = await shellCollector.collect(startTime, endTime);
    events.push(...shellEvents);
  } catch (_err) {
    // Silently continue
  }

  const gitCollector = new GitCollector();
  try {
    const gitEvents = await gitCollector.collect(startTime, endTime);
    events.push(...gitEvents);
  } catch (_err) {
    // Silently continue
  }

  const browserCollector = new BrowserHistoryCollector();
  if (browserCollector.isAvailable()) {
    try {
      const browserEvents = await browserCollector.collect(startTime, endTime, true);
      events.push(...browserEvents);
    } catch (_err) {
      // Silently continue
    }
  }

  if (events.length === 0) {
    console.log('No events found to export.');
    return;
  }

  console.log(`Found ${events.length} events`);

  // Analyze events
  const analyzer = new Analyzer();
  const result = analyzer.analyze(events, { start: startTime, end: endTime });

  // Handle daily note format for Obsidian
  if (options.daily && options.format === 'obsidian') {
    const entry = generateObsidianDailyEntry(result);
    console.log('\nObsidian Daily Entry:\n');
    console.log(entry);
    return;
  }

  // Export based on format
  const exportOptions: ExportOptions = {
    format: options.format,
    outputPath: options.output,
    includeAnalysis: options.verbose,
    title: `Siphon Export - ${new Date().toISOString().split('T')[0]}`,
  };

  try {
    const output = await exportResults(result, exportOptions);

    if (options.output) {
      console.log(`\nExported to: ${output}`);
    } else {
      console.log(`\n${'='.repeat(60)}`);
      console.log(`${options.format.toUpperCase()} OUTPUT`);
      console.log(`${'='.repeat(60)}\n`);
      console.log(output);
    }
  } catch (err) {
    console.error('Export failed:', err instanceof Error ? err.message : String(err));
    process.exit(1);
  }
}
