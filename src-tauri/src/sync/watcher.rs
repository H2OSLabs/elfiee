//! JSONL file system watcher
//!
//! Uses the `notify` crate with `notify-debouncer-mini` to watch Claude Code
//! session directories for new/modified `.jsonl` files and forwards change
//! events to the sync pipeline.
//!
//! ## Debouncing
//!
//! File changes are debounced at 100ms to coalesce rapid writes from Claude Code.
//! This is handled in the watcher itself using `notify-debouncer-mini`.

use crate::sync::session_path;
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEvent, DebouncedEventKind, Debouncer};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use tokio::sync::mpsc;

/// Session sync status for an Agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub enum SyncStatus {
    /// Actively watching the session directory
    Watching,
    /// Waiting for session directory to be created (watching parent directory)
    WaitingForDirectory,
    /// Stopped (manually stopped)
    Stopped,
    /// Error state
    Error(String),
}

/// A file change event forwarded from the watcher to the parser/writer pipeline.
#[derive(Debug)]
pub struct FileChangeEvent {
    /// Path to the changed `.jsonl` file
    pub path: PathBuf,
    /// Agent Block ID that owns this session directory
    pub agent_block_id: String,
    /// The `.elf` file ID
    pub file_id: String,
    /// Agent's config_dir (used to derive project name)
    pub config_dir: String,
}

/// Metadata for a watched directory (maps session_dir → agent info).
///
/// Shared between the watcher and the notify bridge thread via Arc<Mutex>.
pub(crate) struct WatchEntry {
    pub file_id: String,
    pub agent_block_id: String,
    pub config_dir: String,
}

/// Pending agent waiting for its session directory to be created.
///
/// When the session directory doesn't exist yet, we watch the parent directory
/// (`~/.claude/projects/`) and wait for the target subdirectory to appear.
#[derive(Clone)]
pub(crate) struct PendingAgent {
    pub file_id: String,
    pub agent_block_id: String,
    pub config_dir: String,
    /// The expected session directory path (may not exist yet)
    pub expected_session_dir: PathBuf,
}

/// Session file watcher using the `notify` crate with debouncing.
///
/// Watches one or more session directories (one per Agent) and sends
/// `FileChangeEvent`s to the sync pipeline via a tokio mpsc channel.
/// File changes are debounced at 100ms to coalesce rapid writes.
///
/// ## Directory Creation Handling
///
/// When the session directory doesn't exist yet (user hasn't used Claude Code
/// in that project), we watch the parent directory (`~/.claude/projects/`)
/// and automatically start watching the subdirectory when it's created.
pub struct SessionWatcher {
    /// The underlying debounced watcher (100ms debounce)
    watcher: Debouncer<notify::RecommendedWatcher>,
    /// Event sender for the sync pipeline
    event_tx: mpsc::UnboundedSender<FileChangeEvent>,
    /// Shared watched directories map (used by both watcher methods and bridge thread)
    watched_dirs: Arc<StdMutex<HashMap<PathBuf, WatchEntry>>>,
    /// Pending agents waiting for their session directories to be created
    pending_agents: Arc<StdMutex<Vec<PendingAgent>>>,
    /// Whether we're watching the parent directory (~/.claude/projects/)
    watching_parent: Arc<StdMutex<bool>>,
    /// Per-agent sync status
    status_map: HashMap<String, SyncStatus>,
}

impl SessionWatcher {
    /// Create a new SessionWatcher with 100ms debouncing.
    ///
    /// Returns `(watcher, event_receiver)` — the receiver should be consumed
    /// by the sync event loop.
    pub fn new() -> Result<(Self, mpsc::UnboundedReceiver<FileChangeEvent>), String> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (notify_tx, notify_rx) = std_mpsc::channel::<Vec<DebouncedEvent>>();

        // Shared state between watcher methods and the bridge thread
        let watched_dirs: Arc<StdMutex<HashMap<PathBuf, WatchEntry>>> =
            Arc::new(StdMutex::new(HashMap::new()));
        let pending_agents: Arc<StdMutex<Vec<PendingAgent>>> = Arc::new(StdMutex::new(Vec::new()));
        let watching_parent: Arc<StdMutex<bool>> = Arc::new(StdMutex::new(false));

        // Create debounced watcher with 100ms debounce time
        let watcher = new_debouncer(
            Duration::from_millis(100),
            move |res: Result<Vec<DebouncedEvent>, notify::Error>| {
                if let Ok(events) = res {
                    let _ = notify_tx.send(events);
                }
            },
        )
        .map_err(|e| format!("Failed to create debounced file watcher: {}", e))?;

        // Bridge thread: std_mpsc → tokio mpsc
        // The notify callback runs on an OS thread, so we bridge to the async world.
        let event_tx_bridge = event_tx.clone();
        let watched_dirs_bridge = watched_dirs.clone();
        let pending_agents_bridge = pending_agents.clone();

