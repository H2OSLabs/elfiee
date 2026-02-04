//! Module-level tests for Agent extension

use super::*;

// ============================================
// Data Model Tests
// ============================================

#[test]
fn test_agent_contents_serialization() {
    let contents = AgentContents {
        name: "elfiee".to_string(),
        provider: "claude_code".to_string(),
        config_dir: "/home/user/repo-a/.claude".to_string(),
        status: AgentStatus::Enabled,
        editor_id: "bot-123".to_string(),
    };

    let json = serde_json::to_value(&contents).unwrap();
    assert_eq!(json["name"], "elfiee");
    assert_eq!(json["provider"], "claude_code");
    assert_eq!(json["config_dir"], "/home/user/repo-a/.claude");
    assert_eq!(json["status"], "enabled");
    assert_eq!(json["editor_id"], "bot-123");

    // Roundtrip
    let deserialized: AgentContents = serde_json::from_value(json).unwrap();
    assert_eq!(deserialized.name, "elfiee");
    assert_eq!(deserialized.config_dir, "/home/user/repo-a/.claude");
    assert_eq!(deserialized.status, AgentStatus::Enabled);
    assert_eq!(deserialized.editor_id, "bot-123");
}

#[test]
fn test_agent_contents_editor_id_required() {
    // JSON without editor_id should fail deserialization (editor_id is now required)
    let json = serde_json::json!({
        "name": "elfiee",
        "config_dir": "/home/user/repo-a/.claude",
        "status": "enabled"
    });

    let result = serde_json::from_value::<AgentContents>(json);
    assert!(
        result.is_err(),
        "editor_id is required, deserialization should fail"
    );
}

#[test]
fn test_agent_status_serialization() {
    assert_eq!(
        serde_json::to_string(&AgentStatus::Enabled).unwrap(),
        "\"enabled\""
    );
    assert_eq!(
        serde_json::to_string(&AgentStatus::Disabled).unwrap(),
        "\"disabled\""
    );
}

#[test]
fn test_agent_status_deserialization() {
    let enabled: AgentStatus = serde_json::from_str("\"enabled\"").unwrap();
    let disabled: AgentStatus = serde_json::from_str("\"disabled\"").unwrap();

    assert_eq!(enabled, AgentStatus::Enabled);
    assert_eq!(disabled, AgentStatus::Disabled);
}

#[test]
fn test_agent_create_payload_with_all_fields() {
    let payload = AgentCreatePayload {
        name: Some("my-agent".to_string()),
        provider: "claude_code".to_string(),
        config_dir: "/home/user/repo-a/.claude".to_string(),
        editor_id: Some("bot-editor-1".to_string()),
    };

    let json = serde_json::to_value(&payload).unwrap();
    assert_eq!(json["name"], "my-agent");
    assert_eq!(json["config_dir"], "/home/user/repo-a/.claude");
    assert_eq!(json["editor_id"], "bot-editor-1");
}

#[test]
fn test_agent_create_payload_without_optionals() {
    let payload = AgentCreatePayload {
        name: None,
        provider: "claude_code".to_string(),
        config_dir: "/home/user/repo-a/.claude".to_string(),
        editor_id: None,
    };

    let json = serde_json::to_value(&payload).unwrap();
    assert!(!json.as_object().unwrap().contains_key("name")); // skip_serializing_if
    assert!(!json.as_object().unwrap().contains_key("editor_id")); // skip_serializing_if
    assert_eq!(json["config_dir"], "/home/user/repo-a/.claude");
}

#[test]
fn test_agent_enable_payload_serialization() {
    let payload = AgentEnablePayload {
        agent_block_id: "agent-block-123".to_string(),
    };

    let json = serde_json::to_value(&payload).unwrap();
    assert_eq!(json["agent_block_id"], "agent-block-123");
}

#[test]
fn test_agent_disable_payload_serialization() {
    let payload = AgentDisablePayload {
        agent_block_id: "agent-block-456".to_string(),
    };

    let json = serde_json::to_value(&payload).unwrap();
    assert_eq!(json["agent_block_id"], "agent-block-456");
}

