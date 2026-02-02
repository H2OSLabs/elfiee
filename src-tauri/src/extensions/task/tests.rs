//! Tests for Task extension
//!
//! Test categories:
//! - Payload deserialization tests
//! - Basic capability functionality tests
//! - Authorization/CBAC tests
//! - Block type validation tests
//! - Integration workflow tests

use super::*;
use crate::capabilities::grants::GrantsTable;
use crate::capabilities::registry::CapabilityRegistry;
use crate::models::{Block, BlockMetadata, Command, RELATION_IMPLEMENT};
use std::collections::HashMap;

// ============================================
// Helper functions
// ============================================

fn create_task_block(owner: &str) -> Block {
    let mut block = Block::new(
        "Test Task".to_string(),
        "task".to_string(),
        owner.to_string(),
    );
    block.contents = serde_json::json!({
        "markdown": "# 实现登录功能\n\n## 需求\n\n添加 OAuth 登录支持"
    });
    block
}

fn create_task_block_with_children(owner: &str) -> Block {
    let mut block = create_task_block(owner);
    let mut children = HashMap::new();
    children.insert(
        RELATION_IMPLEMENT.to_string(),
        vec!["block-code-1".to_string(), "block-code-2".to_string()],
    );
    block.children = children;
    block
}

// ============================================
// TaskWritePayload Tests
// ============================================

#[test]
fn test_write_payload_deserialize() {
    let json = serde_json::json!({
        "content": "# 实现登录\n\n## 需求\n\n添加 OAuth"
    });
    let payload: TaskWritePayload = serde_json::from_value(json).unwrap();
    assert_eq!(payload.content, "# 实现登录\n\n## 需求\n\n添加 OAuth");
}

#[test]
fn test_write_payload_missing_content() {
    let json = serde_json::json!({});
    let result: Result<TaskWritePayload, _> = serde_json::from_value(json);
    assert!(result.is_err(), "Should reject missing content");
}

// ============================================
// TaskReadPayload Tests
// ============================================

#[test]
fn test_read_payload_deserialize_empty() {
    let json = serde_json::json!({});
    let result: Result<TaskReadPayload, _> = serde_json::from_value(json);
    assert!(result.is_ok(), "Empty object should deserialize for read");
}

// ============================================
// TaskCommitPayload Tests
// ============================================

#[test]
fn test_commit_payload_deserialize_empty() {
    let json = serde_json::json!({});
    let result: Result<TaskCommitPayload, _> = serde_json::from_value(json);
    assert!(result.is_ok(), "Empty object should deserialize for commit");
}

// ============================================
// task.write Functionality Tests
// ============================================

#[test]
fn test_write_basic() {
    let registry = CapabilityRegistry::new();
    let cap = registry
        .get("task.write")
        .expect("task.write should be registered");

    let block = Block::new(
        "Test Task".to_string(),
        "task".to_string(),
        "alice".to_string(),
    );

    let cmd = Command::new(
        "alice".to_string(),
        "task.write".to_string(),
        block.block_id.clone(),
        serde_json::json!({
            "content": "# 实现登录功能\n\n添加 OAuth 登录支持"
        }),
    );

    let result = cap.handler(&cmd, Some(&block));
    assert!(result.is_ok(), "Handler should execute successfully");

    let events = result.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].entity, block.block_id);
    assert_eq!(events[0].attribute, "alice/task.write");

    // 验证 contents.markdown
    let contents = events[0].value.get("contents").unwrap();
    assert_eq!(
        contents.get("markdown").unwrap().as_str().unwrap(),
        "# 实现登录功能\n\n添加 OAuth 登录支持"
    );
}

#[test]
fn test_write_preserves_existing_fields() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("task.write").unwrap();

    let mut block = Block::new("Task".to_string(), "task".to_string(), "alice".to_string());
    block.contents = serde_json::json!({
        "markdown": "旧内容",
        "custom_field": "custom_value"
    });

    let cmd = Command::new(
        "alice".to_string(),
        "task.write".to_string(),
        block.block_id.clone(),
        serde_json::json!({
            "content": "新内容"
        }),
    );

    let events = cap.handler(&cmd, Some(&block)).unwrap();
    let contents = events[0].value.get("contents").unwrap();

    // markdown 被更新
    assert_eq!(
        contents.get("markdown").unwrap().as_str().unwrap(),
        "新内容"
    );
    // 其他字段保留
    assert!(
        contents.get("custom_field").is_some(),
        "custom_field should be preserved"
    );
}

