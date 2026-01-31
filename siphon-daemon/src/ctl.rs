//! siphon-ctl - Control CLI for the Siphon daemon

use clap::{Parser, Subcommand};
use serde::Deserialize;

const DEFAULT_API_URL: &str = "http://127.0.0.1:9847";

#[derive(Parser)]
#[command(name = "siphon-ctl")]
#[command(about = "Control CLI for the Siphon daemon")]
#[command(version)]
struct Cli {
    /// API URL for the daemon
    #[arg(long, default_value = DEFAULT_API_URL)]
    api_url: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check if daemon is running
    Status,

    /// Show event statistics
    Stats,

    /// List recent events
    Events {
        /// Number of hours to look back
        #[arg(short = 'H', long, default_value = "2")]
        hours: u32,

        /// Filter by source (shell, editor, filesystem)
        #[arg(short, long)]
        source: Option<String>,
    },

    /// Show content ideas based on recent activity
    Ideas {
        /// Number of hours to analyze
        #[arg(short = 'H', long, default_value = "4")]
        hours: u32,
    },
}

#[derive(Deserialize)]
struct HealthResponse {
    status: String,
    version: String,
}

#[derive(Deserialize)]
struct StatsResponse {
    total_events: i64,
    events_by_source: Vec<(String, i64)>,
}

#[derive(Deserialize)]
struct EventsResponse {
    events: Vec<Event>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct Event {
    id: String,
    timestamp: String,
    source: String,
    event_type: String,
    event_data: String,
    project: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Status => cmd_status(&cli.api_url),
        Commands::Stats => cmd_stats(&cli.api_url),
        Commands::Events { hours, source } => cmd_events(&cli.api_url, hours, source),
        Commands::Ideas { hours } => cmd_ideas(&cli.api_url, hours),
    }
}

fn cmd_status(api_url: &str) {
    let url = format!("{}/health", api_url);

    match reqwest::blocking::get(&url) {
        Ok(resp) => {
            if resp.status().is_success() {
                let health: HealthResponse = resp.json().unwrap_or(HealthResponse {
                    status: "unknown".to_string(),
                    version: "unknown".to_string(),
                });
                println!("Siphon daemon is running");
                println!("  Status: {}", health.status);
                println!("  Version: {}", health.version);
                println!("  API: {}", api_url);
            } else {
                println!("Siphon daemon returned error: {}", resp.status());
            }
        }
        Err(_) => {
            println!("Siphon daemon is not running");
            println!("  Start it with: siphon-daemon");
            std::process::exit(1);
        }
    }
}

