/**
 * Insights Command
 *
 * Generates weekly insights dashboard showing trends,
 * productivity patterns, and content potential over time.
 */

import chalk from 'chalk';
import { Analyzer } from '../analyzer.js';
import { BrowserHistoryCollector } from '../collectors/browser.js';
import { GitCollector } from '../collectors/git.js';
import { ShellHistoryCollector } from '../collectors/shell.js';
import type { AnalysisResult, Event } from '../types.js';

interface InsightsOptions {
  days: string;
  verbose?: boolean;
}

interface DailyStats {
  date: string;
  events: number;
  commands: number;
  failedCommands: number;
  struggleScore: number;
  topTopics: string[];
  contentPotential: number;
  ideasCount: number;
}

interface WeeklyInsights {
  period: { start: Date; end: Date };
  dailyStats: DailyStats[];
  totalEvents: number;
  totalCommands: number;
  avgStruggleScore: number;
  topTopicsOverall: Array<{ topic: string; count: number; days: number }>;
  productivityTrend: 'increasing' | 'decreasing' | 'stable';
  bestDay: { date: string; events: number };
  worstDay: { date: string; events: number };
  contentPotentialScore: number;
  totalIdeas: number;
  narratives: Array<{ topic: string; duration: number; days: string[] }>;
}

export async function insightsCommand(options: InsightsOptions): Promise<void> {
  const days = Number.parseInt(options.days, 10);
  const endTime = new Date();
  const startTime = new Date(endTime.getTime() - days * 24 * 60 * 60 * 1000);

  console.log(chalk.bold(`\n📊 Developer Insights - Last ${days} Days\n`));
  console.log(chalk.gray(`${formatDate(startTime)} → ${formatDate(endTime)}`));
  console.log(chalk.gray('─'.repeat(50)));

  // Collect all events for the period
  const allEvents: Event[] = [];

  console.log('\nCollecting data...');

  const shellCollector = new ShellHistoryCollector();
  try {
    const shellEvents = await shellCollector.collect(startTime, endTime);
    allEvents.push(...shellEvents);
    console.log(chalk.gray(`  Shell: ${shellEvents.length} commands`));
  } catch (_err) {
    console.log(chalk.gray('  Shell: unavailable'));
  }

  const gitCollector = new GitCollector();
  try {
    const gitEvents = await gitCollector.collect(startTime, endTime);
    allEvents.push(...gitEvents);
    console.log(chalk.gray(`  Git: ${gitEvents.length} commits`));
  } catch (_err) {
    console.log(chalk.gray('  Git: unavailable'));
  }

  const browserCollector = new BrowserHistoryCollector();
  if (browserCollector.isAvailable()) {
    try {
      const browserEvents = await browserCollector.collect(startTime, endTime, true);
      allEvents.push(...browserEvents);
      console.log(chalk.gray(`  Browser: ${browserEvents.length} visits`));
    } catch (_err) {
      console.log(chalk.gray('  Browser: unavailable'));
    }
  }

  if (allEvents.length === 0) {
    console.log(chalk.yellow('\nNo events found for this period.'));
    return;
  }

  // Analyze by day
  const insights = await generateInsights(allEvents, startTime, endTime, days);

  // Display insights
  displayInsights(insights, options.verbose);
}

