use crate::capabilities::grants::GrantsTable;
use crate::models::{Block, BlockMetadata, Editor, EditorType, Event, RELATION_IMPLEMENT};
use log;
use std::collections::HashMap;

/// In-memory state projection from events.
///
/// Replays all events to build the current state of blocks, editors, and grants.
/// This is the authoritative source of truth for the engine's current state.
pub struct StateProjector {
    /// All blocks indexed by block_id
    pub blocks: HashMap<String, Block>,

    /// All editors indexed by editor_id
    pub editors: HashMap<String, Editor>,

    /// Grants table for authorization (reuses existing implementation)
    pub grants: GrantsTable,

    /// Vector clock counts for each editor (for conflict detection)
    pub editor_counts: HashMap<String, i64>,

    /// Reverse index: child_block_id → list of parent_block_ids
    ///
    /// Maintained for `implement` relations only (the sole relation type).
    /// Updated on core.link, core.unlink, and core.delete events.
    pub parents: HashMap<String, Vec<String>>,
}

impl StateProjector {
    /// Create a new empty state projector.
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            editors: HashMap::new(),
            grants: GrantsTable::new(),
            editor_counts: HashMap::new(),
            parents: HashMap::new(),
        }
    }

    /// Replay all events to build current state.
    ///
    /// This is called once during engine initialization to rebuild
    /// state from the event store.
    pub fn replay(&mut self, events: Vec<Event>) {
        for event in events {
            self.apply_event(&event);
        }
    }

    /// Apply a single event to state.
    ///
    /// This method updates the in-memory state based on the event type.
    pub fn apply_event(&mut self, event: &Event) {
        // Update editor transaction counts from vector clock
        for (editor_id, count) in &event.timestamp {
            let current = self.editor_counts.entry(editor_id.clone()).or_insert(0);
            *current = (*current).max(*count);
        }

        // Parse attribute format: "{editor_id}/{cap_id}"
        let parts: Vec<&str> = event.attribute.split('/').collect();
        if parts.len() != 2 {
            return; // Invalid attribute format, skip
        }
        let cap_id = parts[1];

        // Handle different event types based on capability
        match cap_id {
            // Block creation
            "core.create" => {
                // Create event should contain full block state
                if let Some(obj) = event.value.as_object() {
                    let block = Block {
                        block_id: event.entity.clone(),
                        name: obj
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        block_type: obj
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        owner: obj
                            .get("owner")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        contents: obj
                            .get("contents")
                            .cloned()
                            .unwrap_or_else(|| serde_json::json!({})),
                        children: obj
                            .get("children")
                            .and_then(|v| serde_json::from_value(v.clone()).ok())
                            .unwrap_or_default(),
                        metadata: obj
                            .get("metadata")
                            .and_then(|v| match BlockMetadata::from_json(v) {
                                Ok(parsed) => Some(parsed),
                                Err(e) => {
                                    log::warn!(
                                        "Failed to parse metadata in core.create event for block {}: {}. Using default metadata.",
                                        event.entity,
                                        e
                                    );
                                    None
                                }
                            })
                            .unwrap_or_default(),
                    };
                    // Build reverse index for initial children
                    if let Some(targets) = block.children.get(RELATION_IMPLEMENT) {
                        for target in targets {
                            self.parents
                                .entry(target.clone())
                                .or_default()
                                .push(block.block_id.clone());
                        }
                    }
                    self.blocks.insert(block.block_id.clone(), block);
                }
            }

            // Block updates (write, link, save)
            _ if cap_id.ends_with(".write")
                || cap_id.ends_with(".link")
                || cap_id.ends_with(".save") =>
            {
                if let Some(block) = self.blocks.get_mut(&event.entity) {
                    // Update contents if present
                    if let Some(contents) = event.value.get("contents") {
                        if let Some(obj) = block.contents.as_object_mut() {
                            if let Some(new_contents) = contents.as_object() {
                                for (k, v) in new_contents {
                                    obj.insert(k.clone(), v.clone());
                                }
                            }
                        }
                    }
                    // Update children if present, maintaining reverse index
                    if let Some(children) = event.value.get("children") {
                        if let Ok(new_children) =
                            serde_json::from_value::<HashMap<String, Vec<String>>>(children.clone())
                        {
                            let old_targets: Vec<String> = block
                                .children
                                .get(RELATION_IMPLEMENT)
                                .cloned()
                                .unwrap_or_default();
                            let new_targets: Vec<String> = new_children
                                .get(RELATION_IMPLEMENT)
                                .cloned()
                                .unwrap_or_default();

                            // Add new parent entries
                            for target in &new_targets {
                                if !old_targets.contains(target) {
                                    self.parents
                                        .entry(target.clone())
                                        .or_default()
                                        .push(event.entity.clone());
                                }
                            }
                            // Remove old parent entries
                            for target in &old_targets {
                                if !new_targets.contains(target) {
                                    if let Some(parent_list) = self.parents.get_mut(target) {
                                        parent_list.retain(|id| id != &event.entity);
                                        if parent_list.is_empty() {
                                            self.parents.remove(target);
                                        }
                                    }
                                }
                            }

                            block.children = new_children;
                        }
                    }
                    // Update metadata if present (e.g. updated_at from write ops)
                    if let Some(new_metadata) = event.value.get("metadata") {
                        match BlockMetadata::from_json(new_metadata) {
                            Ok(parsed) => {
                                block.metadata = parsed;
                            }
                            Err(e) => {
                                log::warn!(
                                    "Failed to parse metadata in {} event for block {}: {}",
                                    cap_id,
                                    event.entity,
                                    e
                                );
                            }
                        }
                    }
                }
            }

            "core.unlink" => {
                if let Some(block) = self.blocks.get_mut(&event.entity) {
                    // Update children, maintaining reverse index
                    if let Some(children) = event.value.get("children") {
                        if let Ok(new_children) =
                            serde_json::from_value::<HashMap<String, Vec<String>>>(children.clone())
                        {
                            let old_targets: Vec<String> = block
                                .children
                                .get(RELATION_IMPLEMENT)
                                .cloned()
                                .unwrap_or_default();
                            let new_targets: Vec<String> = new_children
                                .get(RELATION_IMPLEMENT)
                                .cloned()
                                .unwrap_or_default();

                            // Remove parent entries for targets that were unlinked
                            for target in &old_targets {
                                if !new_targets.contains(target) {
                                    if let Some(parent_list) = self.parents.get_mut(target) {
                                        parent_list.retain(|id| id != &event.entity);
                                        if parent_list.is_empty() {
                                            self.parents.remove(target);
                                        }
                                    }
                                }
                            }

                            block.children = new_children;
                        }
                    }
                }
            }

            // Block deletion
            "core.delete" => {
                // 1. Clean up forward direction: remove this block as a parent of its children
                if let Some(block) = self.blocks.get(&event.entity) {
                    if let Some(targets) = block.children.get(RELATION_IMPLEMENT) {
                        for target in targets {
                            if let Some(parent_list) = self.parents.get_mut(target) {
                                parent_list.retain(|id| id != &event.entity);
                                if parent_list.is_empty() {
                                    self.parents.remove(target);
                                }
                            }
                        }
                    }
                }

                // 2. Clean up reverse direction: remove this block from parent blocks' children
                if let Some(parent_ids) = self.parents.remove(&event.entity) {
                    for parent_id in &parent_ids {
                        if let Some(parent_block) = self.blocks.get_mut(parent_id) {
                            if let Some(targets) = parent_block.children.get_mut(RELATION_IMPLEMENT)
                            {
                                targets.retain(|id| id != &event.entity);
                            }
                        }
                    }
                }

                self.blocks.remove(&event.entity);
            }

            // Block rename
            "core.rename" => {
                if let Some(block) = self.blocks.get_mut(&event.entity) {
                    if let Some(name) = event.value.get("name").and_then(|v| v.as_str()) {
                        block.name = name.to_string();
                    }
                    // Update metadata if present (e.g. updated_at)
                    if let Some(new_metadata) = event.value.get("metadata") {
                        if let Ok(parsed) = BlockMetadata::from_json(new_metadata) {
                            block.metadata = parsed;
                        }
                    }
                }
            }

            // Block type change
            "core.change_type" => {
                if let Some(block) = self.blocks.get_mut(&event.entity) {
                    if let Some(block_type) = event.value.get("block_type").and_then(|v| v.as_str())
                    {
                        block.block_type = block_type.to_string();
                    }
                    // Update metadata if present (e.g. updated_at)
                    if let Some(new_metadata) = event.value.get("metadata") {
                        if let Ok(parsed) = BlockMetadata::from_json(new_metadata) {
                            block.metadata = parsed;
                        }
                    }
                }
            }

            // Block metadata update
            "core.update_metadata" => {
                if let Some(block) = self.blocks.get_mut(&event.entity) {
                    // Update metadata by merging with existing
                    if let Some(new_metadata) = event.value.get("metadata") {
                        match BlockMetadata::from_json(new_metadata) {
                            Ok(parsed) => {
                                block.metadata = parsed;
                            }
                            Err(e) => {
                                log::warn!(
                                    "Failed to parse metadata for block {}: {}",
                                    event.entity,
                                    e
                                );
                                log::debug!("Metadata value: {}", new_metadata);
                            }
                        }
                    }
                }
            }

            // Grant/Revoke - update the grants table directly
            "core.grant" | "core.revoke" => {
                if event.attribute.ends_with("/core.grant") {
                    // Process grant event
                    if let Some(grant_obj) = event.value.as_object() {
                        let editor = grant_obj
                            .get("editor")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let capability = grant_obj
                            .get("capability")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let block = grant_obj
                            .get("block")
                            .and_then(|v| v.as_str())
                            .unwrap_or("*");

                        if !editor.is_empty() && !capability.is_empty() {
                            self.grants.add_grant(
                                editor.to_string(),
                                capability.to_string(),
                                block.to_string(),
                            );
                        }
                    }
                } else if event.attribute.ends_with("/core.revoke") {
                    // Process revoke event
                    if let Some(revoke_obj) = event.value.as_object() {
                        let editor = revoke_obj
                            .get("editor")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let capability = revoke_obj
                            .get("capability")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let block = revoke_obj
                            .get("block")
                            .and_then(|v| v.as_str())
                            .unwrap_or("*");

                        if !editor.is_empty() && !capability.is_empty() {
                            self.grants.remove_grant(editor, capability, block);
                        }
                    }
                }
            }

            // Editor creation
            "editor.create" => {
                if let Some(editor_obj) = event.value.as_object() {
                    let editor_id = editor_obj
                        .get("editor_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let name = editor_obj
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let editor_type_str = editor_obj
                        .get("editor_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Human");

                    if !editor_id.is_empty() && !name.is_empty() {
                        let editor_type = match editor_type_str {
                            "Bot" => EditorType::Bot,
                            _ => EditorType::Human,
                        };

                        self.editors.insert(
                            editor_id.to_string(),
                            Editor {
                                editor_id: editor_id.to_string(),
                                name: name.to_string(),
                                editor_type,
                            },
                        );
                    }
                }
            }

            // Editor deletion
            "editor.delete" => {
                self.editors.remove(&event.entity);
                // Also remove all grants for this editor to prevent leaks in GrantsTable
                self.grants.remove_all_grants_for_editor(&event.entity);
            }

            _ => {
                // Unknown capability - ignore for now
            }
        }
    }

    /// Get a block by ID.
    pub fn get_block(&self, block_id: &str) -> Option<&Block> {
        self.blocks.get(block_id)
    }

    /// Get all parent (upstream) block IDs for a given block.
    ///
    /// Returns blocks that have an `implement` relation pointing to this block.
    pub fn get_parents(&self, block_id: &str) -> Vec<String> {
        self.parents.get(block_id).cloned().unwrap_or_default()
    }

    /// Get all child (downstream) block IDs for a given block.
    ///
    /// Returns blocks that this block has an `implement` relation to.
    pub fn get_children(&self, block_id: &str) -> Vec<String> {
        self.blocks
            .get(block_id)
            .and_then(|b| b.children.get(RELATION_IMPLEMENT))
            .cloned()
            .unwrap_or_default()
    }

    /// Check if an editor is authorized to execute a capability on a block.
    ///
    /// Authorization logic:
    /// 1. Block owner always has all permissions on their own block.
    /// 2. Otherwise, check the grants table for explicit authorization.
    pub fn is_authorized(&self, editor_id: &str, cap_id: &str, block_id: &str) -> bool {
        // Special case: core.create and editor.create are usually handled at a higher level
        // or have implicit permissions for any registered editor in this simple version.
        // But for block-level capabilities:
        if let Some(block) = self.get_block(block_id) {
            if block.owner == editor_id {
                return true;
            }
        }

        // Check explicit grants
        self.grants.has_grant(editor_id, cap_id, block_id)
    }

    /// Get the current transaction count for an editor.
    pub fn get_editor_count(&self, editor_id: &str) -> i64 {
        *self.editor_counts.get(editor_id).unwrap_or(&0)
    }

    /// Check for conflicts using vector clocks (MVP simple version).
    ///
    /// Returns true if the command is based on stale state.
    /// For MVP, we only check the command editor's count.
    pub fn has_conflict(&self, editor_id: &str, expected_count: i64) -> bool {
        let current_count = self.get_editor_count(editor_id);
        expected_count < current_count
    }
}