        std::thread::Builder::new()
            .name("session-sync-bridge".into())
            .spawn(move || {
                while let Ok(events) = notify_rx.recv() {
                    for event in events {
                        // Only process Any events (debouncer merges Create/Modify/etc.)
                        if !matches!(event.kind, DebouncedEventKind::Any) {
                            continue;
                        }

                        let path = &event.path;

                        // Check if this is a new directory that matches a pending agent
                        if path.is_dir() {
                            let mut pending = pending_agents_bridge.lock().unwrap();
                            let mut to_activate: Vec<PendingAgent> = Vec::new();

                            pending.retain(|agent| {
                                // Check if the created directory matches expected session dir
                                // Also check case-insensitive match for Windows
                                let matches = path == &agent.expected_session_dir
                                    || path.to_string_lossy().eq_ignore_ascii_case(
                                        &agent.expected_session_dir.to_string_lossy(),
                                    );

                                if matches {
                                    log::info!(
                                        "Session directory created: {} (agent {})",
                                        path.display(),
                                        agent.agent_block_id
                                    );
                                    to_activate.push(agent.clone());
                                    false // Remove from pending
                                } else {
                                    true // Keep in pending
                                }
                            });

                            // For activated agents, add to watched_dirs and scan for existing files
                            for agent in to_activate {
                                let mut dirs = watched_dirs_bridge.lock().unwrap();
                                dirs.insert(
                                    path.clone(),
                                    WatchEntry {
                                        file_id: agent.file_id.clone(),
                                        agent_block_id: agent.agent_block_id.clone(),
                                        config_dir: agent.config_dir.clone(),
                                    },
                                );

                                // Scan for existing .jsonl files
                                if let Ok(entries) = std::fs::read_dir(path) {
                                    for entry in entries.flatten() {
                                        let file_path = entry.path();
                                        if file_path.is_file() {
                                            if let Some(ext) = file_path.extension() {
                                                if ext == "jsonl" {
                                                    let _ = event_tx_bridge.send(FileChangeEvent {
                                                        path: file_path,
                                                        agent_block_id: agent
                                                            .agent_block_id
                                                            .clone(),
                                                        file_id: agent.file_id.clone(),
                                                        config_dir: agent.config_dir.clone(),
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            continue;
                        }

                        // Handle .jsonl file changes
                        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                            continue;
                        }

                        // Skip temp/hidden files
                        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        if filename.starts_with('.')
                            || filename.ends_with(".tmp")
                            || filename.ends_with(".swp")
                        {
                            continue;
                        }

                        // Find the matching watched directory
                        if let Ok(dirs) = watched_dirs_bridge.lock() {
                            for (dir, entry) in dirs.iter() {
                                if path.starts_with(dir) {
                                    let _ = event_tx_bridge.send(FileChangeEvent {
                                        path: path.clone(),
                                        agent_block_id: entry.agent_block_id.clone(),
                                        file_id: entry.file_id.clone(),
                                        config_dir: entry.config_dir.clone(),
                                    });
                                    break;
                                }
                            }
                        }
                    }
                }
            })
            .map_err(|e| format!("Failed to spawn bridge thread: {}", e))?;

        Ok((
            Self {
                watcher,
                event_tx,
                watched_dirs,
                pending_agents,
                watching_parent,
                status_map: HashMap::new(),
            },
            event_rx,
        ))
    }

    /// Start watching session directory for an Agent.
    ///
    /// If the session directory exists:
    /// 1. Registers the directory with `notify`
    /// 2. Scans existing `.jsonl` files and sends initial events
    /// 3. Status becomes `Watching`
    ///
    /// If the session directory doesn't exist yet:
    /// 1. Watches the parent directory (`~/.claude/projects/`)
    /// 2. Adds agent to pending list
    /// 3. Status becomes `WaitingForDirectory`
    /// 4. When the directory is created, automatically starts watching
    pub fn watch_agent(
        &mut self,
        file_id: &str,
        agent_block_id: &str,
        config_dir: &str,
    ) -> Result<(), String> {
        // Compute session directory (with case-insensitive fallback)
        let session_dir = session_path::find_session_dir_from_config(config_dir)
            .or_else(|| session_path::compute_session_dir_from_config(config_dir).ok());

        let session_dir = match session_dir {
            Some(dir) if dir.is_dir() => dir,
            Some(dir) => {
                // Directory doesn't exist yet — watch parent and wait
                log::info!(
                    "Session directory does not exist yet: {} (agent {}). Watching parent directory.",
                    dir.display(),
                    agent_block_id
                );

                // Add to pending agents
                {
                    let mut pending = self.pending_agents.lock().unwrap();
                    pending.push(PendingAgent {
                        file_id: file_id.to_string(),
                        agent_block_id: agent_block_id.to_string(),
                        config_dir: config_dir.to_string(),
                        expected_session_dir: dir.clone(),
                    });
                }

                // Watch parent directory (~/.claude/projects/) if not already watching
                self.ensure_watching_parent()?;

                self.status_map
                    .insert(agent_block_id.to_string(), SyncStatus::WaitingForDirectory);
                return Ok(());
            }
            None => {
                log::warn!(
                    "Cannot compute session directory for config_dir: {}",
                    config_dir
                );
                self.status_map.insert(
                    agent_block_id.to_string(),
                    SyncStatus::Error("Cannot compute session directory".to_string()),
                );
                return Ok(());
            }
        };

        // Register with notify (non-recursive — we only care about top-level .jsonl files)
        self.watcher
            .watcher()
            .watch(&session_dir, RecursiveMode::NonRecursive)
            .map_err(|e| format!("Failed to watch {}: {}", session_dir.display(), e))?;

        // Insert into shared watched_dirs
        {
            let mut dirs = self.watched_dirs.lock().unwrap();
            dirs.insert(
                session_dir.clone(),
                WatchEntry {
                    file_id: file_id.to_string(),
                    agent_block_id: agent_block_id.to_string(),
                    config_dir: config_dir.to_string(),
                },
            );
        }

        self.status_map
            .insert(agent_block_id.to_string(), SyncStatus::Watching);

        // Initial scan: send events for all existing .jsonl files
        if let Ok(entries) = std::fs::read_dir(&session_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "jsonl" {
                            let _ = self.event_tx.send(FileChangeEvent {
                                path: path.clone(),
                                agent_block_id: agent_block_id.to_string(),
                                file_id: file_id.to_string(),
                                config_dir: config_dir.to_string(),
                            });
                        }
                    }
                }
            }
        }

        log::info!(
            "Session watcher started for agent {} at {}",
            agent_block_id,
            session_dir.display()
        );

        Ok(())
    }

    /// Ensure we're watching the parent directory (~/.claude/projects/).
    ///
    /// This is used to detect when new session directories are created.
    fn ensure_watching_parent(&mut self) -> Result<(), String> {
        let mut watching = self.watching_parent.lock().unwrap();
        if *watching {
            return Ok(());
        }

        let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
        let projects_dir = home.join(".claude").join("projects");

        // Create the projects directory if it doesn't exist
        if !projects_dir.exists() {
            std::fs::create_dir_all(&projects_dir).map_err(|e| {
                format!(
                    "Failed to create projects directory {}: {}",
                    projects_dir.display(),
                    e
                )
            })?;
        }

        // Watch the projects directory (recursive to catch new subdirectories)
        self.watcher
            .watcher()
            .watch(&projects_dir, RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch {}: {}", projects_dir.display(), e))?;

        log::info!(
            "Watching parent directory for new session directories: {}",
            projects_dir.display()
        );

        *watching = true;
        Ok(())
    }

    /// Stop watching session directory for an Agent.
    ///
    /// Also removes the agent from pending list if it was waiting for directory creation.
    pub fn unwatch_agent(&mut self, agent_block_id: &str) -> Result<(), String> {
        // Remove from watched directories
        let dir_to_remove: Option<PathBuf> = {
            let dirs = self.watched_dirs.lock().unwrap();
            dirs.iter()
                .find(|(_, entry)| entry.agent_block_id == agent_block_id)
                .map(|(dir, _)| dir.clone())
        };

        if let Some(dir) = dir_to_remove {
            let _ = self.watcher.watcher().unwatch(&dir);
            let mut dirs = self.watched_dirs.lock().unwrap();
            dirs.remove(&dir);
        }

        // Remove from pending agents (if waiting for directory creation)
        {
            let mut pending = self.pending_agents.lock().unwrap();
            pending.retain(|agent| agent.agent_block_id != agent_block_id);
        }

        self.status_map
            .insert(agent_block_id.to_string(), SyncStatus::Stopped);

        log::info!("Session watcher stopped for agent {}", agent_block_id);
        Ok(())
    }

    /// Get sync status for an Agent.
    ///
    /// Status is derived dynamically:
    /// - If in watched_dirs: Watching
    /// - If in pending_agents: WaitingForDirectory
    /// - Otherwise: use status_map (Stopped or Error)
    pub fn get_status(&self, agent_block_id: &str) -> SyncStatus {
        // Check if actively watching
        {
            let dirs = self.watched_dirs.lock().unwrap();
            for entry in dirs.values() {
                if entry.agent_block_id == agent_block_id {
                    return SyncStatus::Watching;
                }
            }
        }

        // Check if waiting for directory
        {
            let pending = self.pending_agents.lock().unwrap();
            for agent in pending.iter() {
                if agent.agent_block_id == agent_block_id {
                    return SyncStatus::WaitingForDirectory;
                }
            }
        }

        // Fall back to stored status (Stopped or Error)
        self.status_map
            .get(agent_block_id)
            .cloned()
            .unwrap_or(SyncStatus::Stopped)
    }

    /// Get sync status for all Agents.
    ///
    /// Combines status from watched_dirs, pending_agents, and status_map.
    pub fn get_all_status(&self) -> HashMap<String, SyncStatus> {
        let mut result = self.status_map.clone();

        // Add/update watching agents
        {
            let dirs = self.watched_dirs.lock().unwrap();
            for entry in dirs.values() {
                result.insert(entry.agent_block_id.clone(), SyncStatus::Watching);
            }
        }

        // Add/update pending agents
        {
            let pending = self.pending_agents.lock().unwrap();
            for agent in pending.iter() {
                result.insert(
                    agent.agent_block_id.clone(),
                    SyncStatus::WaitingForDirectory,
                );
            }
        }

        result
    }
}
