use super::core::CapabilityHandler;
use std::collections::HashMap;
use std::sync::Arc;

/// Registry for managing capability handlers.
///
/// Capabilities are registered at initialization and can be looked up by ID.
pub struct CapabilityRegistry {
    handlers: HashMap<String, Arc<dyn CapabilityHandler>>,
}

impl CapabilityRegistry {
    /// Create a new registry with all built-in capabilities registered.
    pub fn new() -> Self {
        let mut registry = Self {
            handlers: HashMap::new(),
        };

        // Register built-in capabilities
        registry.register_builtins();

        // Register extension capabilities
        registry.register_extensions();

        registry
    }

    /// Backwards-compatible helper used by older tests.
    ///
    /// Some tests call `with_extensions()` explicitly to emphasize that
    /// extension capabilities must be part of the registry. The current
    /// implementation already registers them in `new()`, so we just
    /// delegate to `new()` here.
    pub fn with_extensions() -> Self {
        Self::new()
    }

    /// Register a capability handler.
    pub fn register(&mut self, handler: Arc<dyn CapabilityHandler>) {
        self.handlers.insert(handler.cap_id().to_string(), handler);
    }

    /// Get a capability handler by ID.
    pub fn get(&self, cap_id: &str) -> Option<Arc<dyn CapabilityHandler>> {
        self.handlers.get(cap_id).cloned()
    }

    /// Register all built-in capabilities.
    fn register_builtins(&mut self) {
        use super::builtins::*;

        self.register(Arc::new(CoreCreateCapability));
        self.register(Arc::new(CoreReadCapability));
        self.register(Arc::new(CoreLinkCapability));
        self.register(Arc::new(CoreUnlinkCapability));
        self.register(Arc::new(CoreDeleteCapability));
        self.register(Arc::new(CoreGrantCapability));
        self.register(Arc::new(CoreRevokeCapability));
        self.register(Arc::new(CoreUpdate_metadataCapability));
        self.register(Arc::new(CoreRenameCapability));
        self.register(Arc::new(CoreChange_typeCapability));
        self.register(Arc::new(EditorCreateCapability));
        self.register(Arc::new(EditorDeleteCapability));
    }

    /// Register all extension capabilities.
    fn register_extensions(&mut self) {
        use crate::extensions::agent::*;
        use crate::extensions::code::*;
        use crate::extensions::directory::*;
        use crate::extensions::markdown::*;
        use crate::extensions::task::*;
        use crate::extensions::terminal::*;

        // Markdown extension
        self.register(Arc::new(MarkdownWriteCapability));
        self.register(Arc::new(MarkdownReadCapability));

        // Terminal extension
        self.register(Arc::new(TerminalInitCapability));
        self.register(Arc::new(TerminalExecuteCapability));
        self.register(Arc::new(TerminalSaveCapability));
        self.register(Arc::new(TerminalCloseCapability));

        // Directory extension
        self.register(Arc::new(DirectoryImportCapability));
        self.register(Arc::new(DirectoryExportCapability));
        self.register(Arc::new(DirectoryWriteCapability));
        self.register(Arc::new(DirectoryCreateCapability));
        self.register(Arc::new(DirectoryDeleteCapability));
        self.register(Arc::new(DirectoryRenameCapability));
        self.register(Arc::new(DirectoryRename_with_type_changeCapability));

        // Code extension
        self.register(Arc::new(CodeReadCapability));
        self.register(Arc::new(CodeWriteCapability));

        // Task extension
        self.register(Arc::new(TaskWriteCapability));
        self.register(Arc::new(TaskReadCapability));
        self.register(Arc::new(TaskCommitCapability));

        // Agent extension
        self.register(Arc::new(AgentCreateCapability));
        self.register(Arc::new(AgentConfigureCapability));
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_initialization() {
        let registry = CapabilityRegistry::new();

        // Verify core capabilities are registered
        assert!(
            registry.get("core.create").is_some(),
            "core.create should be registered"
        );
        assert!(
            registry.get("core.link").is_some(),
            "core.link should be registered"
        );
        assert!(
            registry.get("core.unlink").is_some(),
            "core.unlink should be registered"
        );
        assert!(
            registry.get("core.delete").is_some(),
            "core.delete should be registered"
        );
        assert!(
            registry.get("core.grant").is_some(),
            "core.grant should be registered"
        );
        assert!(
            registry.get("core.revoke").is_some(),
            "core.revoke should be registered"
        );

        // Verify extension capabilities are registered
        assert!(
            registry.get("markdown.write").is_some(),
            "markdown.write should be registered"
        );
        assert!(
            registry.get("markdown.read").is_some(),
            "markdown.read should be registered"
        );
        assert!(
            registry.get("terminal.save").is_some(),
            "terminal.save should be registered"
        );
    }

    #[test]
    fn test_capability_lookup() {
        let registry = CapabilityRegistry::new();

        let cap = registry.get("core.link").unwrap();
        assert_eq!(cap.cap_id(), "core.link");
        assert_eq!(cap.target(), "core/*");
    }

    #[test]
    fn test_nonexistent_capability() {
        let registry = CapabilityRegistry::new();
        assert!(registry.get("nonexistent.capability").is_none());
    }

    #[test]
    fn test_link_capability_execution() {
        use crate::models::{Block, Command};

        let registry = CapabilityRegistry::new();
        let cap = registry.get("core.link").unwrap();

        let block = Block::new(
            "test".to_string(),
            "markdown".to_string(),
            "editor1".to_string(),
        );
        let cmd = Command::new(
            "editor1".to_string(),
            "core.link".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "relation": "implement",
                "target_id": "block2"
            }),
        );

