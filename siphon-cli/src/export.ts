/**
 * Export Module
 *
 * Exports content ideas and analysis results to various formats
 * including Markdown (Obsidian), JSON, and RSS.
 */

import * as fs from 'node:fs';
import type { AnalysisResult } from './types.js';

export interface ExportOptions {
  format: 'markdown' | 'obsidian' | 'json' | 'rss' | 'notion';
  outputPath?: string;
  includeAnalysis?: boolean;
  title?: string;
}

/**
 * Export analysis results to various formats
 */
export async function exportResults(
  result: AnalysisResult,
  options: ExportOptions
): Promise<string> {
  switch (options.format) {
    case 'markdown':
    case 'obsidian':
      return exportToMarkdown(result, options);
    case 'json':
      return exportToJson(result, options);
    case 'rss':
      return exportToRss(result, options);
    case 'notion':
      return exportToNotion(result, options);
    default:
      throw new Error(`Unknown export format: ${options.format}`);
  }
}

/**
 * Export to Markdown (Obsidian-compatible)
 */
// biome-ignore lint/complexity/noExcessiveCognitiveComplexity: Complex markdown generation with multiple sections
function exportToMarkdown(result: AnalysisResult, options: ExportOptions): string {
  const title = options.title || `Siphon Capture - ${formatDate(result.timeRange.start)}`;
  const lines: string[] = [];

  // YAML frontmatter for Obsidian
  lines.push('---');
  lines.push(`title: "${title}"`);
  lines.push(`date: ${result.timeRange.start.toISOString()}`);
  lines.push(`duration_minutes: ${result.timeRange.durationMinutes}`);
  lines.push(`total_events: ${result.summary.totalEvents}`);
  lines.push(`struggle_score: ${result.summary.struggleScore}`);
  lines.push(`topics: [${result.summary.topTopics.map((t) => `"${t.topic}"`).join(', ')}]`);
  lines.push('tags: [siphon, dev-activity]');
  lines.push('---');
  lines.push('');

  // Header
  lines.push(`# ${title}`);
  lines.push('');

  // Summary section
  lines.push('## Summary');
  lines.push('');
  lines.push(`- **Duration:** ${result.timeRange.durationMinutes} minutes`);
  lines.push(`- **Total Events:** ${result.summary.totalEvents}`);
  lines.push(
    `- **Commands:** ${result.summary.totalCommands} (${result.summary.failedCommands} failed)`
  );
  lines.push(`- **Struggle Score:** ${result.summary.struggleScore}%`);
  lines.push(`- **Sessions:** ${result.summary.sessionCount}`);
  lines.push('');

  // Topics section
  if (result.summary.topTopics.length > 0) {
    lines.push('## Topics');
    lines.push('');
    for (const topic of result.summary.topTopics) {
      lines.push(`- **${topic.topic}:** ${topic.count} events (${topic.timeMinutes} min)`);
    }
    lines.push('');
  }

  // Aha moments
  if (result.summary.ahaMonments.length > 0) {
    lines.push('## Breakthroughs');
    lines.push('');
    for (const aha of result.summary.ahaMonments) {
      lines.push(`- ${aha.description} at ${formatTime(aha.timestamp)}`);
    }
    lines.push('');
  }

  // Content ideas
  if (result.ideas.length > 0) {
    lines.push('## Content Ideas');
    lines.push('');
    for (const idea of result.ideas) {
      const confidence =
        idea.confidence === 'high' ? '🟢' : idea.confidence === 'medium' ? '🟡' : '🔴';
      lines.push(`### ${confidence} ${idea.title}`);
      lines.push('');
      lines.push(`> ${idea.hook}`);
      lines.push('');
      lines.push(`- **Angle:** ${idea.angle}`);
      lines.push(`- **Format:** ${idea.suggestedFormat}`);
      lines.push(`- **Evidence:** ${idea.evidence.join('; ')}`);
      lines.push('');
    }
  }

  // Clusters (detailed analysis)
  if (options.includeAnalysis && result.clusters.length > 0) {
    lines.push('## Detailed Clusters');
    lines.push('');
    for (const cluster of result.clusters) {
      lines.push(`### ${cluster.topic}`);
      lines.push('');
      lines.push(`- **Duration:** ${cluster.durationMinutes} minutes`);
      lines.push(`- **Events:** ${cluster.events.length}`);
      lines.push(`- **Confidence:** ${cluster.confidence}`);
      lines.push(`- **Struggle Score:** ${cluster.struggleScore}%`);
      lines.push(`- **Aha Index:** ${cluster.ahaIndex}%`);
      if (cluster.signals.length > 0) {
        lines.push(`- **Signals:** ${cluster.signals.map((s) => s.type).join(', ')}`);
      }
      lines.push('');
    }
  }

  const content = lines.join('\n');

  // Write to file if path specified
  if (options.outputPath) {
    const outputPath = options.outputPath.endsWith('.md')
      ? options.outputPath
      : `${options.outputPath}.md`;
    fs.writeFileSync(outputPath, content, 'utf-8');
    return outputPath;
  }

  return content;
}

