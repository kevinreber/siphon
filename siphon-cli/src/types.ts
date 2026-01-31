/**
 * Core types for Siphon CLI
 */

export interface Event {
  id: string;
  timestamp: Date;
  source: EventSource;
  eventType: string;
  data: ShellEventData | EditorEventData | GitEventData;
  project?: string;
}

export type EventSource = "shell" | "editor" | "filesystem" | "git" | "browser";

export interface ShellEventData {
  command: string;
  exitCode: number;
  durationMs: number;
  cwd: string;
  gitBranch?: string;
}

export interface EditorEventData {
  action: string;
  filePath: string;
  language?: string;
  linesChanged?: number;
}

export interface GitEventData {
  action: string;
  branch?: string;
  message?: string;
  filesChanged?: number;
}

export interface Cluster {
  id: string;
  topic: string;
  events: Event[];
  startTime: Date;
  endTime: Date;
  durationMinutes: number;
  confidence: "high" | "medium" | "low";
  struggleScore: number;
  ahaIndex: number;
  signals: LearningSignal[];
}

export interface LearningSignal {
  type: "research" | "troubleshooting" | "debugging" | "exploration" | "breakthrough";
  description: string;
  intensity: number; // 0-100
}

export interface ContentIdea {
  title: string;
  hook: string;
  angle: string;
  confidence: "high" | "medium" | "low";
  evidence: string[];
  suggestedFormat: "video" | "blog" | "thread" | "newsletter";
}

export interface AnalysisResult {
  timeRange: {
    start: Date;
    end: Date;
    durationMinutes: number;
  };
  events: Event[];
  clusters: Cluster[];
  ideas: ContentIdea[];
  summary: {
    totalEvents: number;
    totalCommands: number;
    failedCommands: number;
    struggleScore: number;
    topTopics: Array<{ topic: string; count: number; timeMinutes: number }>;
    ahaMonments: Array<{ description: string; timestamp: Date }>;
  };
}
