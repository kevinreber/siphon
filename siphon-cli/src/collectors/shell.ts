/**
 * Shell History Collector
 *
 * Reads shell history from ~/.zsh_history or ~/.bash_history
 * and converts to Siphon events.
 */

import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';
import type { Event, ShellEventData } from '../types.js';

export class ShellHistoryCollector {
  private historyPath: string;
  private shell: 'zsh' | 'bash';

  constructor() {
    // Detect shell and history file
    const zshHistory = path.join(os.homedir(), '.zsh_history');
    const bashHistory = path.join(os.homedir(), '.bash_history');

    if (fs.existsSync(zshHistory)) {
      this.historyPath = zshHistory;
      this.shell = 'zsh';
    } else if (fs.existsSync(bashHistory)) {
      this.historyPath = bashHistory;
      this.shell = 'bash';
    } else {
      this.historyPath = '';
      this.shell = 'bash';
    }
  }

  /**
   * Collect shell events within a time range
   */
  // biome-ignore lint/complexity/noExcessiveCognitiveComplexity: Handles multiple shell history formats
  async collect(startTime: Date, endTime: Date): Promise<Event[]> {
    if (!this.historyPath || !fs.existsSync(this.historyPath)) {
      return [];
    }

    const content = fs.readFileSync(this.historyPath, 'utf-8');
    const lines = content.split('\n');

    const events: Event[] = [];

    if (this.shell === 'zsh') {
      // Zsh extended history format: : timestamp:0;command
      const historyRegex = /^:\s*(\d+):\d+;(.+)$/;

      for (const line of lines) {
        const match = line.match(historyRegex);
        if (match) {
          const timestamp = new Date(Number.parseInt(match[1], 10) * 1000);
          const command = match[2];

          // Filter by time range
          if (timestamp >= startTime && timestamp <= endTime) {
            events.push(this.createEvent(command, timestamp));
          }
        }
      }
    } else {
      // Bash history doesn't have timestamps by default
      // We'll estimate based on file modification time and line count
      const stats = fs.statSync(this.historyPath);
      const totalLines = lines.filter((l) => l.trim()).length;

      // Estimate: assume commands are evenly distributed over the last 24 hours
      const msPerCommand = (24 * 60 * 60 * 1000) / totalLines;
      let estimatedTime = new Date(stats.mtimeMs - totalLines * msPerCommand);

      for (const line of lines) {
        const command = line.trim();
        if (!command || command.startsWith('#')) continue;

        // Filter by time range
        if (estimatedTime >= startTime && estimatedTime <= endTime) {
          events.push(this.createEvent(command, estimatedTime));
        }

        estimatedTime = new Date(estimatedTime.getTime() + msPerCommand);
      }
    }

    return events;
  }

  /**
   * Create an event from a command
   */
  private createEvent(command: string, timestamp: Date): Event {
    // Try to detect git branch from command context
    const gitBranch = this.detectGitBranch(command);

    // Infer exit code (we can't know for sure from history)
    // Assume success unless it's a common failure pattern
    const exitCode = this.inferExitCode(command);

    const data: ShellEventData = {
      command,
      exitCode,
      durationMs: 0, // Unknown from history
      cwd: process.cwd(),
      gitBranch,
    };

    return {
      id: `shell-${timestamp.getTime()}-${Math.random().toString(36).substr(2, 9)}`,
      timestamp,
      source: 'shell',
      eventType: exitCode === 0 ? 'command' : 'command_failed',
      data,
      project: this.detectProject(command),
    };
  }

  /**
   * Try to detect git branch from command
   */
  private detectGitBranch(command: string): string | undefined {
    // Check if it's a git command with branch reference
    const branchMatch = command.match(/git\s+(checkout|switch|branch|merge|rebase)\s+(\S+)/);
    if (branchMatch) {
      return branchMatch[2];
    }
    return undefined;
  }

  /**
   * Infer exit code from command pattern
   */
  private inferExitCode(command: string): number {
    // Commands that commonly fail (this is a rough heuristic)
    const failurePatterns = [
      /^(sudo\s+)?rm\s+-rf?\s+\//, // Dangerous rm
      /\|\s*grep.*\$/, // Grep with no results often returns 1
    ];

    for (const pattern of failurePatterns) {
      if (pattern.test(command)) {
        return 1;
      }
    }

    return 0;
  }

  /**
   * Try to detect project from command
   */
  private detectProject(command: string): string | undefined {
    // Look for common project directory patterns in command
    const cdMatch = command.match(/cd\s+(?:~\/)?([^\/\s]+)/);
    if (cdMatch) {
      return cdMatch[1];
    }

    return undefined;
  }
}