/**
 * Export to JSON
 */
function exportToJson(result: AnalysisResult, options: ExportOptions): string {
  const exportData = {
    exported_at: new Date().toISOString(),
    time_range: {
      start: result.timeRange.start.toISOString(),
      end: result.timeRange.end.toISOString(),
      duration_minutes: result.timeRange.durationMinutes,
    },
    summary: {
      total_events: result.summary.totalEvents,
      total_commands: result.summary.totalCommands,
      failed_commands: result.summary.failedCommands,
      struggle_score: result.summary.struggleScore,
      session_count: result.summary.sessionCount,
      average_session_minutes: result.summary.averageSessionMinutes,
      top_topics: result.summary.topTopics,
      aha_moments: result.summary.ahaMonments.map((a) => ({
        description: a.description,
        timestamp: a.timestamp.toISOString(),
      })),
    },
    ideas: result.ideas.map((idea) => ({
      title: idea.title,
      hook: idea.hook,
      angle: idea.angle,
      confidence: idea.confidence,
      suggested_format: idea.suggestedFormat,
      evidence: idea.evidence,
    })),
    clusters: options.includeAnalysis
      ? result.clusters.map((c) => ({
          id: c.id,
          topic: c.topic,
          start_time: c.startTime.toISOString(),
          end_time: c.endTime.toISOString(),
          duration_minutes: c.durationMinutes,
          event_count: c.events.length,
          confidence: c.confidence,
          struggle_score: c.struggleScore,
          aha_index: c.ahaIndex,
          signals: c.signals,
        }))
      : undefined,
  };

  const content = JSON.stringify(exportData, null, 2);

  if (options.outputPath) {
    const outputPath = options.outputPath.endsWith('.json')
      ? options.outputPath
      : `${options.outputPath}.json`;
    fs.writeFileSync(outputPath, content, 'utf-8');
    return outputPath;
  }

  return content;
}

/**
 * Export to RSS feed
 */
function exportToRss(result: AnalysisResult, options: ExportOptions): string {
  const title = options.title || 'Siphon Content Ideas';
  const date = new Date().toUTCString();

  const items = result.ideas
    .map(
      (idea) => `
    <item>
      <title><![CDATA[${idea.title}]]></title>
      <description><![CDATA[${idea.hook}

Angle: ${idea.angle}
Format: ${idea.suggestedFormat}
Confidence: ${idea.confidence}

Evidence:
${idea.evidence.map((e) => `- ${e}`).join('\n')}]]></description>
      <pubDate>${result.timeRange.end.toUTCString()}</pubDate>
      <guid>${`siphon-idea-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`}</guid>
      <category>${idea.suggestedFormat}</category>
    </item>`
    )
    .join('\n');

  const rss = `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>${title}</title>
    <description>Content ideas generated from developer activity</description>
    <link>https://github.com/siphon-dev/siphon</link>
    <lastBuildDate>${date}</lastBuildDate>
    <generator>Siphon CLI</generator>
${items}
  </channel>
</rss>`;

  if (options.outputPath) {
    const outputPath = options.outputPath.endsWith('.xml')
      ? options.outputPath
      : `${options.outputPath}.xml`;
    fs.writeFileSync(outputPath, rss, 'utf-8');
    return outputPath;
  }

  return rss;
}