#[test]
fn test_write_updates_metadata_timestamp() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("task.write").unwrap();

    let mut block = Block::new("Task".to_string(), "task".to_string(), "alice".to_string());
    block.metadata = BlockMetadata {
        description: None,
        created_at: Some("2026-01-01T00:00:00Z".to_string()),
        updated_at: Some("2026-01-01T00:00:00Z".to_string()),
        custom: HashMap::new(),
    };

    let cmd = Command::new(
        "alice".to_string(),
        "task.write".to_string(),
        block.block_id.clone(),
        serde_json::json!({
            "content": "内容"
        }),
    );

    let events = cap.handler(&cmd, Some(&block)).unwrap();
    let metadata_json = &events[0].value["metadata"];
    let metadata = BlockMetadata::from_json(metadata_json).unwrap();

    assert_ne!(
        metadata.updated_at,
        Some("2026-01-01T00:00:00Z".to_string()),
        "updated_at should be updated"
    );
    assert_eq!(
        metadata.created_at,
        Some("2026-01-01T00:00:00Z".to_string()),
        "created_at should be preserved"
    );
}

#[test]
fn test_write_missing_payload_fails() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("task.write").unwrap();

    let block = Block::new("Task".to_string(), "task".to_string(), "alice".to_string());

    let cmd = Command::new(
        "alice".to_string(),
        "task.write".to_string(),
        block.block_id.clone(),
        serde_json::json!({}),
    );

    let result = cap.handler(&cmd, Some(&block));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid payload"));
}

#[test]
fn test_write_wrong_block_type() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("task.write").unwrap();

    // markdown block, not task
    let block = Block::new(
        "Doc".to_string(),
        "markdown".to_string(),
        "alice".to_string(),
    );

    let cmd = Command::new(
        "alice".to_string(),
        "task.write".to_string(),
        block.block_id.clone(),
        serde_json::json!({
            "content": "内容"
        }),
    );

    let result = cap.handler(&cmd, Some(&block));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Expected task block"));
}

#[test]
fn test_write_no_block_fails() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("task.write").unwrap();

    let cmd = Command::new(
        "alice".to_string(),
        "task.write".to_string(),
        "nonexistent".to_string(),
        serde_json::json!({ "content": "c" }),
    );

    let result = cap.handler(&cmd, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Block required"));
}

// ============================================
// task.write Authorization Tests
// ============================================

#[test]
fn test_write_authorization_owner() {
    let grants_table = GrantsTable::new();
    let block = create_task_block("alice");

    let is_authorized =
        block.owner == "alice" || grants_table.has_grant("alice", "task.write", &block.block_id);
    assert!(is_authorized, "Owner should be authorized");
}

#[test]
fn test_write_authorization_non_owner_without_grant() {
    let grants_table = GrantsTable::new();
    let block = create_task_block("alice");

    let is_authorized =
        block.owner == "bob" || grants_table.has_grant("bob", "task.write", &block.block_id);
    assert!(
        !is_authorized,
        "Non-owner without grant should not be authorized"
    );
}

#[test]
fn test_write_authorization_non_owner_with_grant() {
    let mut grants_table = GrantsTable::new();
    let block = create_task_block("alice");

    grants_table.add_grant(
        "bob".to_string(),
        "task.write".to_string(),
        block.block_id.clone(),
    );

    let is_authorized =
        block.owner == "bob" || grants_table.has_grant("bob", "task.write", &block.block_id);
    assert!(is_authorized, "Non-owner with grant should be authorized");
}

// ============================================
// task.read Functionality Tests
// ============================================

