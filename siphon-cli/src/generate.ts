/**
 * Claude API Integration
 *
 * Generates polished content ideas using Claude API.
 * Requires ANTHROPIC_API_KEY environment variable.
 */

import Anthropic from '@anthropic-ai/sdk';
import type { AnalysisResult } from './types.js';

const SYSTEM_PROMPT = `You are a content strategist helping developers turn their work sessions into engaging content.
You analyze their development activity data and suggest video ideas, blog posts, and social media threads.

Your suggestions should be:
1. Authentic - based on real struggles and breakthroughs, not manufactured drama
2. Educational - provide genuine value to other developers
3. Engaging - have compelling hooks that make people want to learn more
4. Actionable - include specific takeaways and lessons

Focus on the "aha moments" and debugging journeys - these make the best content.
Be concise but specific in your suggestions.`;

export interface GeneratedContent {
  ideas: EnhancedIdea[];
  weeklyTheme?: string;
  suggestedSeries?: string;
}

export interface EnhancedIdea {
  title: string;
  hook: string;
  format: 'video' | 'blog' | 'thread' | 'newsletter';
  outline: string[];
  targetAudience: string;
  estimatedEngagement: 'high' | 'medium' | 'low';
  keyTakeaways: string[];
}

/**
 * Check if Claude API is available
 */
export function isClaudeAvailable(): boolean {
  return !!process.env.ANTHROPIC_API_KEY;
}

/**
 * Generate enhanced content ideas using Claude
 */
export async function generateWithClaude(result: AnalysisResult): Promise<GeneratedContent> {
  const apiKey = process.env.ANTHROPIC_API_KEY;
  if (!apiKey) {
    throw new Error(
      'ANTHROPIC_API_KEY environment variable is required.\n' +
        'Set it with: export ANTHROPIC_API_KEY=your-key-here'
    );
  }

  const client = new Anthropic({ apiKey });

  const userPrompt = buildPrompt(result);

  console.log('\nGenerating content ideas with Claude...\n');

  const response = await client.messages.create({
    model: 'claude-sonnet-4-20250514',
    max_tokens: 2048,
    messages: [
      {
        role: 'user',
        content: userPrompt,
      },
    ],
    system: SYSTEM_PROMPT,
  });

  // Extract text from response
  const textContent = response.content.find((block) => block.type === 'text');
  if (!textContent || textContent.type !== 'text') {
    throw new Error('Unexpected response format from Claude');
  }

  return parseClaudeResponse(textContent.text);
}

/**
 * Build the prompt for Claude
 */
function buildPrompt(result: AnalysisResult): string {
  const { summary, clusters, ideas, timeRange } = result;

  let prompt = `I just completed a ${timeRange.durationMinutes}-minute development session. Here's my activity data:

## Session Summary
- Total commands: ${summary.totalCommands} (${summary.failedCommands} failed)
- Struggle score: ${summary.struggleScore}% (higher = more debugging)
- Top topics: ${summary.topTopics.map((t) => `${t.topic} (${t.count} events, ${t.timeMinutes} min)`).join(', ')}

## Work Clusters
`;

  for (const cluster of clusters) {
    prompt += `
### ${cluster.topic} (${cluster.durationMinutes} min)
- Events: ${cluster.events.length}
- Struggle score: ${cluster.struggleScore}%
- Aha moment index: ${cluster.ahaIndex}%
- Confidence: ${cluster.confidence}
- Signals: ${cluster.signals.map((s) => s.description).join(', ') || 'none'}
`;
  }

  if (summary.ahaMonments.length > 0) {
    prompt += `
## Breakthrough Moments
${summary.ahaMonments.map((a) => `- ${a.description}`).join('\n')}
`;
  }

  if (ideas.length > 0) {
    prompt += `
## Initial Content Ideas (auto-detected)
${ideas.map((i, idx) => `${idx + 1}. ${i.title}\n   Hook: "${i.hook}"\n   Format: ${i.suggestedFormat}`).join('\n')}
`;
  }

  prompt += `
Based on this session data, please provide:

1. **3-5 Content Ideas** - For each idea include:
   - A catchy title
   - An attention-grabbing hook (first sentence)
   - Format recommendation (video/blog/thread/newsletter)
   - Brief outline (3-5 bullet points)
   - Target audience
   - Key takeaways (2-3)

2. **Weekly Theme** (optional) - If you see a pattern across my work, suggest a theme

3. **Series Potential** (optional) - If this could be part of a content series, describe it

Format your response as structured sections that are easy to parse.`;

  return prompt;
}

/**
 * Parse Claude's response into structured content
 */
