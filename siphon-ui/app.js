// Siphon Dashboard
// Fetches data from the daemon API at localhost:9847 and renders it.

const API = "http://127.0.0.1:9847";

// ── Helpers ─────────────────────────────────────────────────────────

function formatNumber(n) {
  if (n >= 1000000) return (n / 1000000).toFixed(1) + "M";
  if (n >= 1000) return (n / 1000).toFixed(1) + "K";
  return String(n);
}

function formatDuration(minutes) {
  if (minutes == null) return "--";
  if (minutes < 60) return minutes + "m";
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  return m > 0 ? h + "h " + m + "m" : h + "h";
}

function formatTime(isoString) {
  const d = new Date(isoString);
  return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function formatDate(dateStr) {
  const d = new Date(dateStr + "T00:00:00");
  return d.toLocaleDateString([], { month: "short", day: "numeric" });
}

function timeAgo(isoString) {
  const seconds = Math.floor((Date.now() - new Date(isoString).getTime()) / 1000);
  if (seconds < 60) return "just now";
  if (seconds < 3600) return Math.floor(seconds / 60) + "m ago";
  if (seconds < 86400) return Math.floor(seconds / 3600) + "h ago";
  return Math.floor(seconds / 86400) + "d ago";
}

const SOURCE_COLORS = {
  shell: "source-shell",
  editor: "source-editor",
  filesystem: "source-filesystem",
  window: "source-window",
  git: "source-git",
  browser: "source-browser",
  clipboard: "source-clipboard",
  hotkey: "source-hotkey",
  meeting: "source-meeting",
};

async function fetchJSON(path) {
  const res = await fetch(API + path);
  if (!res.ok) throw new Error("HTTP " + res.status);
  return res.json();
}

// ── Data Fetching ───────────────────────────────────────────────────

async function loadSession() {
  try {
    const data = await fetchJSON("/session");
    const badge = document.getElementById("session-badge");
    badge.textContent = data.state || "offline";
    badge.className = "badge badge-" + (data.state || "offline");

    const duration = document.getElementById("session-duration");
    duration.textContent = data.duration_minutes != null
      ? formatDuration(data.duration_minutes)
      : "--";
  } catch {
    const badge = document.getElementById("session-badge");
    badge.textContent = "offline";
    badge.className = "badge badge-offline";
  }
}

async function loadStats() {
  try {
    const data = await fetchJSON("/stats");
    document.getElementById("total-events").textContent = formatNumber(data.total_events);
    renderSourceBreakdown(data.events_by_source);
  } catch {
    document.getElementById("total-events").textContent = "--";
  }
}

async function loadStorage() {
  try {
    const data = await fetchJSON("/storage");
    document.getElementById("db-size").textContent = data.db_size_human;
    renderDailyChart(data.daily_counts);
  } catch {
    document.getElementById("db-size").textContent = "--";
  }
}

async function loadSummary(hours) {
  const container = document.getElementById("summary-content");
  try {
    const data = await fetchJSON("/summary?hours=" + hours);
    if (!data.summary) {
      container.innerHTML = '<p class="placeholder-text">No activity data for this period</p>';
      document.getElementById("focus-score").textContent = "--";
      return;
    }

    const s = data.summary;

    // Focus score
    const scoreEl = document.getElementById("focus-score");
    scoreEl.textContent = s.focus_score != null ? s.focus_score : "--";
    scoreEl.className = "stat-value";
    if (s.focus_score != null) {
      if (s.focus_score >= 70) scoreEl.classList.add("focus-high");
      else if (s.focus_score >= 40) scoreEl.classList.add("focus-medium");
      else scoreEl.classList.add("focus-low");
    }

    let html = "";

    // Summary text
    if (s.summary_text) {
      html += '<p class="summary-text">' + escapeHtml(s.summary_text) + "</p>";
    }

    // Projects
    if (s.projects && s.projects.length > 0) {
      html += '<div class="summary-section-label">Projects</div>';
      html += '<div class="summary-projects">';
      for (const p of s.projects) {
        html += '<div class="project-item">';
        html += '<span class="project-name">' + escapeHtml(p.name) + "</span>";
        html += '<span class="project-meta">' + p.event_count + " events &middot; " + formatDuration(p.duration_minutes) + "</span>";
        html += "</div>";
      }
      html += "</div>";
    }

    // Applications
    if (s.applications && s.applications.length > 0) {
      html += '<div class="summary-section-label">Applications</div>';
      html += '<div class="summary-apps">';
      for (const a of s.applications) {
        html += '<span class="app-tag">' + escapeHtml(a.app_name) + " &middot; " + formatDuration(a.duration_minutes) + "</span>";
      }
      html += "</div>";
    }

    // Key activities
    if (s.key_activities && s.key_activities.length > 0) {
      html += '<div class="summary-section-label">Key Activities</div>';
      html += '<div class="key-activities">';
      for (const a of s.key_activities) {
        html += '<div class="activity-item">';
        html += '<span class="activity-dot"></span>';
        html += '<span>' + escapeHtml(a.description) + "</span>";
        if (a.timestamp) {
          html += '<span class="activity-time">' + formatTime(a.timestamp) + "</span>";
        }
        html += "</div>";
      }
      html += "</div>";
    }

    // Meetings
    if (s.meetings && s.meetings.length > 0) {
      html += '<div class="summary-section-label">Meetings</div>';
      html += '<div class="key-activities">';
      for (const m of s.meetings) {
        html += '<div class="activity-item">';
        html += '<span class="activity-dot" style="background: #a78bfa"></span>';
        html += "<span>" + escapeHtml(m.title || m.platform) + " &middot; " + formatDuration(m.duration_minutes) + "</span>";
        html += "</div>";
      }
      html += "</div>";
    }

    container.innerHTML = html || '<p class="placeholder-text">No summary data available</p>';
  } catch {
    container.innerHTML = '<p class="placeholder-text">Could not load summary</p>';
  }
}

async function loadRecentEvents() {
  const container = document.getElementById("recent-events");
  const countEl = document.getElementById("recent-count");
  try {
    const data = await fetchJSON("/events/recent");
    const events = data.events || [];
    countEl.textContent = events.length + " events (2h)";

    if (events.length === 0) {
      container.innerHTML = '<div class="no-data">No recent events</div>';
      return;
    }

    // Show most recent first, limit to 50
    const recent = events.slice(-50).reverse();
    let html = "";
    for (const evt of recent) {
      const colorClass = SOURCE_COLORS[evt.source] || "source-shell";
      const displayText = getEventDisplayText(evt);
      html += '<div class="event-item">';
      html += '<div class="event-source-icon ' + colorClass + '"></div>';
      html += '<div class="event-body">';
      html += '<div class="event-text">' + escapeHtml(displayText) + "</div>";
      html += '<div class="event-meta">' + evt.source + (evt.project ? " &middot; " + escapeHtml(evt.project) : "") + "</div>";
      html += "</div>";
      html += '<span class="event-time">' + timeAgo(evt.timestamp) + "</span>";
      html += "</div>";
    }
    container.innerHTML = html;
  } catch {
    container.innerHTML = '<p class="placeholder-text">Could not load events</p>';
  }
}

async function loadActiveWindow() {
  const container = document.getElementById("active-window");
  try {
    const data = await fetchJSON("/window");
    if (!data.tracking_enabled || !data.window) {
      container.innerHTML = '<p class="placeholder-text">Window tracking disabled</p>';
      return;
    }

    const w = data.window;
    const dur = data.duration_ms ? formatDuration(Math.floor(data.duration_ms / 60000)) : "";
    let html = '<div class="window-info"><div>';
    html += '<div class="window-app">' + escapeHtml(w.app_name) + "</div>";
    if (w.window_title) {
      html += '<div class="window-title">' + escapeHtml(w.window_title) + "</div>";
    }
    if (dur) {
      html += '<div class="window-duration">Active for ' + dur + "</div>";
    }
    html += "</div></div>";
    container.innerHTML = html;
  } catch {
    container.innerHTML = '<p class="placeholder-text">Could not load window info</p>';
  }
}

// ── Rendering ───────────────────────────────────────────────────────

function renderDailyChart(dailyCounts) {
  const chart = document.getElementById("daily-chart");
  const rangeEl = document.getElementById("activity-range");

  if (!dailyCounts || dailyCounts.length === 0) {
    chart.innerHTML = '<div class="no-data">No activity data yet</div>';
    return;
  }

  // dailyCounts is [[date, count], ...] — show last 14 days
  const data = dailyCounts.slice(-14);
  const maxCount = Math.max(...data.map(function(d) { return d[1]; }), 1);

  if (data.length > 0) {
    rangeEl.textContent = formatDate(data[0][0]) + " - " + formatDate(data[data.length - 1][0]);
  }

  let html = "";
  for (const entry of data) {
    const date = entry[0];
    const count = entry[1];
    const heightPct = Math.max((count / maxCount) * 100, 2);
    const label = formatDate(date);
    html += '<div class="bar-col">';
    html += '<div class="bar" style="height: ' + heightPct + '%">';
    html += '<span class="bar-tooltip">' + count.toLocaleString() + " events</span>";
    html += "</div>";
    html += '<span class="bar-label">' + label + "</span>";
    html += "</div>";
  }
  chart.innerHTML = html;
}

function renderSourceBreakdown(eventsBySource) {
  const container = document.getElementById("source-breakdown");

  if (!eventsBySource || eventsBySource.length === 0) {
    container.innerHTML = '<div class="no-data">No data</div>';
    return;
  }

  // Sort by count descending
  const sorted = eventsBySource.slice().sort(function(a, b) { return b[1] - a[1]; });
  const maxCount = sorted[0][1] || 1;

  let html = "";
  for (const entry of sorted) {
    const source = entry[0];
    const count = entry[1];
    const pct = (count / maxCount) * 100;
    const colorClass = SOURCE_COLORS[source] || "source-shell";
    html += '<div class="source-row">';
    html += '<span class="source-name">' + source + "</span>";
    html += '<div class="source-bar-track"><div class="source-bar-fill ' + colorClass + '" style="width: ' + pct + '%"></div></div>';
    html += '<span class="source-count">' + formatNumber(count) + "</span>";
    html += "</div>";
  }
  container.innerHTML = html;
}

// ── Event display text extraction ───────────────────────────────────

function getEventDisplayText(evt) {
  try {
    const data = typeof evt.event_data === "string" ? JSON.parse(evt.event_data) : evt.event_data;
    switch (evt.source) {
      case "shell":
        return data.command || evt.event_type;
      case "editor":
        return (data.action || "") + " " + (data.file_path || "").split("/").pop();
      case "filesystem":
        return (data.action || "") + " " + (data.file_path || "").split("/").pop();
      case "window":
        if (data.current) return data.current.app_name + " - " + (data.current.window_title || "");
        return evt.event_type;
      case "git":
        return data.message || data.action || evt.event_type;
      case "browser":
        return data.title || data.url || evt.event_type;
      case "clipboard":
        return "Clipboard change";
      case "hotkey":
        return data.action || "Hotkey triggered";
      case "meeting":
        return (data.event_type || evt.event_type) + " - " + (data.platform || "");
      default:
        return evt.event_type;
    }
  } catch {
    return evt.event_type;
  }
}

function escapeHtml(text) {
  if (!text) return "";
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}

// ── Initialization ──────────────────────────────────────────────────

async function refreshAll() {
  await Promise.allSettled([
    loadSession(),
    loadStats(),
    loadStorage(),
    loadSummary(document.getElementById("summary-hours").value),
    loadRecentEvents(),
    loadActiveWindow(),
  ]);

  document.getElementById("last-updated").textContent =
    "Updated " + new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

// Initial load
refreshAll();

// Auto-refresh every 15 seconds
setInterval(refreshAll, 15000);

// Summary hours selector
document.getElementById("summary-hours").addEventListener("change", function () {
  loadSummary(this.value);
});
