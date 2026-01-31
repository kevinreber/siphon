/**
 * Browser History Collector
 *
 * Reads browser history from Chrome and Firefox
 * to detect research and documentation browsing sessions.
 */

import Database from 'better-sqlite3';
import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';
import type { Event } from '../types.js';

export interface BrowserEventData {
  url: string;
  title: string;
  visitCount: number;
  browser: 'chrome' | 'firefox' | 'safari' | 'edge';
  domain: string;
  category?: string;
}

// Domains that indicate developer research
const DEVELOPER_DOMAINS: Record<string, string> = {
  'stackoverflow.com': 'q&a',
  'github.com': 'code',
  'docs.github.com': 'documentation',
  'developer.mozilla.org': 'documentation',
  'mdn.io': 'documentation',
  'npmjs.com': 'package',
  'crates.io': 'package',
  'pypi.org': 'package',
  'pkg.go.dev': 'documentation',
  'rust-lang.org': 'documentation',
  'docs.rs': 'documentation',
  'nodejs.org': 'documentation',
  'typescriptlang.org': 'documentation',
  'python.org': 'documentation',
  'reactjs.org': 'documentation',
  'react.dev': 'documentation',
  'vuejs.org': 'documentation',
  'angular.io': 'documentation',
  'svelte.dev': 'documentation',
  'kubernetes.io': 'documentation',
  'docker.com': 'documentation',
  'aws.amazon.com': 'documentation',
  'cloud.google.com': 'documentation',
  'docs.microsoft.com': 'documentation',
  'learn.microsoft.com': 'documentation',
  'medium.com': 'article',
  'dev.to': 'article',
  'hashnode.dev': 'article',
  'freecodecamp.org': 'tutorial',
  'codecademy.com': 'tutorial',
  'udemy.com': 'tutorial',
  'coursera.org': 'tutorial',
  'egghead.io': 'tutorial',
  'frontendmasters.com': 'tutorial',
  'youtube.com': 'video',
  'twitch.tv': 'video',
  'reddit.com': 'forum',
  'news.ycombinator.com': 'forum',
  'lobste.rs': 'forum',
  'discord.com': 'community',
  'slack.com': 'community',
  'chat.openai.com': 'ai',
  'claude.ai': 'ai',
  'bard.google.com': 'ai',
  'vercel.com': 'platform',
  'netlify.com': 'platform',
  'heroku.com': 'platform',
  'digitalocean.com': 'platform',
};

// Patterns that indicate developer-related searches
const SEARCH_PATTERNS = [
  /how\s+to/i,
  /error|exception|bug|fix/i,
  /tutorial|guide|example/i,
  /documentation|docs|api/i,
  /install|setup|config/i,
  /debug|troubleshoot/i,
  /best\s+practice/i,
  /vs\s+|versus|comparison/i,
];

export class BrowserHistoryCollector {
  private chromeDbPath: string | null;
  private firefoxDbPath: string | null;

  constructor() {
    this.chromeDbPath = this.findChromeHistory();
    this.firefoxDbPath = this.findFirefoxHistory();
  }

  /**
   * Find Chrome history database path
   */
  private findChromeHistory(): string | null {
    const homeDir = os.homedir();
    const platform = os.platform();

    const possiblePaths = [];

    if (platform === 'darwin') {
      possiblePaths.push(
        path.join(homeDir, 'Library/Application Support/Google/Chrome/Default/History'),
        path.join(homeDir, 'Library/Application Support/Google/Chrome/Profile 1/History'),
        path.join(homeDir, 'Library/Application Support/Chromium/Default/History')
      );
    } else if (platform === 'linux') {
      possiblePaths.push(
        path.join(homeDir, '.config/google-chrome/Default/History'),
        path.join(homeDir, '.config/google-chrome/Profile 1/History'),
        path.join(homeDir, '.config/chromium/Default/History'),
        path.join(homeDir, 'snap/chromium/common/chromium/Default/History')
      );
    } else if (platform === 'win32') {
      const appData = process.env.LOCALAPPDATA || path.join(homeDir, 'AppData/Local');
      possiblePaths.push(
        path.join(appData, 'Google/Chrome/User Data/Default/History'),
        path.join(appData, 'Google/Chrome/User Data/Profile 1/History')
      );
    }

    for (const p of possiblePaths) {
      if (fs.existsSync(p)) {
        return p;
      }
    }

    return null;
  }

