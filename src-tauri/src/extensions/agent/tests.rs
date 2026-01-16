//! Module-level tests for Agent extension

use super::*;

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
    assert_eq!(json["model"], "claude-sonnet-4-20250514");
    assert_eq!(json["api_key_env"], "ANTHROPIC_API_KEY");
    assert_eq!(json["system_prompt"], "You are helpful.");

    // Roundtrip
    let deserialized: AgentConfig = serde_json::from_value(json).unwrap();
    assert_eq!(deserialized.editor_id, config.editor_id);
    assert_eq!(deserialized.provider, config.provider);
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
    assert_eq!(json["block_id"], "block-abc");
    assert_eq!(json["payload"]["content"], "# Hello");
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
    // With skip_serializing_if, None fields are omitted entirely
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
        proposed_commands: vec![
            ProposedCommand {
                cap_id: "markdown.write".to_string(),
                block_id: "block-1".to_string(),
                payload: serde_json::json!({"content": "Hello"}),
                description: Some("Write content".to_string()),
            },
        ],
        status: ProposalStatus::Pending,
        prompt: "Create a document".to_string(),
        raw_response: Some("LLM response here".to_string()),
    };

    let json = serde_json::to_value(&proposal).unwrap();
    assert_eq!(json["status"], "pending");
    assert_eq!(json["prompt"], "Create a document");
    assert_eq!(json["proposed_commands"].as_array().unwrap().len(), 1);
}

#[test]
fn test_agent_create_payload_serialization() {
    let payload = AgentCreatePayload {
        name: "My Assistant".to_string(),
        provider: "anthropic".to_string(),
        model: "claude-sonnet-4-20250514".to_string(),
        api_key_env: "ANTHROPIC_API_KEY".to_string(),
        system_prompt: Some("Be helpful".to_string()),
    };

    let json = serde_json::to_value(&payload).unwrap();
    assert_eq!(json["name"], "My Assistant");
    assert_eq!(json["system_prompt"], "Be helpful");
}

#[test]
fn test_agent_create_payload_without_system_prompt() {
    let payload = AgentCreatePayload {
        name: "Assistant".to_string(),
        provider: "anthropic".to_string(),
        model: "claude-sonnet-4-20250514".to_string(),
        api_key_env: "KEY".to_string(),
        system_prompt: None,
    };

    let json = serde_json::to_value(&payload).unwrap();
    // With skip_serializing_if, None fields are omitted entirely
    assert!(!json.as_object().unwrap().contains_key("system_prompt"));
}

#[test]
fn test_agent_configure_payload_all_fields() {
    let payload = AgentConfigurePayload {
        provider: Some("openai".to_string()),
        model: Some("gpt-4".to_string()),
        api_key_env: Some("OPENAI_KEY".to_string()),
        system_prompt: Some("New prompt".to_string()),
    };

    let json = serde_json::to_value(&payload).unwrap();
    assert_eq!(json["provider"], "openai");
    assert_eq!(json["model"], "gpt-4");
}

#[test]
fn test_agent_configure_payload_partial() {
    let payload = AgentConfigurePayload {
        provider: None,
        model: Some("new-model".to_string()),
        api_key_env: None,
        system_prompt: None,
    };

    let json = serde_json::to_value(&payload).unwrap();
    // With skip_serializing_if, None fields are omitted entirely
    assert!(!json.as_object().unwrap().contains_key("provider"));
    assert_eq!(json["model"], "new-model");
    // Only model should be present
    assert_eq!(json.as_object().unwrap().len(), 1);
}

#[test]
fn test_agent_invoke_payload_serialization() {
    let payload = AgentInvokePayload {
        prompt: "Help me write code".to_string(),
        max_context_tokens: Some(4000),
        context_block_ids: Some(vec!["block-1".to_string(), "block-2".to_string()]),
    };

    let json = serde_json::to_value(&payload).unwrap();
    assert_eq!(json["prompt"], "Help me write code");
    assert_eq!(json["max_context_tokens"], 4000);
    assert_eq!(json["context_block_ids"].as_array().unwrap().len(), 2);
}

