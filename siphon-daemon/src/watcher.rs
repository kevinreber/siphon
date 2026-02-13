//! File System Watcher
//!
//! Watches project directories for file changes and reports them as events.
//! Uses the notify crate for cross-platform file system watching.

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// File system event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEventData {
    pub action: String,
    pub file_path: String,
    pub file_type: Option<String>,
    pub is_directory: bool,
}

/// Patterns to ignore when watching
const IGNORE_PATTERNS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    "__pycache__",
    ".venv",
    "venv",
    ".idea",
    ".vscode",
    "*.swp",
    "*.swo",
    "*~",
    ".DS_Store",
    "Thumbs.db",
    "*.log",
    "*.tmp",
    "*.temp",
    ".cache",
    "coverage",
    ".next",
    ".nuxt",
];

/// File extensions that indicate source code
const SOURCE_EXTENSIONS: &[&str] = &[
    // Programming languages
    "rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "c", "cpp", "h", "hpp", "rb", "php",
    "swift", "kt", "scala", "lua", "vim", "sh", "bash", "zsh", "fish", "sql", "graphql",
    // Config/data formats
    "yaml", "yml", "json", "toml", "xml", "html", "css", "scss", "sass", "less",
    // Documentation
    "md", "mdx", "txt", "rst", "adoc", // Frontend frameworks
    "vue", "svelte",
];

/// File extensions for creative/design work
const CREATIVE_EXTENSIONS: &[&str] = &[
    // Adobe Creative Suite
    "psd", "ai", "indd", "xd", "aep", "prproj", // Image formats
    "png", "jpg", "jpeg", "gif", "webp", "svg", "ico", "tiff", "tif", "bmp", "raw", "cr2", "nef",
    // Design tools
    "sketch", "fig", "afdesign", "afphoto", "afpub", // 3D modeling
    "blend", "fbx", "obj", "stl", "3mf", "gltf", "glb", "dae", "3ds", "max", "ma", "mb", "c4d",
    // 3D printing / CAD
    "gcode", "step", "stp", "iges", "igs", "dwg", "dxf", "scad", "f3d", "fusion",
    // Video
    "mp4", "mov", "avi", "mkv", "webm", "m4v", "wmv", // Audio
    "mp3", "wav", "flac", "aac", "ogg", "m4a", "aiff",
    // Audio production (DAW projects)
    "als", "flp", "logic", "ptx", "rpp", "cpr", // Documents
    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "pages", "numbers", "key",
];

/// File system watcher configuration
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    pub paths: Vec<PathBuf>,
    pub debounce_ms: u64,
    pub recursive: bool,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            paths: vec![],
            debounce_ms: 500,
            recursive: true,
        }
    }
}

/// File system watcher manager
pub struct FileWatcher {
    config: WatcherConfig,
    watcher: Option<RecommendedWatcher>,
    receiver: Option<Receiver<Result<Event, notify::Error>>>,
    last_events: HashSet<String>,
    last_event_time: Instant,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new(config: WatcherConfig) -> Self {
        Self {
            config,
            watcher: None,
            receiver: None,
            last_events: HashSet::new(),
            last_event_time: Instant::now(),
        }
    }

    /// Start watching configured paths
    pub fn start(&mut self) -> Result<(), notify::Error> {
        let (tx, rx) = mpsc::channel();

        let watcher_config =
            Config::default().with_poll_interval(Duration::from_millis(self.config.debounce_ms));

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            watcher_config,
        )?;

        let mode = if self.config.recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        for path in &self.config.paths {
            if path.exists() {
                info!("Watching path: {:?}", path);
                watcher.watch(path, mode)?;
            } else {
                warn!("Path does not exist, skipping: {:?}", path);
            }
        }

        self.watcher = Some(watcher);
        self.receiver = Some(rx);