impl Default for StateProjector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap as StdHashMap;

    #[test]
    fn test_state_projector_create_block() {
        let mut state = StateProjector::new();

        let mut ts = StdHashMap::new();
        ts.insert("alice".to_string(), 1);

        let event = Event::new(
            "block1".to_string(),
            "alice/core.create".to_string(),
            serde_json::json!({
                "name": "Test Block",
                "type": "markdown",
                "owner": "alice",
                "contents": {},
                "children": {}
            }),
            ts,
        );

        state.apply_event(&event);

        assert_eq!(state.blocks.len(), 1);
        let block = state.get_block("block1").unwrap();
        assert_eq!(block.name, "Test Block");
        assert_eq!(block.block_type, "markdown");
        assert_eq!(block.owner, "alice");
    }

    #[test]
    fn test_state_projector_delete_block() {
        let mut state = StateProjector::new();

        // Create block
        let mut ts1 = StdHashMap::new();
        ts1.insert("alice".to_string(), 1);

        let create_event = Event::new(
            "block1".to_string(),
            "alice/core.create".to_string(),
            serde_json::json!({
                "name": "Test Block",
                "type": "markdown",
                "owner": "alice",
                "contents": {},
                "children": {}
            }),
            ts1,
        );

        state.apply_event(&create_event);
        assert_eq!(state.blocks.len(), 1);

        // Delete block
        let mut ts2 = StdHashMap::new();
        ts2.insert("alice".to_string(), 2);

        let delete_event = Event::new(
            "block1".to_string(),
            "alice/core.delete".to_string(),
            serde_json::json!({ "deleted": true }),
            ts2,
        );

        state.apply_event(&delete_event);
        assert_eq!(state.blocks.len(), 0);
    }

    #[test]
    fn test_state_projector_editor_count() {
        let mut state = StateProjector::new();

        let mut ts = StdHashMap::new();
        ts.insert("alice".to_string(), 5);
        ts.insert("bob".to_string(), 3);

        let event = Event::new(
            "block1".to_string(),
            "alice/core.create".to_string(),
            serde_json::json!({
                "name": "Test",
                "type": "markdown",
                "owner": "alice",
                "contents": {},
                "children": {}
            }),
            ts,
        );

        state.apply_event(&event);

        assert_eq!(state.get_editor_count("alice"), 5);
        assert_eq!(state.get_editor_count("bob"), 3);
        assert_eq!(state.get_editor_count("charlie"), 0);
    }

    #[test]
    fn test_conflict_detection() {
        let mut state = StateProjector::new();

        let mut ts = StdHashMap::new();
        ts.insert("alice".to_string(), 5);

        let event = Event::new(
            "block1".to_string(),
            "alice/core.create".to_string(),
            serde_json::json!({
                "name": "Test",
                "type": "markdown",
                "owner": "alice",
                "contents": {},
                "children": {}
            }),
            ts,
        );

        state.apply_event(&event);

        // Command based on count 5 is ok (current state)
        assert!(!state.has_conflict("alice", 5));

        // Command based on count 4 is a conflict (stale)
        assert!(state.has_conflict("alice", 4));

        // Command based on count 6 is ok (newer)
        assert!(!state.has_conflict("alice", 6));
    }

    #[test]
    fn test_apply_create_event_with_metadata() {
        let mut state = StateProjector::new();

        let event = Event::new(
            "block1".to_string(),
            "alice/core.create".to_string(),
            serde_json::json!({
                "name": "Test Block",
                "type": "markdown",
                "owner": "alice",
                "contents": {},
                "children": {},
                "metadata": {
                    "description": "测试描述",
                    "created_at": "2025-12-17T02:30:00Z",
                    "updated_at": "2025-12-17T02:30:00Z"
                }
            }),
            {
                let mut ts = std::collections::HashMap::new();
                ts.insert("alice".to_string(), 1);
                ts
            },
        );

        state.apply_event(&event);

        assert_eq!(state.blocks.len(), 1);
        let block = state.get_block("block1").unwrap();
        assert_eq!(block.name, "Test Block");

        // Verify metadata
        assert_eq!(block.metadata.description, Some("测试描述".to_string()));
        assert_eq!(
            block.metadata.created_at,
            Some("2025-12-17T02:30:00Z".to_string())
        );
        assert_eq!(
            block.metadata.updated_at,
            Some("2025-12-17T02:30:00Z".to_string())
        );
    }

    #[test]
    fn test_apply_write_event_updates_metadata() {
        let mut state = StateProjector::new();

        // 先创建 Block
        let create_event = Event::new(
            "block1".to_string(),
            "alice/core.create".to_string(),
            serde_json::json!({
                "name": "Test",
                "type": "markdown",
                "owner": "alice",
                "contents": {},
                "children": {},
                "metadata": {
                    "created_at": "2025-12-17T02:30:00Z",
                    "updated_at": "2025-12-17T02:30:00Z"
                }
            }),
            {
                let mut ts = std::collections::HashMap::new();
                ts.insert("alice".to_string(), 1);
                ts
            },
        );
        state.apply_event(&create_event);

        // 写入内容（模拟更新了 updated_at）
        let write_event = Event::new(
            "block1".to_string(),
            "alice/markdown.write".to_string(),
            serde_json::json!({
                "contents": {
                    "markdown": "# Hello"
                },
                "metadata": {
                    "created_at": "2025-12-17T02:30:00Z",
                    "updated_at": "2025-12-17T10:15:00Z"
                }
            }),
            {
                let mut ts = std::collections::HashMap::new();
                ts.insert("alice".to_string(), 2);
                ts
            },
        );
        state.apply_event(&write_event);

        let block = state.get_block("block1").unwrap();

        // Contents should be updated
        assert_eq!(block.contents["markdown"], "# Hello");

        // Metadata should be updated
        assert_eq!(
            block.metadata.created_at,
            Some("2025-12-17T02:30:00Z".to_string())
        );
        assert_eq!(
            block.metadata.updated_at,
            Some("2025-12-17T10:15:00Z".to_string())
        );
    }

    #[test]
    fn test_replay_maintains_metadata() {
        let mut state = StateProjector::new();

        let events = vec![
            Event::new(
                "block1".to_string(),
                "alice/core.create".to_string(),
                serde_json::json!({
                    "name": "Block 1",
                    "type": "markdown",
                    "owner": "alice",
                    "contents": {},
                    "children": {},
                    "metadata": {
                        "description": "描述1",
                        "created_at": "2025-12-17T02:00:00Z",
                        "updated_at": "2025-12-17T02:00:00Z"
                    }
                }),
                {
                    let mut ts = std::collections::HashMap::new();
                    ts.insert("alice".to_string(), 1);
                    ts
                },
            ),
            Event::new(
                "block1".to_string(),
                "alice/markdown.write".to_string(),
                serde_json::json!({
                    "contents": { "markdown": "内容" },
                    "metadata": {
                        "description": "描述1",
                        "created_at": "2025-12-17T02:00:00Z",
                        "updated_at": "2025-12-17T03:00:00Z"
                    }
                }),
                {
                    let mut ts = std::collections::HashMap::new();
                    ts.insert("alice".to_string(), 2);
                    ts
                },
            ),
        ];

        state.replay(events);

        let block = state.get_block("block1").unwrap();
        assert_eq!(block.metadata.description, Some("描述1".to_string()));
        assert_eq!(
            block.metadata.created_at,
            Some("2025-12-17T02:00:00Z".to_string())
        );
        assert_eq!(
            block.metadata.updated_at,
            Some("2025-12-17T03:00:00Z".to_string())
        );
    }

    #[test]
    fn test_grant_event_adds_permission() {
        let mut state = StateProjector::new();

        let grant_event = Event::new(
            "alice".to_string(),
            "alice/core.grant".to_string(),
            serde_json::json!({
                "editor": "bob",
                "capability": "markdown.write",
                "block": "block1"
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 1);
                ts
            },
        );

        state.apply_event(&grant_event);

        // Verify grant was added
        assert!(state.grants.has_grant("bob", "markdown.write", "block1"));
        assert!(!state.grants.has_grant("bob", "markdown.read", "block1"));
    }

    #[test]
    fn test_revoke_event_removes_permission() {
        let mut state = StateProjector::new();

        // First grant permission
        let grant_event = Event::new(
            "alice".to_string(),
            "alice/core.grant".to_string(),
            serde_json::json!({
                "editor": "bob",
                "capability": "markdown.write",
                "block": "block1"
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 1);
                ts
            },
        );
        state.apply_event(&grant_event);

        // Verify grant exists
        assert!(state.grants.has_grant("bob", "markdown.write", "block1"));

        // Now revoke it
        let revoke_event = Event::new(
            "alice".to_string(),
            "alice/core.revoke".to_string(),
            serde_json::json!({
                "editor": "bob",
                "capability": "markdown.write",
                "block": "block1"
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 2);
                ts
            },
        );
        state.apply_event(&revoke_event);

        // CRITICAL: Verify revoke actually removed the grant (bug fix validation)
        assert!(!state.grants.has_grant("bob", "markdown.write", "block1"));
    }

    #[test]
    fn test_grant_and_revoke_multiple_permissions() {
        let mut state = StateProjector::new();

        // Grant multiple permissions
        let events = vec![
            Event::new(
                "alice".to_string(),
                "alice/core.grant".to_string(),
                serde_json::json!({
                    "editor": "bob",
                    "capability": "markdown.write",
                    "block": "block1"
                }),
                {
                    let mut ts = StdHashMap::new();
                    ts.insert("alice".to_string(), 1);
                    ts
                },
            ),
            Event::new(
                "alice".to_string(),
                "alice/core.grant".to_string(),
                serde_json::json!({
                    "editor": "bob",
                    "capability": "markdown.read",
                    "block": "block1"
                }),
                {
                    let mut ts = StdHashMap::new();
                    ts.insert("alice".to_string(), 2);
                    ts
                },
            ),
        ];

        for event in events {
            state.apply_event(&event);
        }

        // Both permissions should exist
        assert!(state.grants.has_grant("bob", "markdown.write", "block1"));
        assert!(state.grants.has_grant("bob", "markdown.read", "block1"));

        // Revoke one permission
        let revoke_event = Event::new(
            "alice".to_string(),
            "alice/core.revoke".to_string(),
            serde_json::json!({
                "editor": "bob",
                "capability": "markdown.write",
                "block": "block1"
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 3);
                ts
            },
        );
        state.apply_event(&revoke_event);

        // Only the revoked one should be removed
        assert!(!state.grants.has_grant("bob", "markdown.write", "block1"));
        assert!(state.grants.has_grant("bob", "markdown.read", "block1"));
    }

    #[test]
    fn test_wildcard_grant_applies_to_all_blocks() {
        let mut state = StateProjector::new();

        // Grant with wildcard block
        let grant_event = Event::new(
            "alice".to_string(),
            "alice/core.grant".to_string(),
            serde_json::json!({
                "editor": "bob",
                "capability": "markdown.read",
                "block": "*"
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 1);
                ts
            },
        );
        state.apply_event(&grant_event);

        // Should have permission on any block with wildcard
        assert!(state.grants.has_grant("bob", "markdown.read", "*"));
    }

    #[test]
    fn test_editor_create_event_adds_to_state() {
        let mut state = StateProjector::new();

        let editor_create_event = Event::new(
            "editor-123".to_string(),
            "system/editor.create".to_string(),
            serde_json::json!({
                "editor_id": "editor-123",
                "name": "Alice",
                "editor_type": "Human"
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("system".to_string(), 1);
                ts
            },
        );

        state.apply_event(&editor_create_event);

        // Verify editor was added to state
        assert_eq!(state.editors.len(), 1);
        let editor = state.editors.get("editor-123").unwrap();
        assert_eq!(editor.name, "Alice");
        assert_eq!(editor.editor_type, crate::models::EditorType::Human);
    }

    #[test]
    fn test_editor_create_event_with_bot_type() {
        let mut state = StateProjector::new();

        let editor_create_event = Event::new(
            "bot-456".to_string(),
            "system/editor.create".to_string(),
            serde_json::json!({
                "editor_id": "bot-456",
                "name": "CodeReviewer",
                "editor_type": "Bot"
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("system".to_string(), 1);
                ts
            },
        );

        state.apply_event(&editor_create_event);

        // Verify bot editor was added
        let editor = state.editors.get("bot-456").unwrap();
        assert_eq!(editor.name, "CodeReviewer");
        assert_eq!(editor.editor_type, crate::models::EditorType::Bot);
    }

    #[test]
    fn test_state_projector_delete_editor() {
        let mut state = StateProjector::new();

        // 1. Create editor
        let editor_id = "editor-123".to_string();
        let create_event = Event::new(
            editor_id.clone(),
            "system/editor.create".to_string(),
            serde_json::json!({
                "editor_id": &editor_id,
                "name": "Alice",
                "editor_type": "Human"
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("system".to_string(), 1);
                ts
            },
        );
        state.apply_event(&create_event);

        // 2. Grant some permissions
        let grant_event = Event::new(
            "system".to_string(),
            "system/core.grant".to_string(),
            serde_json::json!({
                "editor": &editor_id,
                "capability": "markdown.write",
                "block": "block1"
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("system".to_string(), 2);
                ts
            },
        );
        state.apply_event(&grant_event);

        assert_eq!(state.editors.len(), 1);
        assert!(state
            .grants
            .has_grant(&editor_id, "markdown.write", "block1"));

        // 3. Delete editor
        let delete_event = Event::new(
            editor_id.clone(),
            "system/editor.delete".to_string(),
            serde_json::json!({ "deleted": true }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("system".to_string(), 3);
                ts
            },
        );
        state.apply_event(&delete_event);

        // 4. Verify editor and grants are gone
        assert_eq!(state.editors.len(), 0);
        assert!(state.editors.get(&editor_id).is_none());
        assert!(!state
            .grants
            .has_grant(&editor_id, "markdown.write", "block1"));
        assert!(state.grants.get_grants(&editor_id).is_none());
    }

    #[test]
    fn test_grant_revoke_with_empty_fields_ignored() {
        let mut state = StateProjector::new();

        // Grant event with empty editor field - should be ignored
        let grant_event = Event::new(
            "alice".to_string(),
            "alice/core.grant".to_string(),
            serde_json::json!({
                "editor": "",
                "capability": "markdown.write",
                "block": "block1"
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 1);
                ts
            },
        );
        state.apply_event(&grant_event);

        // Should not have added grant with empty editor
        assert!(!state.grants.has_grant("", "markdown.write", "block1"));

        // Grant event with empty capability - should be ignored
        let grant_event2 = Event::new(
            "alice".to_string(),
            "alice/core.grant".to_string(),
            serde_json::json!({
                "editor": "bob",
                "capability": "",
                "block": "block1"
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 2);
                ts
            },
        );
        state.apply_event(&grant_event2);

        // Should not have added grant with empty capability
        assert!(!state.grants.has_grant("bob", "", "block1"));
    }

    #[test]
    fn test_apply_change_type_event() {
        let mut state = StateProjector::new();

        // 1. Create a block (markdown)
        let create_event = Event::new(
            "block1".to_string(),
            "alice/core.create".to_string(),
            serde_json::json!({
                "name": "Test",
                "type": "markdown",
                "owner": "alice",
                "contents": {},
                "children": {}
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 1);
                ts
            },
        );
        state.apply_event(&create_event);

        let block = state.get_block("block1").unwrap();
        assert_eq!(block.block_type, "markdown");

        // 2. Apply change_type event
        let change_event = Event::new(
            "block1".to_string(),
            "alice/core.change_type".to_string(),
            serde_json::json!({
                "block_type": "code"
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 2);
                ts
            },
        );
        state.apply_event(&change_event);

        // 3. Verify block type updated
        let block = state.get_block("block1").unwrap();
        assert_eq!(block.block_type, "code");
    }

    // ========================================================================
    // Reverse index (parents) tests
    // ========================================================================

    fn create_block_event(entity: &str, name: &str, owner: &str, count: i64) -> Event {
        Event::new(
            entity.to_string(),
            format!("{}/core.create", owner),
            serde_json::json!({
                "name": name,
                "type": "markdown",
                "owner": owner,
                "contents": {},
                "children": {}
            }),
            {
                let mut ts = StdHashMap::new();
                ts.insert(owner.to_string(), count);
                ts
            },
        )
    }

    fn link_event(source: &str, target: &str, editor: &str, count: i64) -> Event {
        let mut children = StdHashMap::new();
        children.insert(RELATION_IMPLEMENT.to_string(), vec![target.to_string()]);
        Event::new(
            source.to_string(),
            format!("{}/core.link", editor),
            serde_json::json!({ "children": children }),
            {
                let mut ts = StdHashMap::new();
                ts.insert(editor.to_string(), count);
                ts
            },
        )
    }

    #[test]
    fn test_parents_after_link() {
        let mut state = StateProjector::new();

        state.apply_event(&create_block_event("a", "A", "alice", 1));
        state.apply_event(&create_block_event("b", "B", "alice", 2));

        // Link A → B
        state.apply_event(&link_event("a", "b", "alice", 3));

        assert_eq!(state.get_parents("b"), vec!["a".to_string()]);
        assert_eq!(state.get_children("a"), vec!["b".to_string()]);
        assert!(state.get_parents("a").is_empty());
        assert!(state.get_children("b").is_empty());
    }

    #[test]
    fn test_parents_after_unlink() {
        let mut state = StateProjector::new();

        state.apply_event(&create_block_event("a", "A", "alice", 1));
        state.apply_event(&create_block_event("b", "B", "alice", 2));

        // Link A → B
        state.apply_event(&link_event("a", "b", "alice", 3));
        assert_eq!(state.get_parents("b"), vec!["a".to_string()]);

        // Unlink A → B (children becomes empty)
        let unlink_event = Event::new(
            "a".to_string(),
            "alice/core.unlink".to_string(),
            serde_json::json!({ "children": {} }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 4);
                ts
            },
        );
        state.apply_event(&unlink_event);

        assert!(state.get_parents("b").is_empty());
        assert!(state.get_children("a").is_empty());
    }

    #[test]
    fn test_parents_multiple_parents() {
        let mut state = StateProjector::new();

        state.apply_event(&create_block_event("a", "A", "alice", 1));
        state.apply_event(&create_block_event("b", "B", "alice", 2));
        state.apply_event(&create_block_event("d", "D", "alice", 3));

        // A → D
        state.apply_event(&link_event("a", "d", "alice", 4));
        // B → D
        state.apply_event(&link_event("b", "d", "alice", 5));

        let parents = state.get_parents("d");
        assert_eq!(parents.len(), 2);
        assert!(parents.contains(&"a".to_string()));
        assert!(parents.contains(&"b".to_string()));
    }

    #[test]
    fn test_parents_cleanup_on_delete() {
        let mut state = StateProjector::new();

        state.apply_event(&create_block_event("a", "A", "alice", 1));
        state.apply_event(&create_block_event("b", "B", "alice", 2));

        // A → B
        state.apply_event(&link_event("a", "b", "alice", 3));
        assert_eq!(state.get_parents("b"), vec!["a".to_string()]);

        // Delete A
        let delete_event = Event::new(
            "a".to_string(),
            "alice/core.delete".to_string(),
            serde_json::json!({ "deleted": true }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 4);
                ts
            },
        );
        state.apply_event(&delete_event);

        // B should no longer have A as parent
        assert!(state.get_parents("b").is_empty());
        // A should not exist
        assert!(state.get_block("a").is_none());
    }

    #[test]
    fn test_delete_child_cleans_parent_children_map() {
        let mut state = StateProjector::new();

        state.apply_event(&create_block_event("a", "A", "alice", 1));
        state.apply_event(&create_block_event("b", "B", "alice", 2));
        state.apply_event(&create_block_event("c", "C", "alice", 3));

        // A → B, A → C
        let mut children = StdHashMap::new();
        children.insert(
            RELATION_IMPLEMENT.to_string(),
            vec!["b".to_string(), "c".to_string()],
        );
        let link_ev = Event::new(
            "a".to_string(),
            "alice/core.link".to_string(),
            serde_json::json!({ "children": children }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 4);
                ts
            },
        );
        state.apply_event(&link_ev);
        assert_eq!(state.get_children("a").len(), 2);

        // Delete B — A's children should shrink to [C]
        let delete_event = Event::new(
            "b".to_string(),
            "alice/core.delete".to_string(),
            serde_json::json!({}),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 5);
                ts
            },
        );
        state.apply_event(&delete_event);

        assert!(state.get_block("b").is_none());
        assert_eq!(state.get_children("a"), vec!["c".to_string()]);
        assert!(state.get_parents("b").is_empty());
    }

    #[test]
    fn test_get_children_convenience() {
        let mut state = StateProjector::new();

        state.apply_event(&create_block_event("a", "A", "alice", 1));
        state.apply_event(&create_block_event("b", "B", "alice", 2));
        state.apply_event(&create_block_event("c", "C", "alice", 3));

        // A → B, A → C (build children with both targets)
        let mut children = StdHashMap::new();
        children.insert(
            RELATION_IMPLEMENT.to_string(),
            vec!["b".to_string(), "c".to_string()],
        );
        let link_event = Event::new(
            "a".to_string(),
            "alice/core.link".to_string(),
            serde_json::json!({ "children": children }),
            {
                let mut ts = StdHashMap::new();
                ts.insert("alice".to_string(), 4);
                ts
            },
        );
        state.apply_event(&link_event);

        let children = state.get_children("a");
        assert_eq!(children.len(), 2);
        assert!(children.contains(&"b".to_string()));
        assert!(children.contains(&"c".to_string()));
    }
}
