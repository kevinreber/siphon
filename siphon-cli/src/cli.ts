#!/usr/bin/env node

/**
 * Siphon CLI
 *
 * Turn your dev work into content. Captures developer activity
 * and generates content ideas, video storyboards, and more.
 */

import { Command } from 'commander';
import { captureCommand } from './commands/capture.js';
import { digestCommand } from './commands/digest.js';
import { exportCommand } from './commands/export.js';
import { insightsCommand } from './commands/insights.js';
import { statusCommand } from './commands/status.js';
import { summaryCommand } from './commands/summary.js';

const program = new Command();

program.name('siphon').description('Turn your dev work into content').version('0.1.0');

program
  .command('capture')
  .description('Capture and analyze recent developer activity')
  .option('-t, --time <duration>', 'Time window to analyze (e.g., 2h, 30m)', '2h')
  .option('--prompt', 'Generate a Claude prompt for enhanced analysis')
  .option('--generate', 'Generate polished content ideas using Claude API')
  .option('--topic <topic>', 'Focus on a specific topic')
  .option('--no-browser', 'Exclude browser history')
  .option('-v, --verbose', 'Show detailed event breakdown')
  .action(captureCommand);

program
  .command('status')
  .description("Show what you've been working on recently")
  .option('-t, --time <duration>', 'Time window (e.g., 2h, 30m)', '1h')
  .action(statusCommand);

program
  .command('summary')
  .description('Generate a summary of your work session')
  .option('-t, --time <duration>', 'Time window (e.g., 2h, 4h)', '4h')
  .action(summaryCommand);

program
  .command('digest')
  .description('Generate a weekly/multi-day digest of your development activity')
  .option('-d, --days <days>', 'Number of days to include (default: 7)', '7')
  .option('--generate', 'Generate polished content ideas using Claude API')
  .option('-v, --verbose', 'Show detailed cluster breakdown')
  .action(digestCommand);

program
  .command('export')
  .description('Export analysis results to various formats')
  .option('-t, --time <duration>', 'Time window to analyze (e.g., 2h, 1d)', '4h')
  .option(
    '-f, --format <format>',
    'Output format: markdown, obsidian, json, rss, notion',
    'markdown'
  )
  .option('-o, --output <path>', 'Output file path')
  .option('--daily', 'Generate Obsidian daily note entry')
  .option('-v, --verbose', 'Include detailed analysis')
  .action(exportCommand);

program
  .command('insights')
  .description('View weekly insights and trends')
  .option('-d, --days <days>', 'Number of days to analyze (default: 7)', '7')
  .option('-v, --verbose', 'Show detailed daily breakdown')
  .action(insightsCommand);

program.parse();