        Ok(())
    }

    /// Add a path to watch
    pub fn watch_path(&mut self, path: &Path) -> Result<(), notify::Error> {
        if let Some(ref mut watcher) = self.watcher {
            let mode = if self.config.recursive {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            };
            watcher.watch(path, mode)?;
            info!("Added watch path: {:?}", path);
        }
        Ok(())
    }

    /// Stop watching a path
    pub fn unwatch_path(&mut self, path: &Path) -> Result<(), notify::Error> {
        if let Some(ref mut watcher) = self.watcher {
            watcher.unwatch(path)?;
            info!("Removed watch path: {:?}", path);
        }
        Ok(())
    }

    /// Poll for file events (non-blocking)
    pub fn poll_events(&mut self) -> Vec<FileEventData> {
        let mut events = Vec::new();

        // Reset deduplication cache periodically
        if self.last_event_time.elapsed() > Duration::from_secs(5) {
            self.last_events.clear();
            self.last_event_time = Instant::now();
        }

        if let Some(ref receiver) = self.receiver {
            // Drain all available events
            while let Ok(result) = receiver.try_recv() {
                match result {
                    Ok(event) => {
                        if let Some(file_event) = self.process_event(event) {
                            // Deduplicate events
                            let key = format!("{}:{}", file_event.action, file_event.file_path);
                            if !self.last_events.contains(&key) {
                                self.last_events.insert(key);
                                events.push(file_event);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("File watcher error: {:?}", e);
                    }
                }
            }
        }

        events
    }

    /// Process a notify event into our event format
    fn process_event(&self, event: Event) -> Option<FileEventData> {
        // Skip events with no paths
        if event.paths.is_empty() {
            return None;
        }

        let path = &event.paths[0];
        let path_str = path.to_string_lossy().to_string();

        // Skip ignored paths
        if self.should_ignore(&path_str) {
            debug!("Ignoring path: {}", path_str);
            return None;
        }

        let action = match event.kind {
            EventKind::Create(_) => "file_create",
            EventKind::Modify(_) => "file_modify",
            EventKind::Remove(_) => "file_remove",
            EventKind::Access(_) => return None, // Skip access events
            EventKind::Other => return None,
            EventKind::Any => return None,
        };

        let is_directory = path.is_dir();
        let file_type = if is_directory {
            Some("directory".to_string())
        } else {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|s| s.to_string())
        };

        Some(FileEventData {
            action: action.to_string(),
            file_path: path_str,
            file_type,
            is_directory,
        })
    }

    /// Check if a path should be ignored
    fn should_ignore(&self, path: &str) -> bool {
        for pattern in IGNORE_PATTERNS {
            if let Some(ext) = pattern.strip_prefix('*') {
                // Extension pattern
                if path.ends_with(ext) {
                    return true;
                }
            } else {
                // Directory/file name pattern
                if path.contains(&format!("/{}/", pattern))
                    || path.contains(&format!("\\{}\\", pattern))
                    || path.ends_with(&format!("/{}", pattern))
                    || path.ends_with(&format!("\\{}", pattern))
                {
                    return true;
                }
            }
        }
        false
    }

    /// Check if a file is a source code file
    pub fn is_source_file(path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            if let Some(ext_str) = ext.to_str() {
                return SOURCE_EXTENSIONS.contains(&ext_str.to_lowercase().as_str());
            }
        }
        false
    }

    /// Check if a file is a creative/design file
    pub fn is_creative_file(path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            if let Some(ext_str) = ext.to_str() {
                return CREATIVE_EXTENSIONS.contains(&ext_str.to_lowercase().as_str());
            }
        }
        false
    }

    /// Check if a file is a tracked file (source or creative)
    pub fn is_tracked_file(path: &Path) -> bool {
        Self::is_source_file(path) || Self::is_creative_file(path)
    }
}

/// Detect project root from a file path
pub fn detect_project_root(path: &Path) -> Option<PathBuf> {
    let project_markers = [
        ".git",
        "package.json",
        "Cargo.toml",
        "go.mod",
        "pyproject.toml",
        "setup.py",
        "pom.xml",
        "build.gradle",
        "Makefile",
        "CMakeLists.txt",
    ];

    let mut current = Some(path);
    while let Some(p) = current {
        for marker in &project_markers {
            if p.join(marker).exists() {
                return Some(p.to_path_buf());
            }
        }
        current = p.parent();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_ignore() {
        let watcher = FileWatcher::new(WatcherConfig::default());

        assert!(watcher.should_ignore("/project/node_modules/foo.js"));
        assert!(watcher.should_ignore("/project/.git/config"));
        assert!(watcher.should_ignore("/project/target/debug/main"));
        assert!(watcher.should_ignore("/project/file.swp"));
        assert!(!watcher.should_ignore("/project/src/main.rs"));
        assert!(!watcher.should_ignore("/project/index.ts"));
    }

    #[test]
    fn test_is_source_file() {
        assert!(FileWatcher::is_source_file(Path::new("main.rs")));
        assert!(FileWatcher::is_source_file(Path::new("index.tsx")));
        assert!(FileWatcher::is_source_file(Path::new("app.py")));
        assert!(!FileWatcher::is_source_file(Path::new("image.png"))); // PNG is creative, not source
        assert!(!FileWatcher::is_source_file(Path::new("data.bin")));
    }

    #[test]
    fn test_is_creative_file() {
        // Adobe files
        assert!(FileWatcher::is_creative_file(Path::new("design.psd")));
        assert!(FileWatcher::is_creative_file(Path::new("logo.ai")));
        // 3D files
        assert!(FileWatcher::is_creative_file(Path::new("model.blend")));
        assert!(FileWatcher::is_creative_file(Path::new("print.stl")));
        assert!(FileWatcher::is_creative_file(Path::new("output.gcode")));
        // Images
        assert!(FileWatcher::is_creative_file(Path::new("photo.png")));
        assert!(FileWatcher::is_creative_file(Path::new("icon.svg")));
        // Audio/Video
        assert!(FileWatcher::is_creative_file(Path::new("video.mp4")));
        assert!(FileWatcher::is_creative_file(Path::new("song.mp3")));
        // DAW projects
        assert!(FileWatcher::is_creative_file(Path::new("track.als")));
        // Not creative
        assert!(!FileWatcher::is_creative_file(Path::new("main.rs")));
        assert!(!FileWatcher::is_creative_file(Path::new("data.bin")));
    }

    #[test]
    fn test_is_tracked_file() {
        // Source files are tracked
        assert!(FileWatcher::is_tracked_file(Path::new("main.rs")));
        assert!(FileWatcher::is_tracked_file(Path::new("app.py")));
        // Creative files are tracked
        assert!(FileWatcher::is_tracked_file(Path::new("design.psd")));
        assert!(FileWatcher::is_tracked_file(Path::new("model.blend")));
        assert!(FileWatcher::is_tracked_file(Path::new("photo.png")));
        // Unknown files are not tracked
        assert!(!FileWatcher::is_tracked_file(Path::new("data.bin")));
        assert!(!FileWatcher::is_tracked_file(Path::new("file.xyz")));
    }
}
