//! Global hotkey module
//!
//! Provides global hotkey support for "mark this moment" functionality.
//! Allows users to manually flag important moments during their work.

use chrono::{DateTime, Utc};
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::OnceLock;
use tracing::{debug, error, info, warn};

/// Global channel for hotkey events (set up once at startup)
static HOTKEY_SENDER: OnceLock<Sender<HotkeyTrigger>> = OnceLock::new();

/// Default hotkey: Cmd+Shift+M (macOS) or Ctrl+Shift+M (Windows/Linux)
#[cfg(target_os = "macos")]
const DEFAULT_MODIFIERS: Modifiers = Modifiers::SUPER.union(Modifiers::SHIFT);
#[cfg(not(target_os = "macos"))]
const DEFAULT_MODIFIERS: Modifiers = Modifiers::CONTROL.union(Modifiers::SHIFT);

const DEFAULT_KEY: Code = Code::KeyM;

/// Types of hotkey triggers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HotkeyAction {
    /// Mark the current moment as important
    MarkMoment,
    /// Start/stop a focus session
    ToggleFocus,
    /// Quick note trigger
    QuickNote,
}

impl std::fmt::Display for HotkeyAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HotkeyAction::MarkMoment => write!(f, "mark_moment"),
            HotkeyAction::ToggleFocus => write!(f, "toggle_focus"),
            HotkeyAction::QuickNote => write!(f, "quick_note"),
        }
    }
}

/// A hotkey trigger event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyTrigger {
    /// The action that was triggered
    pub action: HotkeyAction,
    /// When it was triggered
    pub timestamp: DateTime<Utc>,
    /// Optional note or context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Configuration for hotkey system
#[derive(Debug, Clone)]
pub struct HotkeyConfig {
    /// Whether hotkeys are enabled
    pub enabled: bool,
    /// Custom hotkey for mark moment (if None, uses default)
    pub mark_moment_hotkey: Option<HotKey>,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mark_moment_hotkey: None,
        }
    }
}

/// Manages global hotkey registration and events
pub struct HotkeyManager {
    manager: Option<GlobalHotKeyManager>,
    receiver: Option<Receiver<HotkeyTrigger>>,
    registered_hotkeys: Vec<(HotKey, HotkeyAction)>,
}

impl HotkeyManager {
    /// Create a new hotkey manager
    ///
    /// IMPORTANT: On macOS, this must be called from the main thread
    /// before entering the tokio runtime.
    pub fn new(config: HotkeyConfig) -> Self {
        if !config.enabled {
            info!("Hotkey system disabled");
            return Self {
                manager: None,
                receiver: None,
                registered_hotkeys: Vec::new(),
            };
        }

        // Create the manager
        let manager = match GlobalHotKeyManager::new() {
            Ok(m) => m,
            Err(e) => {
                error!("Failed to create hotkey manager: {}", e);
                return Self {
                    manager: None,
                    receiver: None,
                    registered_hotkeys: Vec::new(),
                };
            }
        };

        // Set up channel for events
        let (tx, rx) = mpsc::channel();
        let _ = HOTKEY_SENDER.set(tx);

        // Register default hotkey
        let mark_hotkey = config
            .mark_moment_hotkey
            .unwrap_or_else(|| HotKey::new(Some(DEFAULT_MODIFIERS), DEFAULT_KEY));

        let mut registered = Vec::new();

        if let Err(e) = manager.register(mark_hotkey) {
            warn!("Failed to register mark moment hotkey: {}", e);
        } else {
            info!(
                "Registered hotkey: {:?}+{:?} for mark_moment",
                DEFAULT_MODIFIERS, DEFAULT_KEY
            );
            registered.push((mark_hotkey, HotkeyAction::MarkMoment));
        }

        Self {
            manager: Some(manager),
            receiver: Some(rx),
            registered_hotkeys: registered,
        }
    }

    /// Start listening for hotkey events in a background thread
    ///
    /// This spawns a thread that listens for global hotkey events
    /// and sends them through the channel.
    pub fn start_listener(&self) {
        if self.manager.is_none() {
            return;
        }

        let registered = self.registered_hotkeys.clone();

        std::thread::spawn(move || {
            info!("Hotkey listener thread started");

            // Get the global event receiver
            let receiver = GlobalHotKeyEvent::receiver();

            loop {
                if let Ok(event) = receiver.recv() {
                    debug!("Hotkey event received: {:?}", event);

                    // Find which action this hotkey corresponds to
                    for (hotkey, action) in &registered {
                        if event.id == hotkey.id() {
                            let trigger = HotkeyTrigger {
                                action: action.clone(),
                                timestamp: Utc::now(),
                                note: None,
                            };

                            if let Some(sender) = HOTKEY_SENDER.get() {
                                if let Err(e) = sender.send(trigger) {
                                    error!("Failed to send hotkey trigger: {}", e);
                                }
                            }
                            break;
                        }
                    }
                }
            }
        });
    }

    /// Poll for hotkey triggers (non-blocking)
    pub fn poll_triggers(&self) -> Vec<HotkeyTrigger> {
        let mut triggers = Vec::new();

        if let Some(ref rx) = self.receiver {
            while let Ok(trigger) = rx.try_recv() {
                triggers.push(trigger);
            }
        }

        triggers
    }

    /// Check if hotkey system is available
    pub fn is_available(&self) -> bool {
        self.manager.is_some()
    }

    /// Get the registered hotkeys
    pub fn registered_hotkeys(&self) -> &[(HotKey, HotkeyAction)] {
        &self.registered_hotkeys
    }
}

/// Get a human-readable description of a hotkey
pub fn hotkey_description(hotkey: &HotKey) -> String {
    let mut parts = Vec::new();

    let mods = hotkey.mods;
    if mods.contains(Modifiers::SUPER) {
        #[cfg(target_os = "macos")]
        parts.push("Cmd");
        #[cfg(not(target_os = "macos"))]
        parts.push("Super");
    }
    if mods.contains(Modifiers::CONTROL) {
        parts.push("Ctrl");
    }
    if mods.contains(Modifiers::ALT) {
        #[cfg(target_os = "macos")]
        parts.push("Option");
        #[cfg(not(target_os = "macos"))]
        parts.push("Alt");
    }
    if mods.contains(Modifiers::SHIFT) {
        parts.push("Shift");
    }

    // Add key code
    let key_str = format!("{:?}", hotkey.key);
    parts.push(&key_str);

    parts.join("+")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hotkey_config_default() {
        let config = HotkeyConfig::default();
        assert!(config.enabled);
        assert!(config.mark_moment_hotkey.is_none());
    }

    #[test]
    fn test_hotkey_action_display() {
        assert_eq!(HotkeyAction::MarkMoment.to_string(), "mark_moment");
        assert_eq!(HotkeyAction::ToggleFocus.to_string(), "toggle_focus");
        assert_eq!(HotkeyAction::QuickNote.to_string(), "quick_note");
    }
}