async function generateInsights(
  events: Event[],
  startTime: Date,
  endTime: Date,
  days: number
): Promise<WeeklyInsights> {
  const dailyStats: DailyStats[] = [];
  const topicCounts = new Map<string, { count: number; days: Set<string> }>();
  const analyzer = new Analyzer();
  let totalIdeas = 0;

  // Group events by day
  const eventsByDay = new Map<string, Event[]>();

  for (const event of events) {
    const dateKey = event.timestamp.toISOString().split('T')[0];
    if (!eventsByDay.has(dateKey)) {
      eventsByDay.set(dateKey, []);
    }
    eventsByDay.get(dateKey)?.push(event);
  }

  // Analyze each day
  for (let d = 0; d < days; d++) {
    const dayDate = new Date(startTime.getTime() + d * 24 * 60 * 60 * 1000);
    const dateKey = dayDate.toISOString().split('T')[0];
    const dayEvents = eventsByDay.get(dateKey) || [];

    if (dayEvents.length === 0) {
      dailyStats.push({
        date: dateKey,
        events: 0,
        commands: 0,
        failedCommands: 0,
        struggleScore: 0,
        topTopics: [],
        contentPotential: 0,
        ideasCount: 0,
      });
      continue;
    }

    const dayStart = new Date(dayDate);
    dayStart.setHours(0, 0, 0, 0);
    const dayEnd = new Date(dayDate);
    dayEnd.setHours(23, 59, 59, 999);

    const result = analyzer.analyze(dayEvents, { start: dayStart, end: dayEnd });

    // Track topics
    for (const topic of result.summary.topTopics) {
      if (!topicCounts.has(topic.topic)) {
        topicCounts.set(topic.topic, { count: 0, days: new Set() });
      }
      const entry = topicCounts.get(topic.topic)!;
      entry.count += topic.count;
      entry.days.add(dateKey);
    }

    totalIdeas += result.ideas.length;

    dailyStats.push({
      date: dateKey,
      events: result.summary.totalEvents,
      commands: result.summary.totalCommands,
      failedCommands: result.summary.failedCommands,
      struggleScore: result.summary.struggleScore,
      topTopics: result.summary.topTopics.slice(0, 3).map((t) => t.topic),
      contentPotential: calculateContentPotential(result),
      ideasCount: result.ideas.length,
    });
  }

  // Calculate overall stats
  const activeDays = dailyStats.filter((d) => d.events > 0);
  const totalEvents = dailyStats.reduce((sum, d) => sum + d.events, 0);
  const totalCommands = dailyStats.reduce((sum, d) => sum + d.commands, 0);
  const avgStruggleScore =
    activeDays.length > 0
      ? Math.round(activeDays.reduce((sum, d) => sum + d.struggleScore, 0) / activeDays.length)
      : 0;

  // Find best and worst days
  const sortedByEvents = [...activeDays].sort((a, b) => b.events - a.events);
  const bestDay = sortedByEvents[0] || { date: 'N/A', events: 0 };
  const worstDay = sortedByEvents[sortedByEvents.length - 1] || { date: 'N/A', events: 0 };

  // Calculate productivity trend
  const firstHalf = dailyStats.slice(0, Math.floor(dailyStats.length / 2));
  const secondHalf = dailyStats.slice(Math.floor(dailyStats.length / 2));
  const firstHalfAvg =
    firstHalf.reduce((sum, d) => sum + d.events, 0) / Math.max(firstHalf.length, 1);
  const secondHalfAvg =
    secondHalf.reduce((sum, d) => sum + d.events, 0) / Math.max(secondHalf.length, 1);

  let productivityTrend: 'increasing' | 'decreasing' | 'stable' = 'stable';
  if (secondHalfAvg > firstHalfAvg * 1.2) {
    productivityTrend = 'increasing';
  } else if (secondHalfAvg < firstHalfAvg * 0.8) {
    productivityTrend = 'decreasing';
  }

  // Top topics overall
  const topTopicsOverall = [...topicCounts.entries()]
    .map(([topic, data]) => ({ topic, count: data.count, days: data.days.size }))
    .sort((a, b) => b.count - a.count)
    .slice(0, 10);

  // Detect multi-day narratives
  const narratives = detectNarratives(topicCounts, dailyStats);

  // Overall content potential
  const contentPotentialScore = Math.round(
    dailyStats.reduce((sum, d) => sum + d.contentPotential, 0) / Math.max(activeDays.length, 1)
  );

  return {
    period: { start: startTime, end: endTime },
    dailyStats,
    totalEvents,
    totalCommands,
    avgStruggleScore,
    topTopicsOverall,
    productivityTrend,
    bestDay: { date: bestDay.date, events: bestDay.events },
    worstDay: { date: worstDay.date, events: worstDay.events },
    contentPotentialScore,
    totalIdeas,
    narratives,
  };
}

function calculateContentPotential(result: AnalysisResult): number {
  let score = 0;

  // High struggle = good content
  score += Math.min(result.summary.struggleScore * 0.4, 40);

  // More ideas = more potential
  score += Math.min(result.ideas.length * 10, 30);

  // High-confidence ideas
  const highConfidence = result.ideas.filter((i) => i.confidence === 'high').length;
  score += highConfidence * 10;

  // Aha moments
  score += Math.min(result.summary.ahaMonments.length * 10, 20);

  return Math.min(Math.round(score), 100);
}

function detectNarratives(
  topicCounts: Map<string, { count: number; days: Set<string> }>,
  _dailyStats: DailyStats[]
): Array<{ topic: string; duration: number; days: string[] }> {
  const narratives: Array<{ topic: string; duration: number; days: string[] }> = [];

  for (const [topic, data] of topicCounts) {
    if (data.days.size >= 2) {
      // Topic spans multiple days
      const sortedDays = [...data.days].sort();
      const firstDay = new Date(sortedDays[0]);
      const lastDay = new Date(sortedDays[sortedDays.length - 1]);
      const duration =
        Math.ceil((lastDay.getTime() - firstDay.getTime()) / (24 * 60 * 60 * 1000)) + 1;

      narratives.push({
        topic,
        duration,
        days: sortedDays,
      });
    }
  }

  // Sort by duration (longest first)
  return narratives.sort((a, b) => b.duration - a.duration).slice(0, 5);
}

