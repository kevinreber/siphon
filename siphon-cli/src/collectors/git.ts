/**
 * Git Collector
 *
 * Reads git log and creates events from commits.
 * Also provides incremental diff analysis.
 */

import { execSync } from 'node:child_process';
import type { Event, GitEventData } from '../types.js';

export interface DiffStats {
  filesChanged: number;
  insertions: number;
  deletions: number;
  files: Array<{
    path: string;
    insertions: number;
    deletions: number;
    status: 'added' | 'modified' | 'deleted' | 'renamed';
  }>;
}

export interface IncrementalDiff {
  fromCommit: string;
  toCommit: string;
  stats: DiffStats;
  summary: string;
  topChangedFiles: string[];
  languageBreakdown: Record<string, { files: number; lines: number }>;
}

export class GitCollector {
  /**
   * Collect git events within a time range
   */
  async collect(startTime: Date, endTime: Date): Promise<Event[]> {
    const events: Event[] = [];

    try {
      // Get git log with timestamps
      const sinceDate = startTime.toISOString();
      const untilDate = endTime.toISOString();

      const output = execSync(
        `git log --since="${sinceDate}" --until="${untilDate}" --format="%H|%aI|%s|%an" --all 2>/dev/null`,
        { encoding: 'utf-8', maxBuffer: 10 * 1024 * 1024 }
      );

      const lines = output
        .trim()
        .split('\n')
        .filter((l) => l);

      for (const line of lines) {
        const [hash, timestamp, message, _author] = line.split('|');
        if (!hash || !timestamp) continue;

        const commitTime = new Date(timestamp);

        // Get files changed in this commit
        let filesChanged = 0;
        try {
          const statOutput = execSync(`git show --stat --format="" ${hash} 2>/dev/null`, {
            encoding: 'utf-8',
          });
          filesChanged = (statOutput.match(/\d+ files? changed/g) || []).length || 1;
        } catch {
          filesChanged = 1;
        }

        const data: GitEventData = {
          action: 'commit',
          branch: this.getCurrentBranch(),
          message: message || '',
          filesChanged,
        };

        events.push({
          id: `git-${hash.slice(0, 8)}`,
          timestamp: commitTime,
          source: 'git',
          eventType: 'commit',
          data,
          project: this.getRepoName(),
        });
      }

      // Also capture recent branch switches from reflog
      const branchEvents = this.getBranchSwitches(startTime, endTime);
      events.push(...branchEvents);
    } catch (_err) {
      // Not in a git repo or git not available
      return [];
    }

    return events;
  }

  /**
   * Get current branch name
   */
  private getCurrentBranch(): string | undefined {
    try {
      return execSync('git symbolic-ref --short HEAD 2>/dev/null', {
        encoding: 'utf-8',
      }).trim();
    } catch {
      return undefined;
    }
  }

  /**
   * Get repository name from remote or directory
   */
  private getRepoName(): string | undefined {
    try {
      const remote = execSync('git remote get-url origin 2>/dev/null', {
        encoding: 'utf-8',
      }).trim();

      // Extract repo name from URL
      const match = remote.match(/\/([^\/]+?)(\.git)?$/);
      if (match) {
        return match[1];
      }
    } catch {
      // No remote, use directory name
      try {
        const topLevel = execSync('git rev-parse --show-toplevel 2>/dev/null', {
          encoding: 'utf-8',
        }).trim();
        const parts = topLevel.split('/');
        return parts[parts.length - 1];
      } catch {
        return undefined;
      }
    }
    return undefined;
  }

  /**
   * Get branch switch events from reflog
   */
  private getBranchSwitches(startTime: Date, endTime: Date): Event[] {
    const events: Event[] = [];

    try {
      const output = execSync(`git reflog --format="%gd|%gs|%ci" --date=iso 2>/dev/null`, {
        encoding: 'utf-8',
      });

      const lines = output
        .trim()
        .split('\n')
        .filter((l) => l);

      for (const line of lines) {
        const [_ref, action, dateStr] = line.split('|');

        // Only capture checkout/switch actions
        if (!action || !action.includes('checkout:')) continue;

        const timestamp = new Date(dateStr);
        if (timestamp < startTime || timestamp > endTime) continue;

        // Extract branch names
        const branchMatch = action.match(/checkout: moving from (\S+) to (\S+)/);
        if (!branchMatch) continue;

        const [, fromBranch, toBranch] = branchMatch;

        const data: GitEventData = {
          action: 'branch_switch',
          branch: toBranch,
          message: `Switched from ${fromBranch} to ${toBranch}`,
        };

        events.push({
          id: `git-switch-${timestamp.getTime()}`,
          timestamp,
          source: 'git',
          eventType: 'branch_switch',
          data,
          project: this.getRepoName(),
        });
      }
    } catch {
      // Reflog not available
    }

    return events;
  }

