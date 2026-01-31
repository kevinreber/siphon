/**
 * Event Analyzer
 *
 * Clusters events by topic, detects learning signals,
 * calculates struggle scores, and identifies "aha moments".
 */

import {
  Event,
  Cluster,
  LearningSignal,
  ContentIdea,
  AnalysisResult,
  ShellEventData,
  Session,
} from "./types.js";

// Topic detection keywords
const TOPIC_KEYWORDS: Record<string, string[]> = {
  kubernetes: ["kubectl", "k8s", "helm", "pod", "deployment", "service", "ingress"],
  docker: ["docker", "container", "dockerfile", "compose"],
  git: ["git", "commit", "push", "pull", "merge", "rebase", "branch"],
  node: ["npm", "node", "yarn", "pnpm", "package.json"],
  python: ["python", "pip", "venv", "pytest", "poetry"],
  rust: ["cargo", "rustc", "rustup"],
  database: ["psql", "mysql", "redis", "mongo", "sqlite"],
  aws: ["aws", "s3", "ec2", "lambda", "cloudformation"],
  testing: ["test", "jest", "pytest", "mocha", "cypress"],
  debugging: ["debug", "log", "error", "trace", "stack"],
};

/**
 * Main analyzer class
 */
export class Analyzer {
  /**
   * Analyze a collection of events
   */
  analyze(events: Event[], timeWindow: { start: Date; end: Date }): AnalysisResult {
    // Sort events by timestamp
    const sortedEvents = [...events].sort(
      (a, b) => a.timestamp.getTime() - b.timestamp.getTime()
    );

    // Cluster events
    const clusters = this.clusterEvents(sortedEvents);

    // Detect learning signals in each cluster
    for (const cluster of clusters) {
      cluster.signals = this.detectLearningSignals(cluster.events);
      cluster.struggleScore = this.calculateStruggleScore(cluster.events);
      cluster.ahaIndex = this.detectAhaMoments(cluster.events);
    }

    // Detect sessions (groups of clusters separated by long gaps)
    const sessions = this.detectSessions(sortedEvents, clusters);

    // Generate content ideas
    const ideas = this.generateContentIdeas(clusters);

    // Calculate summary
    const summary = this.calculateSummary(sortedEvents, clusters, sessions);

    return {
      timeRange: {
        start: timeWindow.start,
        end: timeWindow.end,
        durationMinutes: Math.round(
          (timeWindow.end.getTime() - timeWindow.start.getTime()) / 60000
        ),
      },
      events: sortedEvents,
      clusters,
      sessions,
      ideas,
      summary,
    };
  }

  /**
   * Cluster events by topic and temporal proximity
   */
  private clusterEvents(events: Event[]): Cluster[] {
    const clusters: Cluster[] = [];
    const CLUSTER_GAP_MS = 30 * 60 * 1000; // 30 minutes

    let currentCluster: Event[] = [];
    let currentTopic = "";

    for (const event of events) {
      const eventTopic = this.detectTopic(event);

      // Check if we should start a new cluster
      const lastEvent = currentCluster[currentCluster.length - 1];
      const timeSinceLastEvent = lastEvent
        ? event.timestamp.getTime() - lastEvent.timestamp.getTime()
        : 0;

      const shouldStartNew =
        currentCluster.length === 0 ||
        timeSinceLastEvent > CLUSTER_GAP_MS ||
        (eventTopic !== currentTopic && eventTopic !== "general");

      if (shouldStartNew && currentCluster.length > 0) {
        // Save current cluster
        clusters.push(this.createCluster(currentCluster, currentTopic));
        currentCluster = [];
      }

      currentCluster.push(event);
      if (eventTopic !== "general") {
        currentTopic = eventTopic;
      }
    }

    // Save final cluster
    if (currentCluster.length > 0) {
      clusters.push(this.createCluster(currentCluster, currentTopic));
    }

    return clusters;
  }