#[test]
fn test_read_basic() {
    let registry = CapabilityRegistry::new();
    let cap = registry
        .get("task.read")
        .expect("task.read should be registered");

    let block = create_task_block("alice");

    let cmd = Command::new(
        "alice".to_string(),
        "task.read".to_string(),
        block.block_id.clone(),
        serde_json::json!({}),
    );

    let result = cap.handler(&cmd, Some(&block));
    assert!(result.is_ok(), "Handler should execute successfully");

    let events = result.unwrap();
    assert_eq!(events.len(), 0, "task.read is permission-only, no events");
}

#[test]
fn test_read_wrong_block_type() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("task.read").unwrap();

    let block = Block::new(
        "Doc".to_string(),
        "markdown".to_string(),
        "alice".to_string(),
    );

    let cmd = Command::new(
        "alice".to_string(),
        "task.read".to_string(),
        block.block_id.clone(),
        serde_json::json!({}),
    );

    let result = cap.handler(&cmd, Some(&block));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Expected task block"));
}

#[test]
fn test_read_no_block_fails() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("task.read").unwrap();

    let cmd = Command::new(
        "alice".to_string(),
        "task.read".to_string(),
        "nonexistent".to_string(),
        serde_json::json!({}),
    );

    let result = cap.handler(&cmd, None);
    assert!(result.is_err());
}

// ============================================
// task.read Authorization Tests
// ============================================

#[test]
fn test_read_authorization_owner() {
    let grants_table = GrantsTable::new();
    let block = create_task_block("alice");

    let is_authorized =
        block.owner == "alice" || grants_table.has_grant("alice", "task.read", &block.block_id);
    assert!(is_authorized);
}

#[test]
fn test_read_authorization_non_owner_without_grant() {
    let grants_table = GrantsTable::new();
    let block = create_task_block("alice");

    let is_authorized =
        block.owner == "bob" || grants_table.has_grant("bob", "task.read", &block.block_id);
    assert!(!is_authorized);
}

#[test]
fn test_read_authorization_non_owner_with_grant() {
    let mut grants_table = GrantsTable::new();
    let block = create_task_block("alice");

    grants_table.add_grant(
        "bob".to_string(),
        "task.read".to_string(),
        block.block_id.clone(),
    );

    let is_authorized =
        block.owner == "bob" || grants_table.has_grant("bob", "task.read", &block.block_id);
    assert!(is_authorized);
}

// ============================================
// task.commit Functionality Tests
// ============================================

#[test]
fn test_commit_basic_with_downstream() {
    let registry = CapabilityRegistry::new();
    let cap = registry
        .get("task.commit")
        .expect("task.commit should be registered");

    let block = create_task_block_with_children("alice");

    let cmd = Command::new(
        "alice".to_string(),
        "task.commit".to_string(),
        block.block_id.clone(),
        serde_json::json!({}),
    );

    let result = cap.handler(&cmd, Some(&block));
    assert!(
        result.is_ok(),
        "Handler should succeed with downstream blocks"
    );

    let events = result.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].entity, block.block_id);
    assert_eq!(events[0].attribute, "alice/task.commit");

    // 验证 event value（无 target_path，只有 downstream_block_ids）
    let value = &events[0].value;
    assert!(
        value.get("target_path").is_none(),
        "target_path should not be present"
    );
    let downstream: Vec<String> =
        serde_json::from_value(value.get("downstream_block_ids").unwrap().clone()).unwrap();
    assert_eq!(downstream.len(), 2);
    assert!(downstream.contains(&"block-code-1".to_string()));
    assert!(downstream.contains(&"block-code-2".to_string()));
}

#[test]
fn test_commit_no_downstream_fails() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("task.commit").unwrap();

    // Task block with no children
    let block = create_task_block("alice");

    let cmd = Command::new(
        "alice".to_string(),
        "task.commit".to_string(),
        block.block_id.clone(),
        serde_json::json!({}),
    );

    let result = cap.handler(&cmd, Some(&block));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No downstream blocks"));
}

#[test]
fn test_commit_empty_payload_accepted() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("task.commit").unwrap();

    let block = create_task_block_with_children("alice");

    let cmd = Command::new(
        "alice".to_string(),
        "task.commit".to_string(),
        block.block_id.clone(),
        serde_json::json!({}),
    );

    let result = cap.handler(&cmd, Some(&block));
    assert!(result.is_ok(), "Empty payload should be accepted");
}

