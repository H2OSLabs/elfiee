//! Module-level tests for Agent extension (Phase 1 + Phase 2)

use super::*;

// ============================================
// Phase 1 Types (preserved for backward compatibility)
// ============================================

#[test]
fn test_agent_config_serialization() {
    let config = AgentConfig {
        editor_id: "agent-123".to_string(),
        provider: "anthropic".to_string(),
        model: "claude-sonnet-4-20250514".to_string(),
        api_key_env: "ANTHROPIC_API_KEY".to_string(),
        system_prompt: "You are helpful.".to_string(),
    };

    let json = serde_json::to_value(&config).unwrap();
    assert_eq!(json["editor_id"], "agent-123");
    assert_eq!(json["provider"], "anthropic");

    // Roundtrip
    let deserialized: AgentConfig = serde_json::from_value(json).unwrap();
    assert_eq!(deserialized.editor_id, config.editor_id);
}

#[test]
fn test_proposed_command_serialization() {
    let cmd = ProposedCommand {
        cap_id: "markdown.write".to_string(),
        block_id: "block-abc".to_string(),
        payload: serde_json::json!({"content": "# Hello"}),
        description: Some("Write heading".to_string()),
    };

    let json = serde_json::to_value(&cmd).unwrap();
    assert_eq!(json["cap_id"], "markdown.write");
    assert_eq!(json["description"], "Write heading");
}

#[test]
fn test_proposed_command_without_description() {
    let cmd = ProposedCommand {
        cap_id: "markdown.read".to_string(),
        block_id: "block-xyz".to_string(),
        payload: serde_json::json!({}),
        description: None,
    };

    let json = serde_json::to_value(&cmd).unwrap();
    assert!(!json.as_object().unwrap().contains_key("description"));
}

#[test]
fn test_proposal_status_serialization() {
    assert_eq!(
        serde_json::to_string(&ProposalStatus::Pending).unwrap(),
        "\"pending\""
    );
    assert_eq!(
        serde_json::to_string(&ProposalStatus::Approved).unwrap(),
        "\"approved\""
    );
    assert_eq!(
        serde_json::to_string(&ProposalStatus::Rejected).unwrap(),
        "\"rejected\""
    );
}

#[test]
fn test_proposal_status_deserialization() {
    let pending: ProposalStatus = serde_json::from_str("\"pending\"").unwrap();
    let approved: ProposalStatus = serde_json::from_str("\"approved\"").unwrap();
    let rejected: ProposalStatus = serde_json::from_str("\"rejected\"").unwrap();

    assert_eq!(pending, ProposalStatus::Pending);
    assert_eq!(approved, ProposalStatus::Approved);
    assert_eq!(rejected, ProposalStatus::Rejected);
}

#[test]
fn test_proposal_serialization() {
    let proposal = Proposal {
        proposed_commands: vec![ProposedCommand {
            cap_id: "markdown.write".to_string(),
            block_id: "block-1".to_string(),
            payload: serde_json::json!({"content": "Hello"}),
            description: Some("Write content".to_string()),
        }],
        status: ProposalStatus::Pending,
        prompt: "Create a document".to_string(),
        raw_response: Some("LLM response here".to_string()),
    };

    let json = serde_json::to_value(&proposal).unwrap();
    assert_eq!(json["status"], "pending");
    assert_eq!(json["prompt"], "Create a document");
    assert_eq!(json["proposed_commands"].as_array().unwrap().len(), 1);
}

// ============================================
// Phase 2 Types
// ============================================

#[test]
fn test_agent_contents_serialization() {
    let contents = AgentContents {
        name: "elfiee".to_string(),
        target_project_id: "proj-123".to_string(),
        status: AgentStatus::Enabled,
    };

    let json = serde_json::to_value(&contents).unwrap();
    assert_eq!(json["name"], "elfiee");
    assert_eq!(json["target_project_id"], "proj-123");
    assert_eq!(json["status"], "enabled");

    // Roundtrip
    let deserialized: AgentContents = serde_json::from_value(json).unwrap();
    assert_eq!(deserialized.name, "elfiee");
    assert_eq!(deserialized.target_project_id, "proj-123");
    assert_eq!(deserialized.status, AgentStatus::Enabled);
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
fn test_agent_create_v2_payload_with_name() {
    let payload = AgentCreateV2Payload {
        name: Some("my-agent".to_string()),
        target_project_id: "proj-123".to_string(),
    };

    let json = serde_json::to_value(&payload).unwrap();
    assert_eq!(json["name"], "my-agent");
    assert_eq!(json["target_project_id"], "proj-123");
}

#[test]
fn test_agent_create_v2_payload_without_name() {
    let payload = AgentCreateV2Payload {
        name: None,
        target_project_id: "proj-123".to_string(),
    };

    let json = serde_json::to_value(&payload).unwrap();
    assert!(!json.as_object().unwrap().contains_key("name")); // skip_serializing_if
    assert_eq!(json["target_project_id"], "proj-123");
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
