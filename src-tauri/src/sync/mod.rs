//! Session Sync Module
//!
//! Synchronizes Claude Code session data (JSONL files) from external project
//! directories into Elfiee's `.elf/` storage as readable Markdown Blocks.
//!
//! ## Architecture
//!
//! ```text
//! SessionSyncManager (top-level coordinator)
//! ├── SessionWatcher   — watches session directories via `notify` crate
//! ├── SessionParser    — incremental JSONL parsing + Markdown conversion
//! └── SessionWriter    — writes Markdown Blocks via EngineHandle
//! ```
//!
//! ## Lifecycle
//!
//! - `start_sync()` — called when an Agent is enabled
//! - `stop_sync()`  — called when an Agent is disabled
//! - `shutdown()`   — called when a file is closed

pub mod observer;
pub mod parser;
pub mod session_path;
pub mod watcher;
pub mod writer;

#[cfg(test)]
mod tests;

use crate::state::AppState;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use watcher::{FileChangeEvent, SessionWatcher, SyncStatus};

/// Top-level session sync manager.
///
/// Coordinates the Watcher → Parser → Writer pipeline.
/// One instance per open `.elf` file.
pub struct SessionSyncManager {
    watcher: Arc<Mutex<SessionWatcher>>,
    parser: Arc<Mutex<parser::SessionParser>>,
    writer: Arc<Mutex<writer::SessionWriter>>,
    /// Handle to the background sync task
    sync_task: Option<tokio::task::JoinHandle<()>>,
}

impl SessionSyncManager {
    /// Create a new SessionSyncManager and start the background event loop.
    pub fn new(app_state: AppState) -> Result<Self, String> {
        let (watcher, event_rx) = SessionWatcher::new()?;

        let parser = Arc::new(Mutex::new(parser::SessionParser::new()));
        let writer = Arc::new(Mutex::new(writer::SessionWriter::new()));
        let watcher = Arc::new(Mutex::new(watcher));

        // Start the background event processing loop
        let parser_clone = parser.clone();
        let writer_clone = writer.clone();
        let sync_task = tokio::spawn(sync_event_loop(
            event_rx,
            parser_clone,
            writer_clone,
            app_state,
        ));

        Ok(Self {
            watcher,
            parser,
            writer,
            sync_task: Some(sync_task),
        })
    }

    /// Start session sync for an Agent.
    ///
    /// Computes the session directory from the Agent's `config_dir`, registers
    /// the directory with the watcher, and scans existing files.
    pub async fn start_sync(
        &self,
        file_id: &str,
        agent_block_id: &str,
        config_dir: &str,
    ) -> Result<(), String> {
        let mut watcher = self.watcher.lock().await;
        watcher.watch_agent(file_id, agent_block_id, config_dir)
    }

    /// Stop session sync for an Agent.
    pub async fn stop_sync(&self, agent_block_id: &str) -> Result<(), String> {
        let mut watcher = self.watcher.lock().await;
        watcher.unwatch_agent(agent_block_id)
    }

    /// Get sync status for an Agent.
    pub async fn get_status(&self, agent_block_id: &str) -> SyncStatus {
        let watcher = self.watcher.lock().await;
        watcher.get_status(agent_block_id)
    }

    /// Restore parser offsets from previously synced blocks.
    ///
    /// Called after `start_sync` when re-opening a file to avoid re-processing
    /// already-synced content.
    pub async fn restore_offsets(
        &self,
        handle: &crate::engine::EngineHandle,
        elf_block_id: &str,
    ) -> Result<(), String> {
        let offset_map = {
            let mut writer = self.writer.lock().await;
            writer.load_existing_sessions(handle, elf_block_id).await?
        };

        let mut parser = self.parser.lock().await;
        for (source_file, offset) in offset_map {
            parser.set_offset(std::path::Path::new(&source_file), offset);
        }

        Ok(())
    }

    /// Shut down the sync manager and all watchers.
    pub async fn shutdown(&mut self) {
        if let Some(task) = self.sync_task.take() {
            task.abort();
        }
        log::info!("Session sync manager shut down");
    }
}

/// Background event loop: receives FileChangeEvents and processes them
/// through the Parser → Writer pipeline.
async fn sync_event_loop(
    mut event_rx: mpsc::UnboundedReceiver<FileChangeEvent>,
    parser: Arc<Mutex<parser::SessionParser>>,
    writer: Arc<Mutex<writer::SessionWriter>>,
    app_state: AppState,
) {
    log::info!("Session sync event loop started");

    while let Some(event) = event_rx.recv().await {
        // Note: Debouncing is handled by notify-debouncer-mini in the watcher (100ms).

        // 1. Parse incrementally (offloaded to blocking thread pool to avoid
        //    blocking the tokio runtime with synchronous file I/O)
        let doc = {
            let parser_clone = parser.clone();
            let path = event.path.clone();
            let result = tokio::task::spawn_blocking(move || {
                let mut parser = parser_clone.blocking_lock();
                parser.parse_incremental(&path)
            })
            .await;

            match result {
                Ok(Ok(Some(doc))) => doc,
                Ok(Ok(None)) => continue,
                Ok(Err(e)) => {
                    log::error!("Parse error for {:?}: {}", event.path, e);
                    continue;
                }
                Err(e) => {
                    log::error!("Parse task panicked for {:?}: {}", event.path, e);
                    continue;
                }
            }
        };

        // 2. Get engine handle
        let handle = match app_state.engine_manager.get_engine(&event.file_id) {
            Some(h) => h,
            None => {
                log::warn!("Engine not found for file_id: {}", event.file_id);
                continue;
            }
        };

        // 3. Get editor_id (use active editor or fallback to any available editor)
        let editor_id = match app_state.get_active_editor(&event.file_id) {
            Some(id) => id,
            None => {
                let editors = handle.get_all_editors().await;
                match editors.keys().next() {
                    Some(id) => id.clone(),
                    None => {
                        log::warn!("No editor available for file_id: {}", event.file_id);
                        continue;
                    }
                }
            }
        };

        // 4. Find .elf/ directory block
        let elf_block_id = match find_elf_block_id(&handle).await {
            Some(id) => id,
            None => {
                log::warn!(
                    "Cannot find .elf/ directory block for file_id: {}",
                    event.file_id
                );
                continue;
            }
        };

        // 5. Derive project name
        let project_name = session_path::extract_project_name_from_config(&event.config_dir);

        // 6. Get source offset
        let source_offset = {
            let p = parser.lock().await;
            p.get_offset(&event.path)
        };

        // 7. Write to Markdown Block
        let mut writer = writer.lock().await;
        if let Err(e) = writer
            .write_session(
                &handle,
                &editor_id,
                &elf_block_id,
                &project_name,
                &event.config_dir,
                doc,
                &event.path.to_string_lossy(),
                source_offset,
            )
            .await
        {
            log::error!("Session write error: {}", e);
        }
    }

    log::info!("Session sync event loop ended");
}

/// Find the .elf/ directory block ID from the engine.
async fn find_elf_block_id(handle: &crate::engine::EngineHandle) -> Option<String> {
    let blocks = handle.get_all_blocks().await;
    for (id, block) in &blocks {
        if block.name == ".elf" && block.block_type == "directory" {
            return Some(id.clone());
        }
    }
    None
}
