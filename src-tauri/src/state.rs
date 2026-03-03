use crate::elf::ElfArchive;
use crate::engine::EngineManager;
use crate::extensions::terminal::TerminalSession;
use crate::sync::observer::AgentSyncEvent;
use dashmap::DashMap;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU16, AtomicUsize};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

/// Information about an open file
#[derive(Clone)]
pub struct FileInfo {
    pub archive: Arc<ElfArchive>,
    pub path: PathBuf,
}

/// Handle for a running per-agent MCP server.
///
/// Each enabled agent gets its own MCP server on a dedicated port.
/// The cancel_token triggers graceful shutdown when the agent is disabled.
pub struct AgentServerHandle {
    /// TCP port this agent's MCP server is listening on
    pub port: u16,
    /// Agent Block ID this server is bound to
    pub agent_block_id: String,
    /// Cancellation token for graceful shutdown (cancel to stop server)
    pub cancel_token: CancellationToken,
    /// Active SSE connection count for this agent's server
    pub sse_count: Arc<AtomicUsize>,
}

/// Application state shared across all Tauri commands.
///
/// This state manages multiple open .elf files and their corresponding engine actors.
/// Each file has a unique file_id and is managed independently.
#[derive(Clone)]
pub struct AppState {
    /// Engine manager for processing commands on .elf files
    pub engine_manager: EngineManager,

    /// Map of file_id -> FileInfo for open files
    /// Using DashMap for thread-safe concurrent access
    pub files: Arc<DashMap<String, FileInfo>>,

    /// Map of file_id -> active editor_id
    /// This is UI state and is NOT persisted to .elf file
    /// Using DashMap for thread-safe concurrent access
    pub active_editors: Arc<DashMap<String, String>>,

    /// Active MCP SSE connection count (management port).
    /// Used to detect when all clients disconnect so we can auto-disable agent blocks.
    pub sse_connection_count: Arc<AtomicUsize>,

    /// Per-agent MCP server handles: agent_block_id -> AgentServerHandle
    pub agent_servers: Arc<DashMap<String, AgentServerHandle>>,

    /// Next port to allocate for agent MCP servers (starts at 47201)
    pub next_agent_port: Arc<AtomicU16>,

    /// Shared terminal sessions for both Tauri commands and MCP server.
    /// This is the same Arc as TerminalState.sessions — both share one map.
    pub terminal_sessions: Arc<Mutex<HashMap<String, TerminalSession>>>,

    /// Output buffers for terminal sessions, keyed by block_id.
    /// Used by MCP terminal_execute to capture command output.
    /// The reader thread appends PTY output here; MCP polls and reads it.
    pub terminal_output_buffers: Arc<DashMap<String, Arc<Mutex<Vec<u8>>>>>,

    /// Broadcast sender for state change notifications.
    /// Both Tauri commands and MCP server send file_id here after successful commands.
    /// The Tauri app subscribes and emits `state_changed` events to the frontend.
    pub state_changed_tx: broadcast::Sender<String>,

    /// Channel for agent sync events.
    /// Agent commands and file commands publish here; the AgentSyncObserver subscribes
    /// and autonomously manages session sync lifecycle.
    pub agent_sync_tx: broadcast::Sender<AgentSyncEvent>,
}

impl AppState {
    /// Create a new application state with empty file list.
    pub fn new() -> Self {
        Self {
            engine_manager: EngineManager::new(),
            files: Arc::new(DashMap::new()),
            active_editors: Arc::new(DashMap::new()),
            sse_connection_count: Arc::new(AtomicUsize::new(0)),
            agent_servers: Arc::new(DashMap::new()),
            next_agent_port: Arc::new(AtomicU16::new(47201)),
            terminal_sessions: Arc::new(Mutex::new(HashMap::new())),
            terminal_output_buffers: Arc::new(DashMap::new()),
            state_changed_tx: broadcast::channel(256).0,
            agent_sync_tx: broadcast::channel(64).0,
        }
    }

    /// Get the active editor for a file.
    ///
    /// Returns the editor_id of the currently active editor for the given file,
    /// or None if no editor is set as active.
    pub fn get_active_editor(&self, file_id: &str) -> Option<String> {
        self.active_editors.get(file_id).map(|e| e.value().clone())
    }

    /// Set the active editor for a file.
    ///
    /// This updates the UI state to track which editor is currently active
    /// for the given file. This state is NOT persisted to the .elf file.
    pub fn set_active_editor(&self, file_id: String, editor_id: String) {
        self.active_editors.insert(file_id, editor_id);
    }

    /// List all open files.
    ///
    /// Returns a vector of (file_id, path) tuples for all currently open files.
    pub fn list_open_files(&self) -> Vec<(String, String)> {
        self.files
            .iter()
            .map(|entry| {
                (
                    entry.key().clone(),
                    entry.value().path.to_string_lossy().to_string(),
                )
            })
            .collect()
    }

    /// Get file info by file_id.
    pub fn get_file_info(&self, file_id: &str) -> Option<(PathBuf, Arc<ElfArchive>)> {
        self.files
            .get(file_id)
            .map(|entry| (entry.value().path.clone(), entry.value().archive.clone()))
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