  /**
   * Find Firefox history database path
   */
  private findFirefoxHistory(): string | null {
    const homeDir = os.homedir();
    const platform = os.platform();

    let profilesDir = '';

    if (platform === 'darwin') {
      profilesDir = path.join(homeDir, 'Library/Application Support/Firefox/Profiles');
    } else if (platform === 'linux') {
      profilesDir = path.join(homeDir, '.mozilla/firefox');
    } else if (platform === 'win32') {
      const appData = process.env.APPDATA || path.join(homeDir, 'AppData/Roaming');
      profilesDir = path.join(appData, 'Mozilla/Firefox/Profiles');
    }

    if (!fs.existsSync(profilesDir)) {
      return null;
    }

    // Find the default profile
    try {
      const profiles = fs.readdirSync(profilesDir);
      for (const profile of profiles) {
        if (profile.endsWith('.default') || profile.endsWith('.default-release')) {
          const historyPath = path.join(profilesDir, profile, 'places.sqlite');
          if (fs.existsSync(historyPath)) {
            return historyPath;
          }
        }
      }

      // Fallback: try any profile
      for (const profile of profiles) {
        const historyPath = path.join(profilesDir, profile, 'places.sqlite');
        if (fs.existsSync(historyPath)) {
          return historyPath;
        }
      }
    } catch {
      return null;
    }

    return null;
  }

  /**
   * Collect browser events within a time range
   */
  async collect(startTime: Date, endTime: Date, filterDevOnly = true): Promise<Event[]> {
    const events: Event[] = [];

    // Collect from Chrome
    if (this.chromeDbPath) {
      try {
        const chromeEvents = await this.collectChrome(startTime, endTime, filterDevOnly);
        events.push(...chromeEvents);
      } catch (error) {
        // Chrome history might be locked if browser is open
        // Silently continue
      }
    }

    // Collect from Firefox
    if (this.firefoxDbPath) {
      try {
        const firefoxEvents = await this.collectFirefox(startTime, endTime, filterDevOnly);
        events.push(...firefoxEvents);
      } catch (error) {
        // Firefox history might be locked if browser is open
        // Silently continue
      }
    }

    // Sort by timestamp
    events.sort((a, b) => a.timestamp.getTime() - b.timestamp.getTime());

    return events;
  }

  /**
   * Collect Chrome history
   */
  private async collectChrome(
    startTime: Date,
    endTime: Date,
    filterDevOnly: boolean
  ): Promise<Event[]> {
    if (!this.chromeDbPath) return [];

    // Copy database to avoid lock issues
    const tempPath = path.join(os.tmpdir(), `siphon-chrome-${Date.now()}.db`);
    fs.copyFileSync(this.chromeDbPath, tempPath);

    try {
      const db = new Database(tempPath, { readonly: true });

      // Chrome uses WebKit timestamps (microseconds since 1601-01-01)
      const chromeEpochOffset = 11644473600000000n; // microseconds
      const startChrome = BigInt(startTime.getTime()) * 1000n + chromeEpochOffset;
      const endChrome = BigInt(endTime.getTime()) * 1000n + chromeEpochOffset;

      const rows = db
        .prepare(
          `
        SELECT url, title, visit_count, last_visit_time
        FROM urls
        WHERE last_visit_time >= ? AND last_visit_time <= ?
        ORDER BY last_visit_time ASC
      `
        )
        .all(startChrome.toString(), endChrome.toString()) as Array<{
        url: string;
        title: string;
        visit_count: number;
        last_visit_time: string;
      }>;

      db.close();
      fs.unlinkSync(tempPath);

      const events: Event[] = [];

      for (const row of rows) {
        const event = this.createBrowserEvent(
          row.url,
          row.title,
          row.visit_count,
          'chrome',
          BigInt(row.last_visit_time)
        );

        if (event && (!filterDevOnly || this.isDeveloperRelated(event.data as BrowserEventData))) {
          events.push(event);
        }
      }

      return events;
    } catch (error) {
      // Clean up temp file on error
      if (fs.existsSync(tempPath)) {
        fs.unlinkSync(tempPath);
      }
      throw error;
    }
  }

