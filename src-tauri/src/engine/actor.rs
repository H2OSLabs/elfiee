use crate::capabilities::registry::CapabilityRegistry;
use crate::engine::event_store::{EventPoolWithPath, EventStore};
use crate::engine::state::StateProjector;
use crate::models::{Block, Command, Editor, Event, LinkBlockPayload, RELATION_IMPLEMENT};
use crate::utils::write_block_snapshot;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tokio::sync::{mpsc, oneshot};

/// Prefix for block-specific directories
const BLOCK_DIR_PREFIX: &str = "block-";

/// Inject _block_dir into block contents and create the directory.
///
/// Creates a block-specific directory and injects its path into the contents.
/// This is a runtime-only operation - _block_dir should be stripped before persistence.
///
/// # Arguments
/// - `temp_dir`: Parent temporary directory
/// - `block_id`: Block identifier
/// - `contents`: Block contents to inject _block_dir into
///
/// # Returns
/// - `Ok(PathBuf)`: Path to the created block directory
/// - `Err(String)`: I/O error
fn inject_block_dir(
    temp_dir: &Path,
    block_id: &str,
    contents: &mut serde_json::Value,
) -> Result<PathBuf, String> {
    let block_dir = temp_dir.join(format!("{}{}", BLOCK_DIR_PREFIX, block_id));

    // Create block directory if it doesn't exist.
    // Handle the case where it might already exist gracefully.
    if let Err(e) = std::fs::create_dir_all(&block_dir) {
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(format!("Failed to create block directory: {}", e));
        }
    }
    // Inject _block_dir into contents (runtime only, will be stripped before persistence)
    if let Some(obj) = contents.as_object_mut() {
        obj.insert(
            "_block_dir".to_string(),
            serde_json::json!(block_dir.to_string_lossy()),
        );
    }

    Ok(block_dir)
}

/// Messages that can be sent to the engine actor.
#[derive(Debug)]
pub enum EngineMessage {
    /// Process a command and return resulting events
    ProcessCommand {
        command: Command,
        response: oneshot::Sender<Result<Vec<Event>, String>>,
    },
    /// Get a block by ID
    GetBlock {
        block_id: String,
        response: oneshot::Sender<Option<Block>>,
    },
    /// Get all blocks
    GetAllBlocks {
        response: oneshot::Sender<HashMap<String, Block>>,
    },
    /// Get all editors
    GetAllEditors {
        response: oneshot::Sender<HashMap<String, Editor>>,
    },
    /// Get all grants as a map: editor_id -> Vec<(cap_id, block_id)>
    GetAllGrants {
        response: oneshot::Sender<HashMap<String, Vec<(String, String)>>>,
    },
    /// Get grants for a specific editor
    GetEditorGrants {
        editor_id: String,
        response: oneshot::Sender<Vec<(String, String)>>,
    },
    /// Get grants for a specific block
    GetBlockGrants {
        block_id: String,
        response: oneshot::Sender<Vec<(String, String, String)>>,
    },
    /// Check if an editor is authorized for a capability on a block
    CheckGrant {
        editor_id: String,
        cap_id: String,
        block_id: String,
        response: oneshot::Sender<bool>,
    },
    /// Get all events
    GetAllEvents {
        response: oneshot::Sender<Result<Vec<Event>, String>>,
    },
    /// Shutdown the actor
    Shutdown,
}

/// Actor that processes commands for a single .elf file.
///
/// Each file has its own engine actor, ensuring serial processing of commands
/// for that file. This prevents race conditions and maintains consistency.
pub struct ElfileEngineActor {
    /// Unique identifier for this file
    #[allow(dead_code)]
    file_id: String,

    /// Event pool with database path for temp_dir derivation
    event_pool_with_path: EventPoolWithPath,

    /// Current state projection
    state: StateProjector,

    /// Capability registry
    registry: CapabilityRegistry,

    /// Mailbox for receiving messages
    mailbox: mpsc::UnboundedReceiver<EngineMessage>,
}

impl ElfileEngineActor {
    /// Execute closure with temp dir derived from event pool path, if available.
    fn with_temp_dir<F>(&self, mut f: F)
    where
        F: FnMut(&Path),
    {
        if let Some(temp_dir) = self
            .event_pool_with_path
            .db_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
        {
            f(temp_dir);
        }
    }

