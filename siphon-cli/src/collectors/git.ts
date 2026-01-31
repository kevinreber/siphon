/**
 * Git Collector
 *
 * Reads git log and creates events from commits.
 */

import { execSync } from "child_process";
import { Event, GitEventData } from "../types.js";

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
        { encoding: "utf-8", maxBuffer: 10 * 1024 * 1024 }
      );

      const lines = output.trim().split("\n").filter((l) => l);

      for (const line of lines) {
        const [hash, timestamp, message, author] = line.split("|");
        if (!hash || !timestamp) continue;

        const commitTime = new Date(timestamp);

        // Get files changed in this commit
        let filesChanged = 0;
        try {
          const statOutput = execSync(
            `git show --stat --format="" ${hash} 2>/dev/null`,
            { encoding: "utf-8" }
          );
          filesChanged = (statOutput.match(/\d+ files? changed/g) || []).length || 1;
        } catch {
          filesChanged = 1;
        }

        const data: GitEventData = {
          action: "commit",
          branch: this.getCurrentBranch(),
          message: message || "",
          filesChanged,
        };

        events.push({
          id: `git-${hash.slice(0, 8)}`,
          timestamp: commitTime,
          source: "git",
          eventType: "commit",
          data,
          project: this.getRepoName(),
        });
      }

      // Also capture recent branch switches from reflog
      const branchEvents = this.getBranchSwitches(startTime, endTime);
      events.push(...branchEvents);

    } catch (err) {
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
      return execSync("git symbolic-ref --short HEAD 2>/dev/null", {
        encoding: "utf-8",
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
      const remote = execSync("git remote get-url origin 2>/dev/null", {
        encoding: "utf-8",
      }).trim();

      // Extract repo name from URL
      const match = remote.match(/\/([^\/]+?)(\.git)?$/);
      if (match) {
        return match[1];
      }
    } catch {
      // No remote, use directory name
      try {
        const topLevel = execSync("git rev-parse --show-toplevel 2>/dev/null", {
          encoding: "utf-8",
        }).trim();
        const parts = topLevel.split("/");
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
      const output = execSync(
        `git reflog --format="%gd|%gs|%ci" --date=iso 2>/dev/null`,
        { encoding: "utf-8" }
      );

      const lines = output.trim().split("\n").filter((l) => l);

      for (const line of lines) {
        const [ref, action, dateStr] = line.split("|");

        // Only capture checkout/switch actions
        if (!action || !action.includes("checkout:")) continue;

        const timestamp = new Date(dateStr);
        if (timestamp < startTime || timestamp > endTime) continue;

        // Extract branch names
        const branchMatch = action.match(/checkout: moving from (\S+) to (\S+)/);
        if (!branchMatch) continue;

        const [, fromBranch, toBranch] = branchMatch;

        const data: GitEventData = {
          action: "branch_switch",
          branch: toBranch,
          message: `Switched from ${fromBranch} to ${toBranch}`,
        };

        events.push({
          id: `git-switch-${timestamp.getTime()}`,
          timestamp,
          source: "git",
          eventType: "branch_switch",
          data,
          project: this.getRepoName(),
        });
      }
    } catch {
      // Reflog not available
    }

    return events;
  }
}
