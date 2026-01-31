/**
 * Content Templates Library
 *
 * Pre-defined templates for different content formats
 * that can be filled with analysis data.
 */

import type { AnalysisResult, Cluster, ContentIdea } from './types.js';

export type ContentFormat = 'twitter_thread' | 'blog_post' | 'video_script' | 'newsletter' | 'linkedin_post' | 'tutorial';

export interface ContentTemplate {
  format: ContentFormat;
  name: string;
  description: string;
  generate: (result: AnalysisResult, cluster?: Cluster) => string;
}

/**
 * Get all available templates
 */
export function getTemplates(): ContentTemplate[] {
  return [
    twitterThreadTemplate,
    blogPostTemplate,
    videoScriptTemplate,
    newsletterTemplate,
    linkedInTemplate,
    tutorialTemplate,
    debuggingStoryTemplate,
    beforeAfterTemplate,
    tilTemplate,
  ];
}

/**
 * Get template by format
 */
export function getTemplate(format: ContentFormat): ContentTemplate | undefined {
  return getTemplates().find(t => t.format === format);
}

/**
 * Generate content from template
 */
export function generateFromTemplate(
  format: ContentFormat,
  result: AnalysisResult,
  cluster?: Cluster
): string {
  const template = getTemplate(format);
  if (!template) {
    throw new Error(`Unknown template format: ${format}`);
  }
  return template.generate(result, cluster);
}

// ============================================================================
// Template Definitions
// ============================================================================