#[test]
fn test_agent_invoke_payload_minimal() {
    let payload = AgentInvokePayload {
        prompt: "Simple request".to_string(),
        max_context_tokens: None,
        context_block_ids: None,
    };

    let json = serde_json::to_value(&payload).unwrap();
    assert_eq!(json["prompt"], "Simple request");
    // With skip_serializing_if, None fields are omitted entirely
    assert!(!json.as_object().unwrap().contains_key("max_context_tokens"));
    assert!(!json.as_object().unwrap().contains_key("context_block_ids"));
}

#[test]
fn test_agent_approve_payload_serialization() {
    let payload = AgentApprovePayload {
        proposal_event_id: "event-12345".to_string(),
        approved: true,
    };

    let json = serde_json::to_value(&payload).unwrap();
    assert_eq!(json["proposal_event_id"], "event-12345");
    assert_eq!(json["approved"], true);

    let rejected = AgentApprovePayload {
        proposal_event_id: "event-67890".to_string(),
        approved: false,
    };

    let json = serde_json::to_value(&rejected).unwrap();
    assert_eq!(json["approved"], false);
}

// ============================================
// AgentCreate - Authorization Tests
// ============================================

#[test]
fn test_create_authorization_owner() {
    use crate::capabilities::grants::GrantsTable;
    use crate::models::Block;

    let grants_table = GrantsTable::new();

    // For agent.create, target is "core/*" so we check general authorization
    let block = Block::new(
        "Test Block".to_string(),
        "agent".to_string(),
        "alice".to_string(),
    );

    // Owner should always be authorized
    let is_authorized = block.owner == "alice"
        || grants_table.has_grant("alice", "agent.create", &block.block_id);

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

    // Bob (non-owner) without grant should not be authorized
    let is_authorized =
        block.owner == "bob" || grants_table.has_grant("bob", "agent.create", &block.block_id);

    assert!(
        !is_authorized,
        "Non-owner without grant should not be authorized"
    );
}

#[test]
fn test_create_authorization_non_owner_with_grant() {
    use crate::capabilities::grants::GrantsTable;
    use crate::models::Block;

    let mut grants_table = GrantsTable::new();

    let block = Block::new(
        "Test Block".to_string(),
        "agent".to_string(),
        "alice".to_string(),
    );

    // Grant Bob permission
    grants_table.add_grant(
        "bob".to_string(),
        "agent.create".to_string(),
        block.block_id.clone(),
    );

    let is_authorized =
        block.owner == "bob" || grants_table.has_grant("bob", "agent.create", &block.block_id);

    assert!(is_authorized, "Non-owner with grant should be authorized");
}

// ============================================
// AgentConfigure - Authorization Tests
// ============================================

#[test]
fn test_configure_authorization_owner() {
    use crate::capabilities::grants::GrantsTable;
    use crate::models::Block;

    let grants_table = GrantsTable::new();

    let block = Block::new(
        "Test Agent".to_string(),
        "agent".to_string(),
        "alice".to_string(),
    );

    // Owner should always be authorized
    let is_authorized = block.owner == "alice"
        || grants_table.has_grant("alice", "agent.configure", &block.block_id);

    assert!(is_authorized, "Block owner should be authorized");
}

#[test]
fn test_configure_authorization_non_owner_without_grant() {
    use crate::capabilities::grants::GrantsTable;
    use crate::models::Block;

    let grants_table = GrantsTable::new();

    let block = Block::new(
        "Test Agent".to_string(),
        "agent".to_string(),
        "alice".to_string(),
    );

    // Bob (non-owner) without grant should not be authorized
    let is_authorized =
        block.owner == "bob" || grants_table.has_grant("bob", "agent.configure", &block.block_id);

    assert!(
        !is_authorized,
        "Non-owner without grant should not be authorized"
    );
}

#[test]
fn test_configure_authorization_non_owner_with_grant() {
    use crate::capabilities::grants::GrantsTable;
    use crate::models::Block;

    let mut grants_table = GrantsTable::new();

    let block = Block::new(
        "Test Agent".to_string(),
        "agent".to_string(),
        "alice".to_string(),
    );

    // Grant Bob permission
    grants_table.add_grant(
        "bob".to_string(),
        "agent.configure".to_string(),
        block.block_id.clone(),
    );

    let is_authorized =
        block.owner == "bob" || grants_table.has_grant("bob", "agent.configure", &block.block_id);

    assert!(is_authorized, "Non-owner with grant should be authorized");
}
