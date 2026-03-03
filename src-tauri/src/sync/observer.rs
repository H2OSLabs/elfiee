//! Agent Sync Observer
//!
//! Autonomous observer that manages session sync lifecycle by subscribing to
//! agent state change events. This decouples sync logic from agent commands —
//! agent.rs publishes facts ("agent X is now enabled"), and this observer
//! reacts by starting or stopping session sync.
//!
//! ## Event Flow
//!
//! ```text
//! file.rs::open_file()   → FileOpened   → observer scans enabled agents → start_sync
//! agent.rs::do_*()       → AgentStateChanged → observer starts/stops sync
//! file.rs::close_file()  → FileClosing  → observer shuts down sync manager
//! ```

use crate::extensions::agent::{AgentContents, AgentStatus};
use crate::state::AppState;
use crate::sync::SessionSyncManager;
use std::collections::HashMap;
use tokio::sync::broadcast;

/// Events that drive the session sync lifecycle.
///
/// Published by agent commands and file commands; consumed by `AgentSyncObserver`.
#[derive(Debug, Clone)]
pub enum AgentSyncEvent {
    /// A file was opened — observer should scan for enabled agents and start sync.
    FileOpened { file_id: String },
    /// A file is being closed — observer should shut down all sync for this file.
    FileClosing { file_id: String },
    /// An agent's status changed — observer should start or stop sync.
    AgentStateChanged {
        file_id: String,
        agent_block_id: String,
        new_status: AgentStatus,
        config_dir: String,
    },
}

/// Per-file sync state owned by the observer.
struct FileSyncState {
    /// The session sync manager for this file.
    sync_manager: SessionSyncManager,
    /// Tracks which agents have active sync: agent_block_id → config_dir.
    active_agents: HashMap<String, String>,
    /// Whether offsets have been restored (must happen before any start_sync).
    offsets_restored: bool,
}

/// Autonomous observer that manages session sync based on agent lifecycle events.
///
/// One instance per application, spawned at startup. Owns all `SessionSyncManager`
/// instances (one per open file with agents).
pub struct AgentSyncObserver {
    app_state: AppState,
    /// Per-file sync state: file_id → FileSyncState.
    file_states: HashMap<String, FileSyncState>,
}

impl AgentSyncObserver {
    fn new(app_state: AppState) -> Self {
        Self {
            app_state,
            file_states: HashMap::new(),
        }
    }

    /// Spawn the observer as a background async task.
    ///
    /// Uses `tauri::async_runtime::spawn` to ensure compatibility with Tauri's
    /// setup hook (which may not have a bare Tokio runtime context).
    /// Subscribes to `app_state.agent_sync_tx` and processes events sequentially.
    pub fn spawn(app_state: AppState) -> tauri::async_runtime::JoinHandle<()> {
        let mut rx = app_state.agent_sync_tx.subscribe();
        let mut observer = Self::new(app_state);

        tauri::async_runtime::spawn(async move {
            log::info!("AgentSyncObserver started");

            loop {
                match rx.recv().await {
                    Ok(event) => observer.handle_event(event).await,
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        log::warn!("AgentSyncObserver lagged by {} events", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        log::info!("AgentSyncObserver channel closed, shutting down");
                        break;
                    }
                }
            }

            observer.shutdown_all().await;
        })
    }

    async fn handle_event(&mut self, event: AgentSyncEvent) {
        match event {
            AgentSyncEvent::FileOpened { file_id } => {
                self.handle_file_opened(&file_id).await;
            }
            AgentSyncEvent::FileClosing { file_id } => {
                self.handle_file_closing(&file_id).await;
            }
            AgentSyncEvent::AgentStateChanged {
                file_id,
                agent_block_id,
                new_status,
                config_dir,
            } => {
                self.handle_agent_state_changed(&file_id, &agent_block_id, new_status, &config_dir)
                    .await;
            }
        }
    }

    /// Handle file opened: create sync manager, restore offsets, start sync for enabled agents.
    async fn handle_file_opened(&mut self, file_id: &str) {
        let handle = match self.app_state.engine_manager.get_engine(file_id) {
            Some(h) => h,
            None => {
                log::warn!(
                    "AgentSyncObserver: engine not found for file_id={}",
                    file_id
                );
                return;
            }
        };

        // Create SessionSyncManager for this file
        let sync_mgr = match SessionSyncManager::new(self.app_state.clone()) {
            Ok(mgr) => {
                log::info!(
                    "AgentSyncObserver: SessionSyncManager created for file_id={}",
                    file_id
                );
                mgr
            }
            Err(e) => {
                log::warn!(
                    "AgentSyncObserver: failed to create SessionSyncManager for {}: {}",
                    file_id,
                    e
                );
                return;
            }
        };

        let mut file_state = FileSyncState {
            sync_manager: sync_mgr,
            active_agents: HashMap::new(),
            offsets_restored: false,
        };

        let blocks = handle.get_all_blocks().await;

        // Restore offsets ONCE before any start_sync calls
        if let Some(elf_id) = blocks
            .iter()
            .find(|(_, b)| b.name == ".elf" && b.block_type == "directory")
            .map(|(id, _)| id.clone())
        {
            if let Err(e) = file_state
                .sync_manager
                .restore_offsets(&handle, &elf_id)
                .await
            {
                log::warn!(
                    "AgentSyncObserver: failed to restore offsets for {}: {}",
                    file_id,
                    e
                );
            }
        }
        file_state.offsets_restored = true;

        // Start sync for all enabled agents
        for block in blocks.values() {
            if block.block_type != "agent" {
                continue;
            }
            let contents: AgentContents = match serde_json::from_value(block.contents.clone()) {
                Ok(c) => c,
                Err(_) => continue,
            };
            if contents.status != AgentStatus::Enabled {
                continue;
            }
            if let Err(e) = file_state
                .sync_manager
                .start_sync(file_id, &block.block_id, &contents.config_dir)
                .await
            {
                log::warn!(
                    "AgentSyncObserver: failed to start sync for agent {}: {}",
                    block.block_id,
                    e
                );
            } else {
                file_state
                    .active_agents
                    .insert(block.block_id.clone(), contents.config_dir.clone());
                log::info!(
                    "AgentSyncObserver: sync started for agent {}",
                    block.block_id
                );
            }
        }

        self.file_states.insert(file_id.to_string(), file_state);
    }