fn cmd_stats(api_url: &str) {
    let url = format!("{}/stats", api_url);

    match reqwest::blocking::get(&url) {
        Ok(resp) => {
            if resp.status().is_success() {
                let stats: StatsResponse = resp.json().unwrap();
                println!("Event Statistics");
                println!("================");
                println!("Total events: {}", stats.total_events);
                println!();
                if !stats.events_by_source.is_empty() {
                    println!("By source:");
                    for (source, count) in stats.events_by_source {
                        println!("  {}: {}", source, count);
                    }
                }
            } else {
                eprintln!("Error: {}", resp.status());
            }
        }
        Err(e) => {
            eprintln!("Failed to connect to daemon: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_events(api_url: &str, hours: u32, source: Option<String>) {
    let mut url = format!("{}/events?hours={}", api_url, hours);
    if let Some(s) = source {
        url.push_str(&format!("&source={}", s));
    }

    match reqwest::blocking::get(&url) {
        Ok(resp) => {
            if resp.status().is_success() {
                let events_resp: EventsResponse = resp.json().unwrap();
                let events = events_resp.events;

                if events.is_empty() {
                    println!("No events in the last {} hours", hours);
                    return;
                }

                println!("Recent Events (last {} hours)", hours);
                println!("==============================");
                println!();

                for event in events.iter().take(20) {
                    print_event(event);
                }

                if events.len() > 20 {
                    println!("... and {} more events", events.len() - 20);
                }
            } else {
                eprintln!("Error: {}", resp.status());
            }
        }
        Err(e) => {
            eprintln!("Failed to connect to daemon: {}", e);
            std::process::exit(1);
        }
    }
}

fn print_event(event: &Event) {
    let time = &event.timestamp[11..19]; // Extract HH:MM:SS

    // Parse event data to show command or action
    let summary = if event.source == "shell" {
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&event.event_data) {
            let cmd = data["command"].as_str().unwrap_or("?");
            let exit_code = data["exit_code"].as_i64().unwrap_or(0);
            let duration = data["duration_ms"].as_u64().unwrap_or(0);

            let status = if exit_code == 0 { "" } else { "" };
            let truncated = if cmd.len() > 60 {
                format!("{}...", &cmd[..60])
            } else {
                cmd.to_string()
            };
            format!("{} {} ({}ms)", status, truncated, duration)
        } else {
            event.event_type.clone()
        }
    } else if event.source == "editor" {
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&event.event_data) {
            let action = data["action"].as_str().unwrap_or("?");
            let path = data["file_path"].as_str().unwrap_or("?");
            let filename = std::path::Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path);
            format!("{} {}", action, filename)
        } else {
            event.event_type.clone()
        }
    } else {
        event.event_type.clone()
    };

    let project = event.project.as_deref().unwrap_or("-");
    println!("[{}] {:10} {:12} {}", time, event.source, project, summary);
}

fn cmd_ideas(api_url: &str, hours: u32) {
    let url = format!("{}/events?hours={}", api_url, hours);

    match reqwest::blocking::get(&url) {
        Ok(resp) => {
            if resp.status().is_success() {
                let events_resp: EventsResponse = resp.json().unwrap();
                let events = events_resp.events;

                if events.is_empty() {
                    println!("No events in the last {} hours", hours);
                    return;
                }

                // Basic analysis - count topics
                let mut topics: std::collections::HashMap<String, usize> =
                    std::collections::HashMap::new();
                let mut failed_commands = 0;
                let mut total_commands = 0;

                for event in &events {
                    if event.source == "shell" {
                        total_commands += 1;
                        if let Ok(data) =
                            serde_json::from_str::<serde_json::Value>(&event.event_data)
                        {
                            if data["exit_code"].as_i64().unwrap_or(0) != 0 {
                                failed_commands += 1;
                            }

                            // Extract topic from command
                            if let Some(cmd) = data["command"].as_str() {
                                let first_word = cmd.split_whitespace().next().unwrap_or("");
                                if !first_word.is_empty() {
                                    *topics.entry(first_word.to_string()).or_insert(0) += 1;
                                }
                            }
                        }
                    }
                }

                println!("Content Ideas (based on last {} hours)", hours);
                println!("=========================================");
                println!();

                // Calculate struggle score
                let struggle_score = if total_commands > 0 {
                    (failed_commands as f64 / total_commands as f64 * 100.0) as u32
                } else {
                    0
                };

                if struggle_score > 20 {
                    println!("High Struggle Score: {}%", struggle_score);
                    println!(
                        "  {} failed commands out of {}",
                        failed_commands, total_commands
                    );
                    println!("  This debugging session could make great content!");
                    println!();
                }

                // Top tools
                let mut topics_vec: Vec<_> = topics.into_iter().collect();
                topics_vec.sort_by(|a, b| b.1.cmp(&a.1));

                println!("Top tools used:");
                for (tool, count) in topics_vec.iter().take(5) {
                    println!("  {} ({}x)", tool, count);
                }

                println!();
                println!(
                    "For detailed analysis, use: siphon capture --time {}h",
                    hours
                );
            } else {
                eprintln!("Error: {}", resp.status());
            }
        }
        Err(e) => {
            eprintln!("Failed to connect to daemon: {}", e);
            eprintln!("Make sure the daemon is running: siphon-daemon");
            std::process::exit(1);
        }
    }
}