const twitterThreadTemplate: ContentTemplate = {
  format: 'twitter_thread',
  name: 'Twitter/X Thread',
  description: 'A multi-tweet thread format for sharing insights',
  generate: (result, cluster) => {
    const topic = cluster?.topic || result.summary.topTopics[0]?.topic || 'development';
    const struggle = result.summary.struggleScore;
    const duration = result.timeRange.durationMinutes;
    const idea = result.ideas[0];

    return `🧵 THREAD: ${idea?.title || `What I learned about ${topic} today`}

1/ Just spent ${duration} minutes working on ${topic}.

${struggle > 50 ? `It was a struggle (${struggle}% difficulty), but I figured it out.` : `Here's what I discovered:`}

---

2/ The problem:
[Describe the specific problem you were trying to solve]

---

3/ What I tried first:
[Your initial approach that didn't work]

---

4/ The breakthrough:
[What actually worked and why]

---

5/ Key takeaway:
${idea?.hook || '[Share the main lesson]'}

---

6/ If you're facing something similar:
- [Tip 1]
- [Tip 2]
- [Tip 3]

---

7/ Resources that helped:
- [Link 1]
- [Link 2]

---

8/ Follow for more ${topic} tips!

Like & RT if this helped 🙏

#${topic.replace(/\s+/g, '')} #coding #webdev #programming`;
  }
};

const blogPostTemplate: ContentTemplate = {
  format: 'blog_post',
  name: 'Blog Post Outline',
  description: 'A structured blog post outline with SEO considerations',
  generate: (result, cluster) => {
    const topic = cluster?.topic || result.summary.topTopics[0]?.topic || 'development';
    const idea = result.ideas[0];
    const duration = result.timeRange.durationMinutes;
    const struggle = result.summary.struggleScore;

    return `# ${idea?.title || `A Developer's Guide to ${topic}`}

## Meta
- **Target keyword:** ${topic}
- **Word count target:** 1500-2000
- **Reading time:** 7-10 minutes

---

## Hook / Introduction (150-200 words)

${idea?.hook || `Start with a relatable problem about ${topic}...`}

- What problem does this solve?
- Why should the reader care?
- What will they learn?

---

## The Problem (200-300 words)

Describe the challenge in detail:
- Context: When does this problem occur?
- Pain points: What makes it frustrating?
- Why existing solutions fall short

---

## My Journey (300-400 words)

Share your experience (${duration} minutes of work, ${struggle}% struggle score):

### What I Tried First
- Approach 1: [description]
- Approach 2: [description]
- Why they didn't work

### The Breakthrough Moment
- What clicked
- The "aha" insight

---

## The Solution (400-500 words)

### Step 1: [Setup/Preparation]
\`\`\`
// Code example
\`\`\`

### Step 2: [Core Implementation]
\`\`\`
// Code example
\`\`\`

### Step 3: [Polish/Edge Cases]
\`\`\`
// Code example
\`\`\`

---

## Key Takeaways (150-200 words)

1. **[Lesson 1]:** Brief explanation
2. **[Lesson 2]:** Brief explanation
3. **[Lesson 3]:** Brief explanation

---

## Common Pitfalls to Avoid

- ❌ [Mistake 1]
- ❌ [Mistake 2]
- ❌ [Mistake 3]

---

## Conclusion (100-150 words)

- Summarize the solution
- Encourage the reader
- Call to action (comments, newsletter, etc.)

---

## Resources

- [Official Documentation]
- [Related Article]
- [Useful Tool]

---

**Tags:** ${topic}, tutorial, ${result.summary.topTopics.slice(1, 4).map(t => t.topic).join(', ')}`;
  }
};

const videoScriptTemplate: ContentTemplate = {
  format: 'video_script',
  name: 'Video Script',
  description: 'A script outline for YouTube or tutorial videos',
  generate: (result, cluster) => {
    const topic = cluster?.topic || result.summary.topTopics[0]?.topic || 'development';
    const idea = result.ideas[0];
    const duration = Math.ceil(result.timeRange.durationMinutes / 10);

    return `# VIDEO SCRIPT: ${idea?.title || `Mastering ${topic}`}

## Video Details
- **Length:** ${duration * 2}-${duration * 3} minutes
- **Style:** Tutorial / Walkthrough
- **Thumbnail idea:** [Before/After code or frustrated → happy dev]

---

## HOOK (0:00 - 0:30)
*[On camera, energetic]*

"${idea?.hook || `Have you ever struggled with ${topic}? In the next few minutes, I'll show you exactly how to solve it.`}"

**B-roll suggestion:** Quick montage of error messages / frustration

---

## INTRO (0:30 - 1:00)
*[On camera]*

"Hey everyone, [name] here. Today we're diving into ${topic}."

"By the end of this video, you'll understand:
- [Key concept 1]
- [Key concept 2]
- [Key concept 3]"

**CTA:** "If you find this helpful, smash that like button and subscribe!"

---

## THE PROBLEM (1:00 - 2:00)
*[Screen recording]*

"Let me show you the problem..."

\`\`\`
// Show the problematic code or situation
\`\`\`

"As you can see, [explain what's wrong]"

---

## SOLUTION PART 1 (2:00 - 4:00)
*[Screen recording with voiceover]*

"First, let's [initial setup step]..."

\`\`\`
// Code walkthrough
\`\`\`

**Key points to emphasize:**
- [Point 1]
- [Point 2]

---

## SOLUTION PART 2 (4:00 - 6:00)
*[Screen recording with voiceover]*

"Now for the important part..."

\`\`\`
// Core solution code
\`\`\`

**Pro tip callout:** "[Helpful tip]"

---

## SOLUTION PART 3 (6:00 - 8:00)
*[Screen recording with voiceover]*

"Let's add some polish..."

\`\`\`
// Final touches
\`\`\`

---

## DEMO (8:00 - 9:00)
*[Screen recording]*

"Let's see it in action!"

*[Show the working solution]*

---

## RECAP (9:00 - 9:30)
*[On camera]*

"So to recap:
1. [Key takeaway 1]
2. [Key takeaway 2]
3. [Key takeaway 3]"

---

## OUTRO (9:30 - 10:00)
*[On camera]*

"That's it for today! If you learned something new, leave a comment below."

"Check out [related video] next."

"See you in the next one!"

---

## CHAPTER MARKERS
- 0:00 Intro
- 0:30 The Problem
- 2:00 Solution Part 1
- 4:00 Solution Part 2
- 6:00 Solution Part 3
- 8:00 Demo
- 9:00 Recap

## DESCRIPTION TEMPLATE
${idea?.title || topic} | Full Tutorial

In this video, I show you how to [brief description].

📚 Resources:
- [Link 1]
- [Link 2]

⏱️ Timestamps:
[paste chapter markers]

#${topic.replace(/\s+/g, '')} #programming #tutorial`;
  }
};

const newsletterTemplate: ContentTemplate = {
  format: 'newsletter',
  name: 'Newsletter Edition',
  description: 'A weekly newsletter format with insights and resources',
  generate: (result) => {
    const topics = result.summary.topTopics.slice(0, 3);
    const ideas = result.ideas.slice(0, 3);
    const date = new Date().toLocaleDateString('en-US', {
      weekday: 'long',
      year: 'numeric',
      month: 'long',
      day: 'numeric'
    });

    return `# 📬 Dev Insights Weekly

*${date}*

---

Hey friend,

This week I spent ${result.timeRange.durationMinutes} minutes deep in code, and here's what I learned.

---

## 🎯 This Week's Focus

${topics.map(t => `**${t.topic}** - ${t.count} activities over ${t.timeMinutes} minutes`).join('\n')}

---

## 💡 Key Insights

${ideas.map((idea, i) => `### ${i + 1}. ${idea.title}

${idea.hook}

**Format:** ${idea.suggestedFormat} | **Confidence:** ${idea.confidence}

`).join('\n')}

---

## 🔥 Struggle of the Week

Difficulty score: **${result.summary.struggleScore}%**

${result.summary.struggleScore > 50
  ? "It was a tough week! But struggle leads to growth. Here's what made it challenging..."
  : "Relatively smooth sailing this week. Here's what went well..."}

[Share your biggest challenge and how you overcame it]

---

## 🛠️ Tool/Resource of the Week

**[Tool Name]**

[Brief description of why it's useful]

[Link]

---

## 📖 What I'm Reading

- [Article 1] - [One-line takeaway]
- [Article 2] - [One-line takeaway]

---

## 🎬 What's Coming

Next week I'm planning to dive into:
- [Topic 1]
- [Topic 2]

---

That's all for this week!

Hit reply and let me know what you're working on.

Happy coding,
[Your name]

---

*Was this forwarded to you? [Subscribe here](#)*

*Too many emails? [Manage preferences](#)*`;
  }
};

const linkedInTemplate: ContentTemplate = {
  format: 'linkedin_post',
  name: 'LinkedIn Post',
  description: 'Professional social post format for LinkedIn',
  generate: (result, cluster) => {
    const topic = cluster?.topic || result.summary.topTopics[0]?.topic || 'development';
    const idea = result.ideas[0];

    return `${idea?.hook || `I just learned something valuable about ${topic}.`}

Here's what happened:

I was working on ${topic} when I ran into a wall.
${result.summary.struggleScore > 50 ? '(Struggle score: ' + result.summary.struggleScore + '% - it was rough!)' : ''}

After ${result.timeRange.durationMinutes} minutes of debugging, I discovered:

${idea?.angle || '[Your key insight]'}

3 lessons from this experience:

1️⃣ [Lesson 1]

2️⃣ [Lesson 2]

3️⃣ [Lesson 3]

The takeaway?
${idea?.evidence[0] || '[Your main conclusion]'}

---

What's the last technical challenge that taught you something unexpected?

#${topic.replace(/\s+/g, '')} #SoftwareEngineering #Programming #CodingLife #TechTips`;
  }
};

const tutorialTemplate: ContentTemplate = {
  format: 'tutorial',
  name: 'Step-by-Step Tutorial',
  description: 'A detailed tutorial format with code examples',
  generate: (result, cluster) => {
    const topic = cluster?.topic || result.summary.topTopics[0]?.topic || 'development';

    return `# Tutorial: ${topic}

## Prerequisites

Before starting, make sure you have:
- [ ] [Requirement 1]
- [ ] [Requirement 2]
- [ ] [Requirement 3]

## What You'll Learn

By the end of this tutorial, you'll be able to:
1. [Outcome 1]
2. [Outcome 2]
3. [Outcome 3]

---

## Step 1: Setup

First, let's set up our environment.

\`\`\`bash
# Installation commands
\`\`\`

**Expected output:**
\`\`\`
[What you should see]
\`\`\`

---

## Step 2: Basic Implementation

Now let's create the foundation.

\`\`\`javascript
// Code for step 2
\`\`\`

**Explanation:**
- Line 1-3: [Explain what this does]
- Line 4-6: [Explain what this does]

---

## Step 3: Adding Features

Let's enhance our implementation.

\`\`\`javascript
// Code for step 3
\`\`\`

**💡 Pro Tip:** [Helpful tip]

---

## Step 4: Testing

Let's make sure everything works.

\`\`\`javascript
// Test code
\`\`\`

**Expected result:**
\`\`\`
[Expected output]
\`\`\`

---

## Step 5: Production Ready

Final polish for production use.

\`\`\`javascript
// Production code
\`\`\`

---

## Troubleshooting

### Common Error 1
**Problem:** [Description]
**Solution:** [Fix]

### Common Error 2
**Problem:** [Description]
**Solution:** [Fix]

---

## Summary

✅ You learned how to [outcome 1]
✅ You implemented [outcome 2]
✅ You can now [outcome 3]

## Next Steps

- [Advanced topic 1]
- [Related tutorial]
- [Documentation link]

---

*Questions? Leave a comment below!*`;
  }
};

// Additional specialized templates

const debuggingStoryTemplate: ContentTemplate = {
  format: 'blog_post',
  name: 'Debugging Story',
  description: 'Narrative-style post about solving a tricky bug',
  generate: (result, cluster) => {
    const topic = cluster?.topic || result.summary.topTopics[0]?.topic || 'development';
    const struggle = result.summary.struggleScore;

    return `# The ${topic} Bug That Took Me ${result.timeRange.durationMinutes} Minutes

*A debugging story with a ${struggle > 50 ? 'frustrating' : 'satisfying'} ending*

---

## The Setup

It started innocently enough. I was working on [project] when...

---

## The Symptom

\`\`\`
// The error message or unexpected behavior
\`\`\`

---

## What I Tried (And Why It Didn't Work)

### Attempt 1: [The obvious fix]
*Result: Still broken*

### Attempt 2: [The Stack Overflow solution]
*Result: Different error*

### Attempt 3: [The "it worked before" revert]
*Result: Even more confused*

---

## The Breakthrough

After ${result.timeRange.durationMinutes} minutes, I finally realized...

\`\`\`
// The actual problem
\`\`\`

---

## The Fix

\`\`\`
// The solution
\`\`\`

---

## Lessons Learned

1. **[Lesson 1]**
2. **[Lesson 2]**
3. **[Lesson 3]**

---

## How to Avoid This

- [Prevention tip 1]
- [Prevention tip 2]

---

*Have you encountered something similar? Share your debugging war stories in the comments!*`;
  }
};

const beforeAfterTemplate: ContentTemplate = {
  format: 'twitter_thread',
  name: 'Before/After Code',
  description: 'Show code transformation with before and after examples',
  generate: (result, cluster) => {
    const topic = cluster?.topic || result.summary.topTopics[0]?.topic || 'development';

    return `🔄 ${topic.toUpperCase()} BEFORE vs AFTER

A thread on writing better code 🧵

---

❌ BEFORE:

\`\`\`
// Bad code example
// - Problem 1
// - Problem 2
// - Problem 3
\`\`\`

---

✅ AFTER:

\`\`\`
// Improved code
// - Fix 1
// - Fix 2
// - Fix 3
\`\`\`

---

What changed?

1. [Improvement 1]
2. [Improvement 2]
3. [Improvement 3]

---

Why it matters:
- Better readability
- Improved performance
- Easier maintenance

---

Save this for later! 🔖

#${topic.replace(/\s+/g, '')} #CleanCode #CodeReview`;
  }
};

const tilTemplate: ContentTemplate = {
  format: 'twitter_thread',
  name: 'TIL (Today I Learned)',
  description: 'Quick learning share format',
  generate: (result, cluster) => {
    const topic = cluster?.topic || result.summary.topTopics[0]?.topic || 'development';
    const idea = result.ideas[0];

    return `TIL: ${idea?.title || `Something cool about ${topic}`} 🤯

${idea?.hook || '[Share your discovery]'}

\`\`\`
// Quick code example if applicable
\`\`\`

Why this matters:
→ [Benefit 1]
→ [Benefit 2]
→ [Benefit 3]

Source: [Where you learned this]

#TIL #${topic.replace(/\s+/g, '')} #DevTips`;
  }
};