  /**
   * Get incremental diff between two commits or time-based range
   */
  // biome-ignore lint/complexity/noExcessiveCognitiveComplexity: Complex git diff parsing with multiple formats
  getIncrementalDiff(fromCommit?: string, toCommit?: string): IncrementalDiff | null {
    try {
      const from = fromCommit || this.getCommitFromHoursAgo(24);
      const to = toCommit || 'HEAD';

      if (!from) return null;

      // Get diff stats
      const numstatOutput = execSync(`git diff --numstat ${from}..${to} 2>/dev/null`, {
        encoding: 'utf-8',
      });

      const files: DiffStats['files'] = [];
      let totalInsertions = 0;
      let totalDeletions = 0;
      const languageBreakdown: Record<string, { files: number; lines: number }> = {};

      for (const line of numstatOutput.trim().split('\n')) {
        if (!line) continue;
        const [ins, del, path] = line.split('\t');

        // Handle binary files
        const insertions = ins === '-' ? 0 : Number.parseInt(ins, 10);
        const deletions = del === '-' ? 0 : Number.parseInt(del, 10);

        totalInsertions += insertions;
        totalDeletions += deletions;

        // Detect file status
        let status: 'added' | 'modified' | 'deleted' | 'renamed' = 'modified';
        if (insertions > 0 && deletions === 0) {
          // Could be a new file - check with diff --name-status
          status = 'added';
        }

        files.push({ path, insertions, deletions, status });

        // Track by language/extension
        const ext = path.split('.').pop() || 'other';
        if (!languageBreakdown[ext]) {
          languageBreakdown[ext] = { files: 0, lines: 0 };
        }
        languageBreakdown[ext].files++;
        languageBreakdown[ext].lines += insertions + deletions;
      }

      // Get name-status for accurate status detection
      try {
        const nameStatusOutput = execSync(`git diff --name-status ${from}..${to} 2>/dev/null`, {
          encoding: 'utf-8',
        });

        const statusMap = new Map<string, string>();
        for (const line of nameStatusOutput.trim().split('\n')) {
          if (!line) continue;
          const [status, ...pathParts] = line.split('\t');
          const path = pathParts[pathParts.length - 1]; // Handle renames
          statusMap.set(path, status);
        }

        for (const file of files) {
          const status = statusMap.get(file.path);
          if (status === 'A') file.status = 'added';
          else if (status === 'D') file.status = 'deleted';
          else if (status?.startsWith('R')) file.status = 'renamed';
        }
      } catch {
        // Ignore status detection errors
      }

      // Sort by changes (most changed first)
      files.sort((a, b) => b.insertions + b.deletions - (a.insertions + a.deletions));

      const stats: DiffStats = {
        filesChanged: files.length,
        insertions: totalInsertions,
        deletions: totalDeletions,
        files,
      };

      // Generate summary
      const summary = this.generateDiffSummary(stats, languageBreakdown);

      return {
        fromCommit: from,
        toCommit: to,
        stats,
        summary,
        topChangedFiles: files.slice(0, 10).map((f) => f.path),
        languageBreakdown,
      };
    } catch {
      return null;
    }
  }

  /**
   * Get commit hash from N hours ago
   */
  private getCommitFromHoursAgo(hours: number): string | null {
    try {
      const output = execSync(`git rev-list -1 --before="${hours} hours ago" HEAD 2>/dev/null`, {
        encoding: 'utf-8',
      }).trim();
      return output || null;
    } catch {
      return null;
    }
  }

  /**
   * Generate a human-readable diff summary
   */
  private generateDiffSummary(
    stats: DiffStats,
    languageBreakdown: Record<string, { files: number; lines: number }>
  ): string {
    const parts: string[] = [];

    parts.push(`${stats.filesChanged} files changed`);
    parts.push(`+${stats.insertions} -${stats.deletions} lines`);

    // Top languages
    const topLangs = Object.entries(languageBreakdown)
      .sort((a, b) => b[1].lines - a[1].lines)
      .slice(0, 3)
      .map(([ext, data]) => `${ext} (${data.files} files)`)
      .join(', ');

    if (topLangs) {
      parts.push(`Main: ${topLangs}`);
    }

    return parts.join(' | ');
  }

  /**
   * Get working directory changes (unstaged + staged)
   */
  getWorkingChanges(): DiffStats | null {
    try {
      // Get unstaged changes
      const unstagedOutput = execSync('git diff --numstat 2>/dev/null', { encoding: 'utf-8' });
      // Get staged changes
      const stagedOutput = execSync('git diff --cached --numstat 2>/dev/null', {
        encoding: 'utf-8',
      });

      const allLines = [
        ...unstagedOutput.trim().split('\n'),
        ...stagedOutput.trim().split('\n'),
      ].filter((l) => l);

      const files: DiffStats['files'] = [];
      let totalInsertions = 0;
      let totalDeletions = 0;

      for (const line of allLines) {
        const [ins, del, path] = line.split('\t');
        const insertions = ins === '-' ? 0 : Number.parseInt(ins, 10);
        const deletions = del === '-' ? 0 : Number.parseInt(del, 10);

        // Check if file already tracked (avoid duplicates)
        const existing = files.find((f) => f.path === path);
        if (existing) {
          existing.insertions = Math.max(existing.insertions, insertions);
          existing.deletions = Math.max(existing.deletions, deletions);
        } else {
          files.push({ path, insertions, deletions, status: 'modified' });
          totalInsertions += insertions;
          totalDeletions += deletions;
        }
      }

      return {
        filesChanged: files.length,
        insertions: totalInsertions,
        deletions: totalDeletions,
        files,
      };
    } catch {
      return null;
    }
  }

  /**
   * Get commits grouped by day for multi-day analysis
   */
  getCommitsByDay(
    days: number
  ): Map<string, Array<{ hash: string; message: string; timestamp: Date }>> {
    const result = new Map<string, Array<{ hash: string; message: string; timestamp: Date }>>();

    try {
      const sinceDate = new Date(Date.now() - days * 24 * 60 * 60 * 1000).toISOString();

      const output = execSync(
        `git log --since="${sinceDate}" --format="%H|%aI|%s" --all 2>/dev/null`,
        { encoding: 'utf-8' }
      );

      for (const line of output.trim().split('\n')) {
        if (!line) continue;
        const [hash, timestamp, message] = line.split('|');
        if (!hash || !timestamp) continue;

        const date = new Date(timestamp);
        const dateKey = date.toISOString().split('T')[0];

        if (!result.has(dateKey)) {
          result.set(dateKey, []);
        }
        result.get(dateKey)?.push({ hash, message, timestamp: date });
      }
    } catch {
      // Not in git repo
    }

    return result;
  }
}