  /**
   * Detect work sessions based on gaps in activity
   * A session is a continuous period of work separated by gaps of 2+ hours
   */
  private detectSessions(events: Event[], clusters: Cluster[]): Session[] {
    if (events.length === 0) return [];

    const SESSION_GAP_MS = 2 * 60 * 60 * 1000; // 2 hours
    const sessions: Session[] = [];

    let currentSessionEvents: Event[] = [];
    let sessionStart: Date | null = null;
    let lastEventTime: Date | null = null;

    for (const event of events) {
      const gapMs = lastEventTime
        ? event.timestamp.getTime() - lastEventTime.getTime()
        : 0;

      if (gapMs > SESSION_GAP_MS && currentSessionEvents.length > 0) {
        // End current session and start a new one
        const session = this.createSession(
          currentSessionEvents,
          clusters,
          sessionStart!,
          lastEventTime!,
          sessions.length > 0 ? Math.round(gapMs / 60000) : undefined
        );
        sessions.push(session);
        currentSessionEvents = [];
        sessionStart = null;
      }

      if (sessionStart === null) {
        sessionStart = event.timestamp;
      }
      currentSessionEvents.push(event);
      lastEventTime = event.timestamp;
    }

    // Save final session
    if (currentSessionEvents.length > 0 && sessionStart && lastEventTime) {
      sessions.push(
        this.createSession(
          currentSessionEvents,
          clusters,
          sessionStart,
          lastEventTime,
          undefined
        )
      );
    }

    return sessions;
  }

  /**
   * Create a session from events
   */
  private createSession(
    events: Event[],
    allClusters: Cluster[],
    startTime: Date,
    endTime: Date,
    gapBeforeMinutes?: number
  ): Session {
    const durationMinutes = Math.round(
      (endTime.getTime() - startTime.getTime()) / 60000
    );

    // Find clusters that overlap with this session
    const sessionClusters = allClusters.filter(
      (c) => c.startTime >= startTime && c.endTime <= endTime
    );

    // Generate a description based on the clusters
    const topTopics = this.getTopTopicsFromClusters(sessionClusters, 3);
    const description =
      topTopics.length > 0
        ? `Working on ${topTopics.join(", ")}`
        : "Development session";

    return {
      id: `session-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
      startTime,
      endTime,
      durationMinutes,
      events,
      clusters: sessionClusters,
      gapBeforeMinutes,
      description,
    };
  }

  /**
   * Get top topics from clusters
   */
  private getTopTopicsFromClusters(clusters: Cluster[], limit: number): string[] {
    const topicCounts = new Map<string, number>();
    for (const cluster of clusters) {
      if (cluster.topic !== "general") {
        topicCounts.set(
          cluster.topic,
          (topicCounts.get(cluster.topic) || 0) + cluster.events.length
        );
      }
    }

    return [...topicCounts.entries()]
      .sort((a, b) => b[1] - a[1])
      .slice(0, limit)
      .map(([topic]) => topic);
  }

  /**
   * Create a cluster from a list of events
   */
  private createCluster(events: Event[], topic: string): Cluster {
    const startTime = events[0].timestamp;
    const endTime = events[events.length - 1].timestamp;
    const durationMinutes = Math.round(
      (endTime.getTime() - startTime.getTime()) / 60000
    );

    // Determine confidence based on event count and duration
    let confidence: "high" | "medium" | "low" = "low";
    if (events.length >= 10 && durationMinutes >= 60) {
      confidence = "high";
    } else if (events.length >= 5) {
      confidence = "medium";
    }

    return {
      id: `cluster-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
      topic: topic || "general",
      events,
      startTime,
      endTime,
      durationMinutes,
      confidence,
      struggleScore: 0,
      ahaIndex: 0,
      signals: [],
    };
  }

  /**
   * Detect topic from an event
   */
  private detectTopic(event: Event): string {
    if (event.source === "shell") {
      const data = event.data as ShellEventData;
      const command = data.command.toLowerCase();

      for (const [topic, keywords] of Object.entries(TOPIC_KEYWORDS)) {
        if (keywords.some((kw) => command.includes(kw))) {
          return topic;
        }
      }

      // Extract first word as topic fallback
      const firstWord = data.command.split(/\s+/)[0];
      if (firstWord && firstWord.length > 1) {
        return firstWord;
      }
    }

    return "general";
  }