#[test]
fn test_agent_create_result_serialization() {
    let result = AgentCreateResult {
        agent_block_id: "abc-123".to_string(),
        status: AgentStatus::Enabled,
        needs_restart: true,
        message: "Agent created".to_string(),
    };

    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["agent_block_id"], "abc-123");
    assert_eq!(json["status"], "enabled");
    assert_eq!(json["needs_restart"], true);
    assert_eq!(json["message"], "Agent created");
}

#[test]
fn test_agent_enable_result_serialization() {
    let result = AgentEnableResult {
        agent_block_id: "abc-123".to_string(),
        status: AgentStatus::Enabled,
        needs_restart: true,
        message: "Agent enabled".to_string(),
        warnings: vec!["symlink warning".to_string()],
    };

    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["warnings"].as_array().unwrap().len(), 1);
}

#[test]
fn test_agent_disable_result_serialization() {
    let result = AgentDisableResult {
        agent_block_id: "abc-123".to_string(),
        status: AgentStatus::Disabled,
        message: "Agent disabled".to_string(),
        warnings: vec![],
    };

    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["status"], "disabled");
    assert!(json["warnings"].as_array().unwrap().is_empty());
}

// ============================================
// Authorization Tests
// ============================================

#[test]
fn test_create_authorization_owner() {
    use crate::capabilities::grants::GrantsTable;
    use crate::models::Block;

    let grants_table = GrantsTable::new();
    let block = Block::new(
        "Test Block".to_string(),
        "agent".to_string(),
        "alice".to_string(),
    );

    let is_authorized =
        block.owner == "alice" || grants_table.has_grant("alice", "agent.create", &block.block_id);

    assert!(is_authorized, "Block owner should be authorized");
}

#[test]
fn test_create_authorization_non_owner_without_grant() {
    use crate::capabilities::grants::GrantsTable;
    use crate::models::Block;

    let grants_table = GrantsTable::new();
    let block = Block::new(
        "Test Block".to_string(),
        "agent".to_string(),
        "alice".to_string(),
    );

    let is_authorized =
        block.owner == "bob" || grants_table.has_grant("bob", "agent.create", &block.block_id);

    assert!(!is_authorized);
}

#[test]
fn test_enable_authorization_owner() {
    use crate::capabilities::grants::GrantsTable;
    use crate::models::Block;

    let grants_table = GrantsTable::new();
    let block = Block::new(
        "Test Agent".to_string(),
        "agent".to_string(),
        "alice".to_string(),
    );

    let is_authorized =
        block.owner == "alice" || grants_table.has_grant("alice", "agent.enable", &block.block_id);

    assert!(is_authorized);
}

#[test]
fn test_enable_authorization_non_owner_without_grant() {
    use crate::capabilities::grants::GrantsTable;
    use crate::models::Block;

    let grants_table = GrantsTable::new();
    let block = Block::new(
        "Test Agent".to_string(),
        "agent".to_string(),
        "alice".to_string(),
    );

    let is_authorized =
        block.owner == "bob" || grants_table.has_grant("bob", "agent.enable", &block.block_id);

    assert!(!is_authorized);
}

#[test]
fn test_enable_authorization_non_owner_with_grant() {
    use crate::capabilities::grants::GrantsTable;
    use crate::models::Block;

    let mut grants_table = GrantsTable::new();
    let block = Block::new(
        "Test Agent".to_string(),
        "agent".to_string(),
        "alice".to_string(),
    );

    grants_table.add_grant(
        "bob".to_string(),
        "agent.enable".to_string(),
        block.block_id.clone(),
    );

    let is_authorized =
        block.owner == "bob" || grants_table.has_grant("bob", "agent.enable", &block.block_id);

    assert!(is_authorized);
}

#[test]
fn test_disable_authorization_owner() {
    use crate::capabilities::grants::GrantsTable;
    use crate::models::Block;

    let grants_table = GrantsTable::new();
    let block = Block::new(
        "Test Agent".to_string(),
        "agent".to_string(),
        "alice".to_string(),
    );

    let is_authorized =
        block.owner == "alice" || grants_table.has_grant("alice", "agent.disable", &block.block_id);

    assert!(is_authorized);
}