function displayInsights(insights: WeeklyInsights, verbose?: boolean): void {
  console.log('\n');

  // Overview stats
  console.log(chalk.bold('📈 Overview'));
  console.log(chalk.gray('─'.repeat(40)));
  console.log(`Total Events: ${chalk.cyan(insights.totalEvents.toString())}`);
  console.log(`Total Commands: ${chalk.cyan(insights.totalCommands.toString())}`);
  console.log(`Avg Struggle Score: ${colorStruggle(insights.avgStruggleScore)}`);
  console.log(`Content Potential: ${colorPotential(insights.contentPotentialScore)}`);
  console.log(`Total Ideas Generated: ${chalk.cyan(insights.totalIdeas.toString())}`);
  console.log('');

  // Productivity trend
  const trendEmoji =
    insights.productivityTrend === 'increasing'
      ? '📈'
      : insights.productivityTrend === 'decreasing'
        ? '📉'
        : '➡️';
  const trendColor =
    insights.productivityTrend === 'increasing'
      ? chalk.green
      : insights.productivityTrend === 'decreasing'
        ? chalk.red
        : chalk.yellow;
  console.log(chalk.bold('🔄 Trend'));
  console.log(chalk.gray('─'.repeat(40)));
  console.log(`Productivity: ${trendEmoji} ${trendColor(insights.productivityTrend)}`);
  console.log(
    `Best Day: ${chalk.green(insights.bestDay.date)} (${insights.bestDay.events} events)`
  );
  console.log(
    `Slowest Day: ${chalk.red(insights.worstDay.date)} (${insights.worstDay.events} events)`
  );
  console.log('');

  // Top topics
  console.log(chalk.bold('🏷️  Top Topics'));
  console.log(chalk.gray('─'.repeat(40)));
  for (const topic of insights.topTopicsOverall.slice(0, 5)) {
    const bar = '█'.repeat(Math.min(Math.ceil(topic.count / 10), 20));
    console.log(`${topic.topic.padEnd(15)} ${chalk.cyan(bar)} ${topic.count} (${topic.days} days)`);
  }
  console.log('');

  // Multi-day narratives
  if (insights.narratives.length > 0) {
    console.log(chalk.bold('📖 Multi-Day Projects'));
    console.log(chalk.gray('─'.repeat(40)));
    for (const narrative of insights.narratives) {
      console.log(
        `${chalk.yellow(narrative.topic)} - ${narrative.duration} days ${chalk.gray(`(${narrative.days[0]} → ${narrative.days[narrative.days.length - 1]})`)}`
      );
    }
    console.log('');
  }

  // Activity heatmap (simple ASCII version)
  console.log(chalk.bold('📅 Activity Heatmap'));
  console.log(chalk.gray('─'.repeat(40)));

  const maxEvents = Math.max(...insights.dailyStats.map((d) => d.events));
  const heatmapRow = insights.dailyStats
    .slice(-14)
    .map((d) => {
      if (d.events === 0) return chalk.gray('░');
      const intensity = d.events / maxEvents;
      if (intensity > 0.75) return chalk.green('█');
      if (intensity > 0.5) return chalk.yellow('▓');
      if (intensity > 0.25) return chalk.cyan('▒');
      return chalk.blue('░');
    })
    .join('');

  console.log(`Last 14 days: ${heatmapRow}`);
  console.log(
    chalk.gray(
      `             ${insights.dailyStats
        .slice(-14)
        .map((d) => d.date.slice(-2))
        .join('')}`
    )
  );
  console.log('');

  // Verbose daily breakdown
  if (verbose) {
    console.log(chalk.bold('📊 Daily Breakdown'));
    console.log(chalk.gray('─'.repeat(60)));
    console.log(chalk.gray('Date       | Events | Commands | Struggle | Potential | Topics'));
    console.log(chalk.gray('─'.repeat(60)));

    for (const day of insights.dailyStats.slice(-14)) {
      if (day.events > 0) {
        console.log(
          `${day.date} | ${day.events.toString().padStart(6)} | ${day.commands.toString().padStart(8)} | ${day.struggleScore.toString().padStart(8)}% | ${day.contentPotential.toString().padStart(9)}% | ${day.topTopics.slice(0, 2).join(', ')}`
        );
      }
    }
    console.log('');
  }

  // Recommendations
  console.log(chalk.bold('💡 Recommendations'));
  console.log(chalk.gray('─'.repeat(40)));

  if (insights.narratives.length > 0) {
    const topNarrative = insights.narratives[0];
    console.log(
      `• Consider writing about your ${chalk.yellow(topNarrative.topic)} journey (${topNarrative.duration} days)`
    );
  }

  if (insights.avgStruggleScore > 40) {
    console.log('• High struggle score suggests good debugging content opportunities');
  }

  if (insights.contentPotentialScore > 60) {
    console.log(
      `• ${chalk.green('High content potential!')} Run 'siphon capture --generate' to get ideas`
    );
  }

  if (insights.productivityTrend === 'decreasing') {
    console.log(`• Activity declining - consider documenting what you've accomplished`);
  }

  console.log('');
}

function formatDate(date: Date): string {
  return date.toLocaleDateString('en-US', {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  });
}

function colorStruggle(score: number): string {
  if (score > 60) return chalk.red(`${score}%`);
  if (score > 30) return chalk.yellow(`${score}%`);
  return chalk.green(`${score}%`);
}

function colorPotential(score: number): string {
  if (score > 70) return chalk.green(`${score}%`);
  if (score > 40) return chalk.yellow(`${score}%`);
  return chalk.red(`${score}%`);
}