  /**
   * Detect learning signals in a cluster
   */
  private detectLearningSignals(events: Event[]): LearningSignal[] {
    const signals: LearningSignal[] = [];

    // Count failed commands
    const shellEvents = events.filter((e) => e.source === "shell");
    const failedCommands = shellEvents.filter(
      (e) => (e.data as ShellEventData).exitCode !== 0
    );

    if (failedCommands.length > 3) {
      signals.push({
        type: "debugging",
        description: `${failedCommands.length} failed commands indicate troubleshooting`,
        intensity: Math.min(failedCommands.length * 10, 100),
      });
    }

    // Detect repeated commands (trial and error)
    const commandCounts = new Map<string, number>();
    for (const event of shellEvents) {
      const cmd = (event.data as ShellEventData).command;
      // Normalize command (remove arguments that look like variable)
      const normalized = cmd.split(/\s+/).slice(0, 2).join(" ");
      commandCounts.set(normalized, (commandCounts.get(normalized) || 0) + 1);
    }

    const repeatedCommands = [...commandCounts.entries()].filter(
      ([, count]) => count >= 3
    );
    if (repeatedCommands.length > 0) {
      signals.push({
        type: "exploration",
        description: `Repeated commands suggest iterative exploration`,
        intensity: Math.min(repeatedCommands.length * 20, 100),
      });
    }

    return signals;
  }

  /**
   * Calculate struggle score (0-100)
   *
   * Higher score = more debugging/troubleshooting = better content potential
   */
  private calculateStruggleScore(events: Event[]): number {
    const shellEvents = events.filter((e) => e.source === "shell");
    if (shellEvents.length === 0) return 0;

    const failedCommands = shellEvents.filter(
      (e) => (e.data as ShellEventData).exitCode !== 0
    );

    // Factor 1: Failure rate (0-40 points)
    const failureRate = failedCommands.length / shellEvents.length;
    const failureScore = Math.round(failureRate * 40);

    // Factor 2: Retry patterns (0-30 points)
    const commandCounts = new Map<string, number>();
    for (const event of shellEvents) {
      const cmd = (event.data as ShellEventData).command.split(/\s+/)[0];
      commandCounts.set(cmd, (commandCounts.get(cmd) || 0) + 1);
    }
    const maxRetries = Math.max(...commandCounts.values());
    const retryScore = Math.min(maxRetries * 5, 30);

    // Factor 3: Time spent (0-30 points)
    const totalDuration = shellEvents.reduce(
      (sum, e) => sum + (e.data as ShellEventData).durationMs,
      0
    );
    const avgDuration = totalDuration / shellEvents.length;
    const durationScore = Math.min(Math.round(avgDuration / 1000), 30);

    return Math.min(failureScore + retryScore + durationScore, 100);
  }

  /**
   * Detect "aha moments" - breakthroughs after failures
   *
   * Returns an index (0-100) indicating how strong the breakthrough was
   */
  private detectAhaMoments(events: Event[]): number {
    const shellEvents = events.filter((e) => e.source === "shell");
    if (shellEvents.length < 3) return 0;

    let maxAhaIntensity = 0;

    // Look for sequences of failures followed by success
    let failStreak = 0;
    for (const event of shellEvents) {
      const exitCode = (event.data as ShellEventData).exitCode;

      if (exitCode !== 0) {
        failStreak++;
      } else if (failStreak >= 2) {
        // Success after failures = aha moment!
        const intensity = Math.min(failStreak * 15, 100);
        maxAhaIntensity = Math.max(maxAhaIntensity, intensity);
        failStreak = 0;
      } else {
        failStreak = 0;
      }
    }

    return maxAhaIntensity;
  }