  /**
   * Collect Firefox history
   */
  private async collectFirefox(
    startTime: Date,
    endTime: Date,
    filterDevOnly: boolean
  ): Promise<Event[]> {
    if (!this.firefoxDbPath) return [];

    // Copy database to avoid lock issues
    const tempPath = path.join(os.tmpdir(), `siphon-firefox-${Date.now()}.db`);
    fs.copyFileSync(this.firefoxDbPath, tempPath);

    try {
      const db = new Database(tempPath, { readonly: true });

      // Firefox uses microseconds since Unix epoch
      const startFirefox = startTime.getTime() * 1000;
      const endFirefox = endTime.getTime() * 1000;

      const rows = db
        .prepare(
          `
        SELECT p.url, p.title, p.visit_count, v.visit_date
        FROM moz_places p
        JOIN moz_historyvisits v ON p.id = v.place_id
        WHERE v.visit_date >= ? AND v.visit_date <= ?
        ORDER BY v.visit_date ASC
      `
        )
        .all(startFirefox, endFirefox) as Array<{
        url: string;
        title: string;
        visit_count: number;
        visit_date: number;
      }>;

      db.close();
      fs.unlinkSync(tempPath);

      const events: Event[] = [];

      for (const row of rows) {
        const timestamp = new Date(row.visit_date / 1000);
        const event = this.createBrowserEventFromTimestamp(
          row.url,
          row.title || '',
          row.visit_count,
          'firefox',
          timestamp
        );

        if (event && (!filterDevOnly || this.isDeveloperRelated(event.data as BrowserEventData))) {
          events.push(event);
        }
      }

      return events;
    } catch (error) {
      // Clean up temp file on error
      if (fs.existsSync(tempPath)) {
        fs.unlinkSync(tempPath);
      }
      throw error;
    }
  }

  /**
   * Create a browser event from Chrome timestamp
   */
  private createBrowserEvent(
    url: string,
    title: string,
    visitCount: number,
    browser: 'chrome' | 'firefox',
    chromeTimestamp: bigint
  ): Event | null {
    // Convert Chrome timestamp to JS Date
    const chromeEpochOffset = 11644473600000000n;
    const unixMicroseconds = chromeTimestamp - chromeEpochOffset;
    const timestamp = new Date(Number(unixMicroseconds / 1000n));

    return this.createBrowserEventFromTimestamp(url, title, visitCount, browser, timestamp);
  }

  /**
   * Create a browser event from a timestamp
   */
  private createBrowserEventFromTimestamp(
    url: string,
    title: string,
    visitCount: number,
    browser: 'chrome' | 'firefox',
    timestamp: Date
  ): Event | null {
    try {
      const urlObj = new URL(url);
      const domain = urlObj.hostname.replace(/^www\./, '');

      // Skip internal browser pages
      if (
        url.startsWith('chrome://') ||
        url.startsWith('chrome-extension://') ||
        url.startsWith('about:') ||
        url.startsWith('moz-extension://')
      ) {
        return null;
      }

      const category = DEVELOPER_DOMAINS[domain];

      const data: BrowserEventData = {
        url,
        title: title || url,
        visitCount,
        browser,
        domain,
        category,
      };

      return {
        id: `browser-${timestamp.getTime()}-${Math.random().toString(36).substr(2, 9)}`,
        timestamp,
        source: 'browser',
        eventType: category ? `browse_${category}` : 'browse',
        data,
      };
    } catch {
      return null;
    }
  }

  /**
   * Check if a browser event is developer-related
   */
  private isDeveloperRelated(data: BrowserEventData): boolean {
    // Check if domain is known developer site
    if (data.category) {
      return true;
    }

    // Check if title matches search patterns
    const title = data.title.toLowerCase();
    for (const pattern of SEARCH_PATTERNS) {
      if (pattern.test(title)) {
        return true;
      }
    }

    // Check URL for common dev patterns
    const url = data.url.toLowerCase();
    if (
      url.includes('/docs/') ||
      url.includes('/api/') ||
      url.includes('/reference/') ||
      url.includes('/tutorial/') ||
      url.includes('/guide/')
    ) {
      return true;
    }

    return false;
  }

  /**
   * Get available browsers
   */
  getAvailableBrowsers(): string[] {
    const browsers: string[] = [];
    if (this.chromeDbPath) browsers.push('chrome');
    if (this.firefoxDbPath) browsers.push('firefox');
    return browsers;
  }

  /**
   * Check if any browser history is available
   */
  isAvailable(): boolean {
    return this.chromeDbPath !== null || this.firefoxDbPath !== null;
  }
}