#[test]
fn test_commit_wrong_block_type() {
    let registry = CapabilityRegistry::new();
    let cap = registry.get("task.commit").unwrap();

    let block = Block::new(
        "Doc".to_string(),
        "markdown".to_string(),
        "alice".to_string(),
    );

    let cmd = Command::new(
        "alice".to_string(),
        "task.commit".to_string(),
        block.block_id.clone(),
        serde_json::json!({}),
    );

    let result = cap.handler(&cmd, Some(&block));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Expected task block"));
}

#[test]
fn test_commit_allows_repeated_commits() {
    // 多次 commit 不被阻止（无状态检查）
    let registry = CapabilityRegistry::new();
    let cap = registry.get("task.commit").unwrap();

    let block = create_task_block_with_children("alice");

    // 第一次 commit
    let cmd1 = Command::new(
        "alice".to_string(),
        "task.commit".to_string(),
        block.block_id.clone(),
        serde_json::json!({}),
    );
    assert!(cap.handler(&cmd1, Some(&block)).is_ok());

    // 第二次 commit（同样成功）
    let cmd2 = Command::new(
        "alice".to_string(),
        "task.commit".to_string(),
        block.block_id.clone(),
        serde_json::json!({}),
    );
    assert!(cap.handler(&cmd2, Some(&block)).is_ok());
}

// ============================================
// task.commit Authorization Tests
// ============================================

#[test]
fn test_commit_authorization_owner() {
    let grants_table = GrantsTable::new();
    let block = create_task_block("alice");

    let is_authorized =
        block.owner == "alice" || grants_table.has_grant("alice", "task.commit", &block.block_id);
    assert!(is_authorized);
}

#[test]
fn test_commit_authorization_non_owner_without_grant() {
    let grants_table = GrantsTable::new();
    let block = create_task_block("alice");

    let is_authorized =
        block.owner == "bob" || grants_table.has_grant("bob", "task.commit", &block.block_id);
    assert!(!is_authorized);
}

#[test]
fn test_commit_authorization_non_owner_with_grant() {
    let mut grants_table = GrantsTable::new();
    let block = create_task_block("alice");

    grants_table.add_grant(
        "bob".to_string(),
        "task.commit".to_string(),
        block.block_id.clone(),
    );

    let is_authorized =
        block.owner == "bob" || grants_table.has_grant("bob", "task.commit", &block.block_id);
    assert!(is_authorized);
}

// ============================================
// Integration Workflow Test
// ============================================

#[test]
fn test_full_workflow_write_then_commit() {
    let registry = CapabilityRegistry::new();

    // Step 1: Create task block with downstream
    let mut block = Block::new(
        "Commit Task".to_string(),
        "task".to_string(),
        "alice".to_string(),
    );

    // Step 2: Write markdown content
    let write_cap = registry.get("task.write").unwrap();
    let write_cmd = Command::new(
        "alice".to_string(),
        "task.write".to_string(),
        block.block_id.clone(),
        serde_json::json!({
            "content": "# Fix Bug\n\n修复登录 bug"
        }),
    );
    let write_events = write_cap.handler(&write_cmd, Some(&block)).unwrap();
    block.contents = write_events[0].value.get("contents").unwrap().clone();

    // Step 3: Link downstream blocks (simulate)
    let mut children = HashMap::new();
    children.insert(
        RELATION_IMPLEMENT.to_string(),
        vec!["block-fix-1".to_string()],
    );
    block.children = children;

    // Step 4: Commit (empty payload, auto-discover)
    let commit_cap = registry.get("task.commit").unwrap();
    let commit_cmd = Command::new(
        "alice".to_string(),
        "task.commit".to_string(),
        block.block_id.clone(),
        serde_json::json!({}),
    );
    let commit_events = commit_cap.handler(&commit_cmd, Some(&block)).unwrap();
    assert_eq!(commit_events.len(), 1);
    assert_eq!(commit_events[0].attribute, "alice/task.commit");

    // Verify: markdown preserved after commit
    assert_eq!(
        block.contents.get("markdown").unwrap().as_str().unwrap(),
        "# Fix Bug\n\n修复登录 bug"
    );
}