    /// Inject _block_dir for a specific block when temp dir is available.
    fn inject_block_dir_if_possible(&self, block: &mut Block) {
        self.with_temp_dir(|temp_dir| {
            let _ = inject_block_dir(temp_dir, &block.block_id, &mut block.contents);
        });
    }

    /// Write physical snapshot files for events that modify block content.
    ///
    /// Called after events are committed and state is projected.
    /// Handles: markdown.write, code.write, directory.write, directory.import,
    /// directory.create, and core.create (for blocks with content).
    fn write_snapshots(&self, events: &[Event]) {
        let temp_dir = match self
            .event_pool_with_path
            .db_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
        {
            Some(dir) => dir,
            None => return, // :memory: database, no filesystem
        };

        for event in events {
            let cap_id = Self::extract_cap_id(&event.attribute);

            match cap_id {
                "markdown.write" | "code.write" | "task.write" => {
                    // Content write: get block from state and write snapshot
                    if let Some(block) = self.state.get_block(&event.entity) {
                        if let Err(e) = write_block_snapshot(
                            temp_dir,
                            &block.block_id,
                            &block.block_type,
                            &block.name,
                            &block.contents,
                        ) {
                            log::warn!("Snapshot error for {}: {}", event.entity, e);
                        }
                    }
                }
                "directory.write" => {
                    // Directory entries changed: write body.json snapshot
                    if let Some(block) = self.state.get_block(&event.entity) {
                        if let Err(e) = write_block_snapshot(
                            temp_dir,
                            &block.block_id,
                            &block.block_type,
                            &block.name,
                            &block.contents,
                        ) {
                            log::warn!("Snapshot error for directory {}: {}", event.entity, e);
                        }
                    }
                }
                "core.create" => {
                    // New block created: write snapshot if it has content
                    if let Some(block) = self.state.get_block(&event.entity) {
                        if let Err(e) = write_block_snapshot(
                            temp_dir,
                            &block.block_id,
                            &block.block_type,
                            &block.name,
                            &block.contents,
                        ) {
                            log::warn!("Snapshot error for new block {}: {}", event.entity, e);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Extract the capability ID from an event attribute.
    ///
    /// Attribute format: `{editor_id}/{cap_id}` (e.g., "alice/markdown.write")
    fn extract_cap_id(attribute: &str) -> &str {
        attribute.split('/').nth(1).unwrap_or("")
    }

    /// Check if linking source → target would create a cycle in the DAG.
    ///
    /// From target, DFS along `implement` children. If we reach source,
    /// a cycle would be formed: source → target → ... → source.
    /// Also rejects self-links (source == target).
    fn check_link_cycle(&self, source_id: &str, target_id: &str) -> Result<(), String> {
        // Self-link is always a cycle
        if source_id == target_id {
            return Err(format!(
                "Cycle detected: linking {} → {} would create a self-cycle",
                source_id, target_id
            ));
        }

        let mut visited = HashSet::new();
        let mut stack = vec![target_id.to_string()];

        while let Some(current) = stack.pop() {
            if current == source_id {
                return Err(format!(
                    "Cycle detected: linking {} → {} would create a cycle",
                    source_id, target_id
                ));
            }
            if visited.insert(current.clone()) {
                if let Some(block) = self.state.get_block(&current) {
                    if let Some(targets) = block.children.get(RELATION_IMPLEMENT) {
                        stack.extend(targets.iter().cloned());
                    }
                }
            }
        }
        Ok(())
    }

    /// Create a new engine actor for a file.
    ///
    /// This initializes the actor by replaying all events from the database
    /// to rebuild the current state.
    pub async fn new(
        file_id: String,
        event_pool_with_path: EventPoolWithPath,
        mailbox: mpsc::UnboundedReceiver<EngineMessage>,
    ) -> Result<Self, String> {
        let registry = CapabilityRegistry::new();
        let mut state = StateProjector::new();

        // Replay all events from database to rebuild state
        let events = EventStore::get_all_events(&event_pool_with_path.pool)
            .await
            .map_err(|e| format!("Failed to load events from database: {}", e))?;
        state.replay(events);

        Ok(Self {
            file_id,
            event_pool_with_path,
            state,
            registry,
            mailbox,
        })
    }

    /// Run the actor's main loop.
    ///
    /// This processes messages from the mailbox until a Shutdown message is received.
    pub async fn run(mut self) {
        while let Some(msg) = self.mailbox.recv().await {
            match msg {
                EngineMessage::ProcessCommand { command, response } => {
                    let result = self.process_command(command).await;
                    let _ = response.send(result);
                }
                EngineMessage::GetBlock { block_id, response } => {
                    let mut block = self.state.get_block(&block_id).cloned();

                    if let Some(ref mut b) = block {
                        self.inject_block_dir_if_possible(b);
                    }
                    let _ = response.send(block);
                }
                EngineMessage::GetAllBlocks { response } => {
                    let mut blocks = self.state.blocks.clone();

                    self.with_temp_dir(|temp_dir| {
                        for block in blocks.values_mut() {
                            let _ =
                                inject_block_dir(temp_dir, &block.block_id, &mut block.contents);
                        }
                    });
                    let _ = response.send(blocks);
                }
                EngineMessage::GetAllEditors { response } => {
                    let editors = self.state.editors.clone();
                    let _ = response.send(editors);
                }
                EngineMessage::GetAllGrants { response } => {
                    let grants = self.state.grants.as_map().clone();
                    let _ = response.send(grants);
                }
                EngineMessage::GetAllEvents { response } => {
                    let events = EventStore::get_all_events(&self.event_pool_with_path.pool)
                        .await
                        .map_err(|e| format!("Failed to get events: {}", e));
                    let _ = response.send(events);
                }
                EngineMessage::GetEditorGrants {
                    editor_id,
                    response,
                } => {
                    let grants = self
                        .state
                        .grants
                        .get_grants(&editor_id)
                        .cloned()
                        .unwrap_or_default();
                    let _ = response.send(grants);
                }
                EngineMessage::GetBlockGrants { block_id, response } => {
                    // Get all grants and filter those that apply to this block
                    let mut block_grants = Vec::new();
                    for (editor_id, grants) in self.state.grants.as_map() {
                        for (cap_id, target_block) in grants {
                            if target_block == &block_id || target_block == "*" {
                                block_grants.push((
                                    editor_id.clone(),
                                    cap_id.clone(),
                                    target_block.clone(),
                                ));
                            }
                        }
                    }
                    let _ = response.send(block_grants);
                }
                EngineMessage::CheckGrant {
                    editor_id,
                    cap_id,
                    block_id,
                    response,
                } => {
                    let authorized = self.state.is_authorized(&editor_id, &cap_id, &block_id);
                    let _ = response.send(authorized);
                }
                EngineMessage::Shutdown => {
                    break;
                }
            }
        }
    }

    /// Process a command and return resulting events.
    ///
    /// This is the core command processing logic:
    /// 1. Get capability handler
    /// 2. Get block (None for create, Some for others)
    /// 3. Check authorization (certificator)
    /// 4. Execute handler
    /// 5. Update vector clock
    /// 6. Check for conflicts (MVP simple version)
    /// 7. Commit events to EventStore
    /// 8. Apply events to StateProjector
    async fn process_command(&mut self, cmd: Command) -> Result<Vec<Event>, String> {
        // 1. Get capability handler
        let handler = self
            .registry
            .get(&cmd.cap_id)
            .ok_or_else(|| format!("Unknown capability: {}", cmd.cap_id))?;

        // 2. Get block (None for create operations, Some for others)
        // System-level operations like core.create, editor.create, editor.delete, and agent.create don't require a block
        let mut block_opt = if cmd.cap_id == "core.create"
            || cmd.cap_id == "editor.create"
            || cmd.cap_id == "editor.delete"
            || cmd.cap_id == "agent.create"
        {
            None
        } else {
            Some(
                self.state
                    .get_block(&cmd.block_id)
                    .ok_or_else(|| format!("Block not found: {}", cmd.block_id))?
                    .clone(), // Clone so we can modify it
            )
        };

        // 2.5. Inject _block_dir into block contents (runtime only, not persisted)
        // Skip for :memory: databases used in unit tests (no filesystem access needed)
        if let Some(ref mut block) = block_opt {
            if let Some(temp_dir) = self
                .event_pool_with_path
                .db_path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
            {
                // Use helper function to inject _block_dir and create directory
                inject_block_dir(temp_dir, &block.block_id, &mut block.contents)?;
            }
        }

        // 3. Check authorization (certificator)
        // Only check if block exists (non-create operations)
        if let Some(block) = block_opt.as_ref() {
            if !self
                .state
                .is_authorized(&cmd.editor_id, &cmd.cap_id, &block.block_id)
            {
                return Err(format!(
                    "Authorization failed: {} does not have permission for {} on block {}",
                    cmd.editor_id, cmd.cap_id, cmd.block_id
                ));
            }
        }

        // 3.5. DAG cycle detection for core.link
        if cmd.cap_id == "core.link" {
            let payload: LinkBlockPayload = serde_json::from_value(cmd.payload.clone())
                .map_err(|e| format!("Invalid payload for cycle check: {}", e))?;
            self.check_link_cycle(&cmd.block_id, &payload.target_id)?;
        }

        // 4. Execute handler (block now contains _block_dir)
        let mut events = handler.handler(&cmd, block_opt.as_ref())?;

        // 5. Update vector clock
        // Get the full current vector clock state and increment the current editor's count
        let mut full_timestamp = self.state.editor_counts.clone();
        let current_count = *full_timestamp.get(&cmd.editor_id).unwrap_or(&0);
        let new_count = current_count + 1;
        full_timestamp.insert(cmd.editor_id.clone(), new_count);

        for event in &mut events {
            event.timestamp = full_timestamp.clone();
        }

        // 5.5. Special handling: inject _block_dir for core.create
        // Skip for :memory: databases used in unit tests
        if cmd.cap_id == "core.create" {
            if let Some(temp_dir) = self
                .event_pool_with_path
                .db_path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
            {
                for event in &mut events {
                    if event.attribute.ends_with("/core.create") {
                        // Inject _block_dir into new block's contents
                        if let Some(contents) = event.value.get_mut("contents") {
                            // Use helper function to inject and create directory
                            inject_block_dir(temp_dir, &event.entity, contents)?;
                        }
                    }
                }
            }
        }

        // 6. Check for conflicts (MVP simple version)
        // For MVP, we just log if there's a potential conflict but don't reject
        // In production, this would trigger merge/resolution logic
        if self.state.has_conflict(&cmd.editor_id, current_count) {
            log::warn!(
                "Potential conflict detected for editor {} (expected: {}, current: {})",
                cmd.editor_id,
                current_count,
                self.state.get_editor_count(&cmd.editor_id)
            );
        }

        // 7. Strip runtime-only fields before persistence
        // _block_dir is injected at runtime and should not be stored in events
        let mut events_to_persist = events.clone();
        for event in &mut events_to_persist {
            if let Some(contents) = event.value.get_mut("contents") {
                if let Some(obj) = contents.as_object_mut() {
                    obj.remove("_block_dir");
                }
            }
        }

        // 8. Persist events to database (without runtime fields)
        EventStore::append_events(&self.event_pool_with_path.pool, &events_to_persist)
            .await
            .map_err(|e| format!("Failed to persist events to database: {}", e))?;

        // 9. Apply events to StateProjector (use original events with runtime fields)
        for event in &events {
            self.state.apply_event(event);
        }

        // 10. Write block snapshots to physical files (non-critical)
        // Snapshots are derived data for symlinks and external access.
        // Errors are logged but do not fail the command.
        self.write_snapshots(&events);

        // Return original events (with _block_dir) for caller
        Ok(events)
    }
}

/// Handle for interacting with an engine actor.
///
/// This provides an async API for sending messages to the actor.
#[derive(Clone, Debug)]
pub struct EngineHandle {
    sender: mpsc::UnboundedSender<EngineMessage>,
}

impl EngineHandle {
    /// Create a new handle with the given sender.
    pub fn new(sender: mpsc::UnboundedSender<EngineMessage>) -> Self {
        Self { sender }
    }

    /// Process a command and return resulting events.
    pub async fn process_command(&self, command: Command) -> Result<Vec<Event>, String> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(EngineMessage::ProcessCommand {
                command,
                response: tx,
            })
            .map_err(|_| "Engine actor has shut down".to_string())?;

        rx.await
            .map_err(|_| "Engine actor did not respond".to_string())?
    }

    /// Get a block by ID.
    pub async fn get_block(&self, block_id: String) -> Option<Block> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(EngineMessage::GetBlock {
                block_id,
                response: tx,
            })
            .ok()?;

        rx.await.ok()?
    }

    /// Get all blocks.
    pub async fn get_all_blocks(&self) -> HashMap<String, Block> {
        let (tx, rx) = oneshot::channel();
        if self
            .sender
            .send(EngineMessage::GetAllBlocks { response: tx })
            .is_err()
        {
            return HashMap::new();
        }

        rx.await.unwrap_or_default()
    }

    /// Get all editors.
    pub async fn get_all_editors(&self) -> HashMap<String, Editor> {
        let (tx, rx) = oneshot::channel();
        if self
            .sender
            .send(EngineMessage::GetAllEditors { response: tx })
            .is_err()
        {
            return HashMap::new();
        }

        rx.await.unwrap_or_default()
    }

    /// Get all grants.
    ///
    /// Returns a map of editor_id -> Vec<(cap_id, block_id)>
    pub async fn get_all_grants(&self) -> HashMap<String, Vec<(String, String)>> {
        let (tx, rx) = oneshot::channel();
        if self
            .sender
            .send(EngineMessage::GetAllGrants { response: tx })
            .is_err()
        {
            return HashMap::new();
        }

        rx.await.unwrap_or_default()
    }

    /// Get grants for a specific editor.
    ///
    /// Returns Vec<(cap_id, block_id)> for the given editor.
    pub async fn get_editor_grants(&self, editor_id: String) -> Vec<(String, String)> {
        let (tx, rx) = oneshot::channel();
        if self
            .sender
            .send(EngineMessage::GetEditorGrants {
                editor_id,
                response: tx,
            })
            .is_err()
        {
            return Vec::new();
        }

        rx.await.unwrap_or_default()
    }

    /// Get grants for a specific block.
    ///
    /// Returns Vec<(editor_id, cap_id, block_id)> for all grants that apply to this block.
    pub async fn get_block_grants(&self, block_id: String) -> Vec<(String, String, String)> {
        let (tx, rx) = oneshot::channel();
        if self
            .sender
            .send(EngineMessage::GetBlockGrants {
                block_id,
                response: tx,
            })
            .is_err()
        {
            return Vec::new();
        }

        rx.await.unwrap_or_default()
    }

    /// Check if an editor is authorized for a capability on a block.
    pub async fn check_grant(&self, editor_id: String, cap_id: String, block_id: String) -> bool {
        let (tx, rx) = oneshot::channel();
        if self
            .sender
            .send(EngineMessage::CheckGrant {
                editor_id,
                cap_id,
                block_id,
                response: tx,
            })
            .is_err()
        {
            return false;
        }

        rx.await.unwrap_or(false)
    }

    /// Get all events.
    ///
    /// Returns all events from the event store for this file.
    pub async fn get_all_events(&self) -> Result<Vec<Event>, String> {
        let (tx, rx) = oneshot::channel();
        if self
            .sender
            .send(EngineMessage::GetAllEvents { response: tx })
            .is_err()
        {
            return Err("Engine actor has shut down".to_string());
        }

        rx.await
            .map_err(|_| "Engine actor did not respond".to_string())?
    }

    /// Shutdown the engine actor.
    pub async fn shutdown(&self) {
        let _ = self.sender.send(EngineMessage::Shutdown);
    }
}

/// Spawn a new engine actor for a file.
///
/// Returns a handle for interacting with the actor.
pub async fn spawn_engine(
    file_id: String,
    event_pool_with_path: EventPoolWithPath,
) -> Result<EngineHandle, String> {
    let (tx, rx) = mpsc::unbounded_channel();

    let actor = ElfileEngineActor::new(file_id.clone(), event_pool_with_path, rx).await?;

    // Spawn the actor on tokio runtime
    tokio::spawn(async move {
        actor.run().await;
    });

    Ok(EngineHandle::new(tx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_engine_actor_creation() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        let handle = spawn_engine("test_file".to_string(), event_pool)
            .await
            .expect("Failed to spawn engine");

        // Test that we can get all blocks (should be empty initially)
        let blocks = handle.get_all_blocks().await;
        assert_eq!(blocks.len(), 0);

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_engine_create_block() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        let handle = spawn_engine("test_file".to_string(), event_pool.clone())
            .await
            .expect("Failed to spawn engine");

        // Create a block
        let cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Test Block",
                "block_type": "markdown"
            }),
        );

        let events = handle
            .process_command(cmd)
            .await
            .expect("Failed to create block");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].attribute, "alice/core.create");

        // Verify block was created
        let blocks = handle.get_all_blocks().await;
        assert_eq!(blocks.len(), 1);

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_engine_authorization_owner() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        let handle = spawn_engine("test_file".to_string(), event_pool.clone())
            .await
            .expect("Failed to spawn engine");

        // Create a block owned by alice
        let create_cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Alice's Block",
                "block_type": "markdown"
            }),
        );

        let create_events = handle
            .process_command(create_cmd)
            .await
            .expect("Failed to create block");
        let block_id = &create_events[0].entity;

        // Alice (owner) should be able to link
        let link_cmd = Command::new(
            "alice".to_string(),
            "core.link".to_string(),
            block_id.clone(),
            serde_json::json!({
                "relation": "implement",
                "target_id": "other_block"
            }),
        );

        let result = handle.process_command(link_cmd).await;
        assert!(result.is_ok(), "Owner should be authorized");

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_engine_authorization_non_owner_rejected() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        let handle = spawn_engine("test_file".to_string(), event_pool.clone())
            .await
            .expect("Failed to spawn engine");

        // Create a block owned by alice
        let create_cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Alice's Block",
                "block_type": "markdown"
            }),
        );

        let create_events = handle
            .process_command(create_cmd)
            .await
            .expect("Failed to create block");
        let block_id = &create_events[0].entity;

        // Bob (non-owner) should NOT be able to link without grant
        let link_cmd = Command::new(
            "bob".to_string(),
            "core.link".to_string(),
            block_id.clone(),
            serde_json::json!({
                "relation": "implement",
                "target_id": "other_block"
            }),
        );

        let result = handle.process_command(link_cmd).await;
        assert!(result.is_err(), "Non-owner should be rejected");
        assert!(result.unwrap_err().contains("Authorization failed"));

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_engine_authorization_with_grant() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        let handle = spawn_engine("test_file".to_string(), event_pool.clone())
            .await
            .expect("Failed to spawn engine");

        // Create a block owned by alice
        let create_cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Alice's Block",
                "block_type": "markdown"
            }),
        );

        let create_events = handle
            .process_command(create_cmd)
            .await
            .expect("Failed to create block");
        let block_id = &create_events[0].entity;

        // Alice grants bob permission
        let grant_cmd = Command::new(
            "alice".to_string(),
            "core.grant".to_string(),
            block_id.clone(),
            serde_json::json!({
                "target_editor": "bob",
                "capability": "core.link",
                "target_block": block_id
            }),
        );

        handle
            .process_command(grant_cmd)
            .await
            .expect("Failed to grant permission");

        // Bob should now be able to link
        let link_cmd = Command::new(
            "bob".to_string(),
            "core.link".to_string(),
            block_id.clone(),
            serde_json::json!({
                "relation": "implement",
                "target_id": "other_block"
            }),
        );

        let result = handle.process_command(link_cmd).await;
        assert!(result.is_ok(), "User with grant should be authorized");

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_engine_vector_clock_updates() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        let handle = spawn_engine("test_file".to_string(), event_pool.clone())
            .await
            .expect("Failed to spawn engine");

        // Create first block
        let cmd1 = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Block 1",
                "block_type": "markdown"
            }),
        );

        let events1 = handle
            .process_command(cmd1)
            .await
            .expect("Failed to create block 1");

        // Create second block
        let cmd2 = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Block 2",
                "block_type": "markdown"
            }),
        );

        let events2 = handle
            .process_command(cmd2)
            .await
            .expect("Failed to create block 2");

        // Verify vector clock incremented
        assert_eq!(events1[0].timestamp.get("alice"), Some(&1));
        assert_eq!(events2[0].timestamp.get("alice"), Some(&2));

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_engine_get_block() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        let handle = spawn_engine("test_file".to_string(), event_pool.clone())
            .await
            .expect("Failed to spawn engine");

        // Create a block
        let create_cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Test Block",
                "block_type": "markdown"
            }),
        );

        let events = handle
            .process_command(create_cmd)
            .await
            .expect("Failed to create block");
        let block_id = &events[0].entity;

        // Get the block
        let block = handle
            .get_block(block_id.clone())
            .await
            .expect("Block should exist");

        assert_eq!(block.name, "Test Block");
        assert_eq!(block.block_type, "markdown");
        assert_eq!(block.owner, "alice");

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_engine_get_block_injects_dir() {
        // Use a real temp directory to test injection logic
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("events.db");
        let event_pool = EventStore::create(db_path.to_str().unwrap()).await.unwrap();

        let handle = spawn_engine("test_file".to_string(), event_pool.clone())
            .await
            .expect("Failed to spawn engine");

        // Create a block
        let create_cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Test Block",
                "block_type": "markdown"
            }),
        );

        let events = handle
            .process_command(create_cmd)
            .await
            .expect("Failed to create block");
        let block_id = &events[0].entity;

        // Get the block
        let block = handle
            .get_block(block_id.clone())
            .await
            .expect("Block should exist");

        // Verify _block_dir is injected
        let contents = block.contents.as_object().unwrap();
        assert!(contents.contains_key("_block_dir"));

        let block_dir = contents.get("_block_dir").unwrap().as_str().unwrap();
        assert!(std::path::Path::new(block_dir).exists());
        assert!(block_dir.contains(block_id));

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_create_block_with_metadata() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        let handle = spawn_engine("test_file".to_string(), event_pool.clone())
            .await
            .expect("Failed to spawn engine");

        let cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "测试文档",
                "block_type": "markdown",
                "metadata": {
                    "description": "这是一个测试文档"
                }
            }),
        );

        let events = handle
            .process_command(cmd)
            .await
            .expect("Failed to create block");

        assert_eq!(events.len(), 1);

        let block_id = events[0].entity.clone();
        let block = handle.get_block(block_id).await.unwrap();

        assert_eq!(block.name, "测试文档");
        assert_eq!(
            block.metadata.description,
            Some("这是一个测试文档".to_string())
        );
        assert!(block.metadata.created_at.is_some());
        assert!(block.metadata.updated_at.is_some());

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_write_updates_timestamp() {
        let event_pool = EventStore::create(":memory:").await.unwrap();
        let handle = spawn_engine("test_file".to_string(), event_pool.clone())
            .await
            .expect("Failed to spawn engine");

        // 创建 Block
        let create_cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Test",
                "block_type": "markdown"
            }),
        );

        let events = handle.process_command(create_cmd).await.unwrap();
        let block_id = events[0].entity.clone();

        // 获取初始时间戳
        let block = handle.get_block(block_id.clone()).await.unwrap();
        let original_updated = block.metadata.updated_at.clone().unwrap();

        // 等待一小段时间
        tokio::time::sleep(tokio::time::Duration::from_millis(1100)).await;

        // 写入内容
        let write_cmd = Command::new(
            "alice".to_string(),
            "markdown.write".to_string(),
            block_id.clone(),
            serde_json::json!({
                "content": "# Hello World"
            }),
        );

        let result = handle.process_command(write_cmd).await;
        assert!(result.is_ok());

        // 检查时间戳是否更新
        let block = handle.get_block(block_id).await.unwrap();
        let new_updated = block.metadata.updated_at.clone().unwrap();

        assert_ne!(original_updated, new_updated);
        // created_at should remain unchanged
        assert!(block.metadata.created_at.is_some());

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_metadata_persists_after_replay() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("events.db");
        let event_pool = EventStore::create(db_path.to_str().unwrap()).await.unwrap();

        // 创建第一个 handle，执行操作
        {
            let handle = spawn_engine("test_file".to_string(), event_pool.clone())
                .await
                .unwrap();

            let cmd = Command::new(
                "alice".to_string(),
                "core.create".to_string(),
                "".to_string(),
                serde_json::json!({
                    "name": "持久化测试",
                    "block_type": "markdown",
                    "metadata": {
                        "description": "测试持久化"
                    }
                }),
            );

            handle.process_command(cmd).await.unwrap();
            handle.shutdown().await;
        }

        // 创建第二个 handle，重放事件
        {
            let handle = spawn_engine("test_file".to_string(), event_pool.clone())
                .await
                .unwrap();

            let blocks = handle.get_all_blocks().await;
            assert_eq!(blocks.len(), 1);

            let block = blocks.values().next().unwrap();
            assert_eq!(block.name, "持久化测试");
            assert_eq!(block.metadata.description, Some("测试持久化".to_string()));
            assert!(block.metadata.created_at.is_some());
            assert!(block.metadata.updated_at.is_some());

            handle.shutdown().await;
        }
    }
}