/**
 * Export to Notion-compatible format (Markdown with Notion blocks)
 */
// biome-ignore lint/complexity/noExcessiveCognitiveComplexity: Complex Notion format generation
function exportToNotion(result: AnalysisResult, options: ExportOptions): string {
  const title = options.title || `Siphon Capture - ${formatDate(result.timeRange.start)}`;
  const lines: string[] = [];

  // Notion page header
  lines.push(`# ${title}`);
  lines.push('');

  // Summary callout
  lines.push('> 📊 **Session Summary**');
  lines.push(`> Duration: ${result.timeRange.durationMinutes} minutes`);
  lines.push(`> Events: ${result.summary.totalEvents} | Commands: ${result.summary.totalCommands}`);
  lines.push(`> Struggle Score: ${result.summary.struggleScore}%`);
  lines.push('');

  // Topics as a database-like table
  if (result.summary.topTopics.length > 0) {
    lines.push('## 📝 Topics');
    lines.push('');
    lines.push('| Topic | Events | Time |');
    lines.push('| --- | --- | --- |');
    for (const topic of result.summary.topTopics) {
      lines.push(`| ${topic.topic} | ${topic.count} | ${topic.timeMinutes} min |`);
    }
    lines.push('');
  }

  // Content ideas as toggles
  if (result.ideas.length > 0) {
    lines.push('## 💡 Content Ideas');
    lines.push('');

    for (const idea of result.ideas) {
      const emoji = idea.confidence === 'high' ? '🟢' : idea.confidence === 'medium' ? '🟡' : '🔴';
      const formatEmoji =
        {
          video: '🎬',
          blog: '📝',
          thread: '🧵',
          newsletter: '📧',
        }[idea.suggestedFormat] || '📄';

      lines.push(`### ${emoji} ${formatEmoji} ${idea.title}`);
      lines.push('');
      lines.push(`> **Hook:** ${idea.hook}`);
      lines.push('');
      lines.push(`- **Angle:** ${idea.angle}`);
      lines.push(`- **Format:** ${idea.suggestedFormat}`);
      lines.push(`- **Confidence:** ${idea.confidence}`);
      lines.push('');
      lines.push('**Evidence:**');
      for (const e of idea.evidence) {
        lines.push(`- ${e}`);
      }
      lines.push('');
      lines.push('---');
      lines.push('');
    }
  }

  // Breakthroughs
  if (result.summary.ahaMonments.length > 0) {
    lines.push('## ⚡ Breakthroughs');
    lines.push('');
    for (const aha of result.summary.ahaMonments) {
      lines.push(`- 💡 ${aha.description} (${formatTime(aha.timestamp)})`);
    }
    lines.push('');
  }

  const content = lines.join('\n');

  if (options.outputPath) {
    const outputPath = options.outputPath.endsWith('.md')
      ? options.outputPath
      : `${options.outputPath}.md`;
    fs.writeFileSync(outputPath, content, 'utf-8');
    return outputPath;
  }

  return content;
}

/**
 * Format date as YYYY-MM-DD
 */
function formatDate(date: Date): string {
  return date.toISOString().split('T')[0];
}

/**
 * Format time as HH:MM
 */
function formatTime(date: Date): string {
  return date.toLocaleTimeString('en-US', {
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  });
}

/**
 * Generate Obsidian daily note entry
 */
export function generateObsidianDailyEntry(result: AnalysisResult): string {
  const lines: string[] = [];

  lines.push('## Siphon Activity');
  lines.push('');
  lines.push(
    `- Worked on: ${result.summary.topTopics.map((t) => `[[${t.topic}]]`).join(', ') || 'various topics'}`
  );
  lines.push(`- Duration: ${result.timeRange.durationMinutes} minutes`);
  lines.push(`- Struggle score: ${result.summary.struggleScore}%`);

  if (result.ideas.length > 0) {
    lines.push('');
    lines.push('### Content Ideas');
    for (const idea of result.ideas.slice(0, 3)) {
      lines.push(`- [ ] ${idea.title}`);
    }
  }

  return lines.join('\n');
}