  /**
   * Generate content ideas from clusters
   */
  private generateContentIdeas(clusters: Cluster[]): ContentIdea[] {
    const ideas: ContentIdea[] = [];

    for (const cluster of clusters) {
      // Skip low-confidence clusters
      if (cluster.confidence === "low" && cluster.struggleScore < 30) {
        continue;
      }

      // High struggle score = debugging story
      if (cluster.struggleScore >= 50) {
        ideas.push({
          title: `Debugging ${cluster.topic}: A Developer's Journey`,
          hook: `I spent ${cluster.durationMinutes} minutes debugging ${cluster.topic}. Here's what I learned.`,
          angle: "Troubleshooting narrative with lessons learned",
          confidence: cluster.confidence,
          evidence: [
            `${cluster.events.length} events over ${cluster.durationMinutes} minutes`,
            `Struggle score: ${cluster.struggleScore}%`,
            cluster.signals.map((s) => s.description).join(", "),
          ],
          suggestedFormat: "video",
        });
      }

      // High aha index = breakthrough story
      if (cluster.ahaIndex >= 40) {
        ideas.push({
          title: `The ${cluster.topic} Bug That Took Me Hours (And the Simple Fix)`,
          hook: `After multiple failed attempts, I finally figured out the solution. Here's the journey.`,
          angle: "Problem-solution narrative with the breakthrough moment",
          confidence: cluster.confidence,
          evidence: [
            `Aha moment intensity: ${cluster.ahaIndex}%`,
            `Topic: ${cluster.topic}`,
          ],
          suggestedFormat: "blog",
        });
      }

      // Long focused session = deep dive
      if (cluster.durationMinutes >= 60 && cluster.events.length >= 15) {
        ideas.push({
          title: `Deep Dive: ${cluster.topic}`,
          hook: `A comprehensive exploration of ${cluster.topic} from my recent work session.`,
          angle: "Educational deep dive based on real work",
          confidence: cluster.confidence,
          evidence: [
            `${cluster.durationMinutes} minute focused session`,
            `${cluster.events.length} events tracked`,
          ],
          suggestedFormat: "video",
        });
      }
    }

    // Sort by confidence
    ideas.sort((a, b) => {
      const order = { high: 0, medium: 1, low: 2 };
      return order[a.confidence] - order[b.confidence];
    });

    return ideas;
  }

  /**
   * Calculate summary statistics
   */
  private calculateSummary(events: Event[], clusters: Cluster[], sessions: Session[]) {
    const shellEvents = events.filter((e) => e.source === "shell");
    const failedCommands = shellEvents.filter(
      (e) => (e.data as ShellEventData).exitCode !== 0
    );

    // Calculate overall struggle score
    const struggleScore =
      shellEvents.length > 0
        ? Math.round((failedCommands.length / shellEvents.length) * 100)
        : 0;

    // Top topics
    const topicCounts = new Map<string, { count: number; timeMs: number }>();
    for (const cluster of clusters) {
      const existing = topicCounts.get(cluster.topic) || { count: 0, timeMs: 0 };
      existing.count += cluster.events.length;
      existing.timeMs += cluster.durationMinutes * 60000;
      topicCounts.set(cluster.topic, existing);
    }

    const topTopics = [...topicCounts.entries()]
      .map(([topic, stats]) => ({
        topic,
        count: stats.count,
        timeMinutes: Math.round(stats.timeMs / 60000),
      }))
      .sort((a, b) => b.count - a.count)
      .slice(0, 5);

    // Aha moments
    const ahaMonments = clusters
      .filter((c) => c.ahaIndex >= 30)
      .map((c) => ({
        description: `Breakthrough in ${c.topic}`,
        timestamp: c.endTime,
      }));

    // Session statistics
    const sessionCount = sessions.length;
    const totalSessionMinutes = sessions.reduce(
      (sum, s) => sum + s.durationMinutes,
      0
    );
    const averageSessionMinutes =
      sessionCount > 0 ? Math.round(totalSessionMinutes / sessionCount) : 0;

    return {
      totalEvents: events.length,
      totalCommands: shellEvents.length,
      failedCommands: failedCommands.length,
      struggleScore,
      topTopics,
      ahaMonments,
      sessionCount,
      averageSessionMinutes,
    };
  }
}