        let events = cap.handler(&cmd, Some(&block)).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].entity, block.block_id);
        assert_eq!(events[0].attribute, "editor1/core.link");
    }

    #[test]
    fn test_create_capability_execution() {
        use crate::models::{Block, Command};

        let registry = CapabilityRegistry::new();
        let cap = registry.get("core.create").unwrap();

        let dummy_block = Block::new(
            "dummy".to_string(),
            "dummy".to_string(),
            "editor1".to_string(),
        );
        let cmd = Command::new(
            "editor1".to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "New Block",
                "block_type": "markdown"
            }),
        );

        let events = cap.handler(&cmd, Some(&dummy_block)).unwrap();
        assert_eq!(events.len(), 1); // Single event with full initial state
        assert_eq!(events[0].attribute, "editor1/core.create");
        // Verify the event contains all initial state
        let value = &events[0].value;
        assert_eq!(
            value.get("name").and_then(|v| v.as_str()),
            Some("New Block")
        );
        assert_eq!(value.get("type").and_then(|v| v.as_str()), Some("markdown"));
        assert_eq!(value.get("owner").and_then(|v| v.as_str()), Some("editor1"));
    }

    #[test]
    fn test_grant_capability_execution() {
        use crate::models::{Block, Command};

        let registry = CapabilityRegistry::new();
        let cap = registry.get("core.grant").unwrap();

        let block = Block::new(
            "test".to_string(),
            "markdown".to_string(),
            "editor1".to_string(),
        );
        let cmd = Command::new(
            "editor1".to_string(),
            "core.grant".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "target_editor": "editor2",
                "capability": "markdown.write",
                "target_block": block.block_id
            }),
        );

        let events = cap.handler(&cmd, Some(&block)).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].attribute, "editor1/core.grant");
    }

    #[test]
    fn test_unlink_capability_execution() {
        use crate::models::{Block, Command, RELATION_IMPLEMENT};
        use std::collections::HashMap;

        let registry = CapabilityRegistry::new();
        let cap = registry.get("core.unlink").unwrap();

        // Create block with existing children
        let mut block = Block::new(
            "test".to_string(),
            "markdown".to_string(),
            "editor1".to_string(),
        );
        let mut children = HashMap::new();
        children.insert(
            RELATION_IMPLEMENT.to_string(),
            vec!["block2".to_string(), "block3".to_string()],
        );
        block.children = children;

        let cmd = Command::new(
            "editor1".to_string(),
            "core.unlink".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "relation": RELATION_IMPLEMENT,
                "target_id": "block2"
            }),
        );

        let events = cap.handler(&cmd, Some(&block)).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].entity, block.block_id);
        assert_eq!(events[0].attribute, "editor1/core.unlink");

        // Verify block3 still exists, block2 was removed
        let value_obj = events[0].value.as_object().unwrap();
        let new_children: HashMap<String, Vec<String>> =
            serde_json::from_value(value_obj.get("children").unwrap().clone()).unwrap();
        assert_eq!(new_children.get(RELATION_IMPLEMENT).unwrap().len(), 1);
        assert_eq!(new_children.get(RELATION_IMPLEMENT).unwrap()[0], "block3");
    }

    #[test]
    fn test_revoke_capability_execution() {
        use crate::models::{Block, Command};

        let registry = CapabilityRegistry::new();
        let cap = registry.get("core.revoke").unwrap();

        let block = Block::new(
            "test".to_string(),
            "markdown".to_string(),
            "editor1".to_string(),
        );
        let cmd = Command::new(
            "editor1".to_string(),
            "core.revoke".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "target_editor": "editor2",
                "capability": "markdown.write",
                "target_block": block.block_id
            }),
        );

        let events = cap.handler(&cmd, Some(&block)).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].attribute, "editor1/core.revoke");
    }

    #[test]
    fn test_authorization_owner_always_authorized() {
        use crate::capabilities::grants::GrantsTable;
        use crate::models::{Block, Command};

        let grants_table = GrantsTable::new(); // Empty grants table
        let registry = CapabilityRegistry::new();

        // Owner is "alice"
        let block = Block::new(
            "Test Block".to_string(),
            "markdown".to_string(),
            "alice".to_string(),
        );

        // Alice (owner) tries to link - should succeed even with empty grants
        let cmd = Command::new(
            "alice".to_string(),
            "core.link".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "relation": "implement",
                "target_id": "block2"
            }),
        );

        // Certificator check: owner should always have permission
        assert!(
            block.owner == cmd.editor_id
                || grants_table.has_grant(&cmd.editor_id, "core.link", &block.block_id),
            "Owner should always be authorized"
        );

        // Execute capability
        let cap = registry.get("core.link").unwrap();
        let result = cap.handler(&cmd, Some(&block));
        assert!(result.is_ok(), "Owner should be able to execute capability");
    }

    #[test]
    fn test_authorization_non_owner_without_grant_rejected() {
        use crate::capabilities::grants::GrantsTable;
        use crate::models::{Block, Command};

        let grants_table = GrantsTable::new(); // Empty grants table
        let _registry = CapabilityRegistry::new();

        // Owner is "alice"
        let block = Block::new(
            "Test Block".to_string(),
            "markdown".to_string(),
            "alice".to_string(),
        );

        // Bob (non-owner) tries to link without grant
        let cmd = Command::new(
            "bob".to_string(),
            "core.link".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "relation": "implement",
                "target_id": "block2"
            }),
        );

        // Certificator check: non-owner without grant should be rejected
        let is_authorized = block.owner == cmd.editor_id
            || grants_table.has_grant(&cmd.editor_id, "core.link", &block.block_id);

        assert!(
            !is_authorized,
            "Non-owner without grant should not be authorized"
        );
    }

    #[test]
    fn test_authorization_non_owner_with_specific_grant_authorized() {
        use crate::capabilities::grants::GrantsTable;
        use crate::models::{Block, Command};

        let mut grants_table = GrantsTable::new();
        let registry = CapabilityRegistry::new();

        // Owner is "alice"
        let block = Block::new(
            "Test Block".to_string(),
            "markdown".to_string(),
            "alice".to_string(),
        );

        // Grant bob specific permission for this block
        grants_table.add_grant(
            "bob".to_string(),
            "core.link".to_string(),
            block.block_id.clone(),
        );

        // Bob tries to link with explicit grant
        let cmd = Command::new(
            "bob".to_string(),
            "core.link".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "relation": "implement",
                "target_id": "block2"
            }),
        );

        // Certificator check: non-owner with specific grant should be authorized
        let is_authorized = block.owner == cmd.editor_id
            || grants_table.has_grant(&cmd.editor_id, "core.link", &block.block_id);

        assert!(
            is_authorized,
            "Non-owner with specific grant should be authorized"
        );

        // Execute capability
        let cap = registry.get("core.link").unwrap();
        let result = cap.handler(&cmd, Some(&block));
        assert!(
            result.is_ok(),
            "User with grant should be able to execute capability"
        );
    }

    #[test]
    fn test_authorization_wildcard_grant_works_for_any_block() {
        use crate::capabilities::grants::GrantsTable;
        use crate::models::{Block, Command};

        let mut grants_table = GrantsTable::new();
        let registry = CapabilityRegistry::new();

        // Grant bob wildcard permission
        grants_table.add_grant("bob".to_string(), "core.link".to_string(), "*".to_string());

        // Create multiple blocks with different owners
        let block1 = Block::new(
            "Block 1".to_string(),
            "markdown".to_string(),
            "alice".to_string(),
        );

        let block2 = Block::new(
            "Block 2".to_string(),
            "markdown".to_string(),
            "charlie".to_string(),
        );

        // Bob tries to link on block1
        let cmd1 = Command::new(
            "bob".to_string(),
            "core.link".to_string(),
            block1.block_id.clone(),
            serde_json::json!({
                "relation": "implement",
                "target_id": "other_block"
            }),
        );

        // Bob tries to link on block2
        let cmd2 = Command::new(
            "bob".to_string(),
            "core.link".to_string(),
            block2.block_id.clone(),
            serde_json::json!({
                "relation": "implement",
                "target_id": "other_block"
            }),
        );

        // Certificator checks: wildcard grant should work for any block
        assert!(
            grants_table.has_grant(&cmd1.editor_id, "core.link", &block1.block_id),
            "Wildcard grant should work for block1"
        );
        assert!(
            grants_table.has_grant(&cmd2.editor_id, "core.link", &block2.block_id),
            "Wildcard grant should work for block2"
        );

        // Execute capabilities
        let cap = registry.get("core.link").unwrap();
        assert!(cap.handler(&cmd1, Some(&block1)).is_ok());
        assert!(cap.handler(&cmd2, Some(&block2)).is_ok());
    }

    #[test]
    fn test_authorization_different_capability_not_authorized() {
        use crate::capabilities::grants::GrantsTable;
        use crate::models::{Block, Command};

        let mut grants_table = GrantsTable::new();

        // Owner is "alice"
        let block = Block::new(
            "Test Block".to_string(),
            "markdown".to_string(),
            "alice".to_string(),
        );

        // Grant bob permission for core.link only
        grants_table.add_grant(
            "bob".to_string(),
            "core.link".to_string(),
            block.block_id.clone(),
        );

        // Bob tries to delete (different capability)
        let cmd = Command::new(
            "bob".to_string(),
            "core.delete".to_string(),
            block.block_id.clone(),
            serde_json::json!({}),
        );

        // Certificator check: should not have permission for different capability
        let is_authorized = block.owner == cmd.editor_id
            || grants_table.has_grant(&cmd.editor_id, "core.delete", &block.block_id);

        assert!(
            !is_authorized,
            "Grant for one capability should not authorize another"
        );
    }

    #[test]
    fn test_core_read_capability_execution() {
        use crate::models::{Block, Command};

        let registry = CapabilityRegistry::new();
        let cap = registry.get("core.read").unwrap();

        let block = Block::new(
            "test".to_string(),
            "markdown".to_string(),
            "editor1".to_string(),
        );
        let cmd = Command::new(
            "editor1".to_string(),
            "core.read".to_string(),
            block.block_id.clone(),
            serde_json::json!({}),
        );

        let events = cap.handler(&cmd, Some(&block)).unwrap();
        // core.read is a permission-only capability, no events generated
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_core_read_owner_always_authorized() {
        use crate::capabilities::grants::GrantsTable;
        use crate::models::{Block, Command};

        let grants_table = GrantsTable::new(); // Empty grants table
        let registry = CapabilityRegistry::new();

        // Owner is "alice"
        let block = Block::new(
            "Test Block".to_string(),
            "markdown".to_string(),
            "alice".to_string(),
        );

        // Alice (owner) tries to read metadata
        let cmd = Command::new(
            "alice".to_string(),
            "core.read".to_string(),
            block.block_id.clone(),
            serde_json::json!({}),
        );

        // Certificator check: owner should always have permission
        assert!(
            block.owner == cmd.editor_id
                || grants_table.has_grant(&cmd.editor_id, "core.read", &block.block_id),
            "Owner should always be authorized for core.read"
        );

        // Execute capability
        let cap = registry.get("core.read").unwrap();
        let result = cap.handler(&cmd, Some(&block));
        assert!(
            result.is_ok(),
            "Owner should be able to execute core.read capability"
        );
    }

    #[test]
    fn test_core_read_with_grant_authorized() {
        use crate::capabilities::grants::GrantsTable;
        use crate::models::{Block, Command};

        let mut grants_table = GrantsTable::new();
        let registry = CapabilityRegistry::new();

        // Owner is "alice"
        let block = Block::new(
            "Test Block".to_string(),
            "markdown".to_string(),
            "alice".to_string(),
        );

        // Grant bob core.read permission
        grants_table.add_grant(
            "bob".to_string(),
            "core.read".to_string(),
            block.block_id.clone(),
        );

        // Bob tries to read metadata with explicit grant
        let cmd = Command::new(
            "bob".to_string(),
            "core.read".to_string(),
            block.block_id.clone(),
            serde_json::json!({}),
        );

        // Certificator check: non-owner with grant should be authorized
        let is_authorized = block.owner == cmd.editor_id
            || grants_table.has_grant(&cmd.editor_id, "core.read", &block.block_id);

        assert!(
            is_authorized,
            "Non-owner with core.read grant should be authorized"
        );

        // Execute capability
        let cap = registry.get("core.read").unwrap();
        let result = cap.handler(&cmd, Some(&block));
        assert!(
            result.is_ok(),
            "User with core.read grant should be able to execute capability"
        );
    }

    #[test]
    fn test_core_read_without_grant_not_authorized() {
        use crate::capabilities::grants::GrantsTable;
        use crate::models::{Block, Command};

        let grants_table = GrantsTable::new(); // Empty grants table

        // Owner is "alice"
        let block = Block::new(
            "Test Block".to_string(),
            "markdown".to_string(),
            "alice".to_string(),
        );

        // Bob (non-owner) tries to read metadata without grant
        let cmd = Command::new(
            "bob".to_string(),
            "core.read".to_string(),
            block.block_id.clone(),
            serde_json::json!({}),
        );

        // Certificator check: non-owner without grant should not be authorized
        let is_authorized = block.owner == cmd.editor_id
            || grants_table.has_grant(&cmd.editor_id, "core.read", &block.block_id);

        assert!(
            !is_authorized,
            "Non-owner without core.read grant should not be authorized"
        );
    }
}