    /// Handle file closing: stop all syncs, shutdown sync manager.
    async fn handle_file_closing(&mut self, file_id: &str) {
        if let Some(mut file_state) = self.file_states.remove(file_id) {
            file_state.sync_manager.shutdown().await;
            log::info!(
                "AgentSyncObserver: sync manager shut down for file_id={}",
                file_id
            );
        }
    }

    /// Handle agent state change: start or stop sync.
    async fn handle_agent_state_changed(
        &mut self,
        file_id: &str,
        agent_block_id: &str,
        new_status: AgentStatus,
        config_dir: &str,
    ) {
        match new_status {
            AgentStatus::Enabled => {
                // Ensure file state exists (lazy init if FileOpened hasn't been received yet)
                self.ensure_file_state(file_id).await;

                if let Some(file_state) = self.file_states.get_mut(file_id) {
                    // Restore offsets if not done yet
                    if !file_state.offsets_restored {
                        Self::restore_offsets_for_file(&self.app_state, file_id, file_state).await;
                    }

                    if let Err(e) = file_state
                        .sync_manager
                        .start_sync(file_id, agent_block_id, config_dir)
                        .await
                    {
                        log::warn!(
                            "AgentSyncObserver: failed to start sync for agent {}: {}",
                            agent_block_id,
                            e
                        );
                    } else {
                        file_state
                            .active_agents
                            .insert(agent_block_id.to_string(), config_dir.to_string());
                        log::info!(
                            "AgentSyncObserver: sync started for agent {}",
                            agent_block_id
                        );
                    }
                }
            }
            AgentStatus::Disabled => {
                if let Some(file_state) = self.file_states.get_mut(file_id) {
                    if let Err(e) = file_state.sync_manager.stop_sync(agent_block_id).await {
                        log::warn!(
                            "AgentSyncObserver: failed to stop sync for agent {}: {}",
                            agent_block_id,
                            e
                        );
                    }
                    file_state.active_agents.remove(agent_block_id);
                    log::info!(
                        "AgentSyncObserver: sync stopped for agent {}",
                        agent_block_id
                    );
                }
            }
        }
    }

    /// Ensure a FileSyncState exists for the given file_id (lazy initialization).
    async fn ensure_file_state(&mut self, file_id: &str) {
        if self.file_states.contains_key(file_id) {
            return;
        }
        match SessionSyncManager::new(self.app_state.clone()) {
            Ok(mgr) => {
                log::info!(
                    "AgentSyncObserver: lazy-initialized SessionSyncManager for file_id={}",
                    file_id
                );
                self.file_states.insert(
                    file_id.to_string(),
                    FileSyncState {
                        sync_manager: mgr,
                        active_agents: HashMap::new(),
                        offsets_restored: false,
                    },
                );
            }
            Err(e) => {
                log::warn!(
                    "AgentSyncObserver: failed to create SessionSyncManager for {}: {}",
                    file_id,
                    e
                );
            }
        }
    }

    /// Restore parser offsets from existing session blocks.
    async fn restore_offsets_for_file(
        app_state: &AppState,
        file_id: &str,
        file_state: &mut FileSyncState,
    ) {
        let handle = match app_state.engine_manager.get_engine(file_id) {
            Some(h) => h,
            None => return,
        };
        let blocks = handle.get_all_blocks().await;
        if let Some(elf_id) = blocks
            .iter()
            .find(|(_, b)| b.name == ".elf" && b.block_type == "directory")
            .map(|(id, _)| id.clone())
        {
            if let Err(e) = file_state
                .sync_manager
                .restore_offsets(&handle, &elf_id)
                .await
            {
                log::warn!(
                    "AgentSyncObserver: failed to restore offsets for {}: {}",
                    file_id,
                    e
                );
            }
        }
        file_state.offsets_restored = true;
    }

    /// Shutdown all file states (called when the observer loop exits).
    async fn shutdown_all(&mut self) {
        for (file_id, mut file_state) in self.file_states.drain() {
            file_state.sync_manager.shutdown().await;
            log::info!(
                "AgentSyncObserver: sync manager shut down for file_id={} (observer shutdown)",
                file_id
            );
        }
    }
}