function parseClaudeResponse(text: string): GeneratedContent {
  const ideas: EnhancedIdea[] = [];

  // Simple parsing - extract content ideas
  // Look for numbered ideas or sections with titles
  const ideaPattern =
    /(?:^|\n)(?:\d+\.\s*\*{0,2}|##?\s*)([^\n]+?)(?:\*{0,2})\n([\s\S]*?)(?=(?:\n(?:\d+\.\s*\*{0,2}|##?\s*)|$))/g;

  let match: RegExpExecArray | null = ideaPattern.exec(text);
  while (match !== null) {
    const title = match[1].trim();
    const content = match[2];

    // Skip if this is a meta-section like "Weekly Theme" or "Series Potential"
    if (title.toLowerCase().includes('weekly theme') || title.toLowerCase().includes('series')) {
      continue;
    }

    // Extract hook
    const hookMatch = content.match(/hook[:\s]*["']?([^"\n]+)["']?/i);
    const hook = hookMatch ? hookMatch[1].trim() : '';

    // Extract format
    let format: 'video' | 'blog' | 'thread' | 'newsletter' = 'video';
    if (content.toLowerCase().includes('blog')) format = 'blog';
    if (content.toLowerCase().includes('thread')) format = 'thread';
    if (content.toLowerCase().includes('newsletter')) format = 'newsletter';

    // Extract outline (bullet points)
    const outlineMatches = content.match(/[-*]\s+([^\n]+)/g) || [];
    const outline = outlineMatches.map((m) => m.replace(/^[-*]\s+/, '').trim());

    // Extract target audience
    const audienceMatch = content.match(/(?:target\s*)?audience[:\s]*([^\n]+)/i);
    const targetAudience = audienceMatch ? audienceMatch[1].trim() : 'Developers';

    // Extract takeaways
    const takeawayMatches = content.match(/takeaway[s]?[:\s]*\n?((?:[-*]\s+[^\n]+\n?)+)/i) || [];
    const keyTakeaways = takeawayMatches[1]
      ? takeawayMatches[1]
          .split('\n')
          .filter((l) => l.trim())
          .map((l) => l.replace(/^[-*]\s+/, '').trim())
      : [];

    if (title && title.length > 5) {
      ideas.push({
        title,
        hook: hook || `Learn how I tackled ${title.toLowerCase()}`,
        format,
        outline: outline.slice(0, 5),
        targetAudience,
        estimatedEngagement: outline.length > 3 ? 'high' : 'medium',
        keyTakeaways: keyTakeaways.slice(0, 3),
      });
    }
    match = ideaPattern.exec(text);
  }

  // Extract weekly theme
  const themeMatch = text.match(/weekly\s*theme[:\s]*\n?([^\n]+(?:\n(?![#\d])[^\n]+)*)/i);
  const weeklyTheme = themeMatch ? themeMatch[1].trim() : undefined;

  // Extract series suggestion
  const seriesMatch = text.match(/series\s*(?:potential)?[:\s]*\n?([^\n]+(?:\n(?![#\d])[^\n]+)*)/i);
  const suggestedSeries = seriesMatch ? seriesMatch[1].trim() : undefined;

  return {
    ideas: ideas.slice(0, 5),
    weeklyTheme,
    suggestedSeries,
  };
}

/**
 * Display generated content to console
 */
export function displayGeneratedContent(content: GeneratedContent): void {
  console.log('='.repeat(60));
  console.log('CLAUDE-GENERATED CONTENT IDEAS');
  console.log('='.repeat(60));
  console.log();

  for (let i = 0; i < content.ideas.length; i++) {
    const idea = content.ideas[i];
    console.log(`${i + 1}. ${idea.title}`);
    console.log(`   Format: ${idea.format}`);
    console.log(`   Hook: "${idea.hook}"`);
    console.log(`   Audience: ${idea.targetAudience}`);

    if (idea.outline.length > 0) {
      console.log('   Outline:');
      for (const point of idea.outline) {
        console.log(`     - ${point}`);
      }
    }

    if (idea.keyTakeaways.length > 0) {
      console.log('   Key Takeaways:');
      for (const takeaway of idea.keyTakeaways) {
        console.log(`     - ${takeaway}`);
      }
    }

    console.log();
  }

  if (content.weeklyTheme) {
    console.log('-'.repeat(40));
    console.log('WEEKLY THEME SUGGESTION');
    console.log('-'.repeat(40));
    console.log(content.weeklyTheme);
    console.log();
  }

  if (content.suggestedSeries) {
    console.log('-'.repeat(40));
    console.log('SERIES POTENTIAL');
    console.log('-'.repeat(40));
    console.log(content.suggestedSeries);
    console.log();
  }
}
