//! Handler for agent.create capability

use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, BlockMetadata, Command, Event};
use capability_macros::capability;

use super::{AgentConfig, AgentCreatePayload};

/// Handler for agent.create capability.
///
/// Creates an Agent Block with the following behavior:
/// 1. Generates a unique editor_id for the agent (format: "agent-{uuid}")
/// 2. Creates the Agent Block with AgentConfig in contents
///
/// Note: In a full implementation, this would also emit an editor.create event
/// to create the associated Editor entity. For now, we only create the block.
///
/// # Payload
/// Uses `AgentCreatePayload` with name, provider, model, api_key_env, system_prompt.
#[capability(id = "agent.create", target = "core/*")]
fn handle_agent_create(cmd: &Command, _block: Option<&Block>) -> CapResult<Vec<Event>> {
    let payload: AgentCreatePayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload for agent.create: {}", e))?;

    // Validate required fields
    if payload.name.trim().is_empty() {
        return Err("Agent name cannot be empty".to_string());
    }
    if payload.provider.trim().is_empty() {
        return Err("Provider cannot be empty".to_string());
    }
    if payload.model.trim().is_empty() {
        return Err("Model cannot be empty".to_string());
    }
    if payload.api_key_env.trim().is_empty() {
        return Err("API key environment variable name cannot be empty".to_string());
    }

    // Generate agent editor_id
    let agent_editor_id = format!("agent-{}", uuid::Uuid::new_v4());

    // Create AgentConfig
    let config = AgentConfig {
        editor_id: agent_editor_id,
        provider: payload.provider,
        model: payload.model,
        api_key_env: payload.api_key_env,
        system_prompt: payload.system_prompt.unwrap_or_default(),
    };

    // Generate block_id for the new Agent Block
    let block_id = uuid::Uuid::new_v4().to_string();

    // Create metadata with timestamps
    let metadata = BlockMetadata::new();

    // Serialize AgentConfig and add source field for consistency with core.create
    let mut contents = serde_json::to_value(&config)
        .map_err(|e| format!("Failed to serialize AgentConfig: {}", e))?;
    if let Some(obj) = contents.as_object_mut() {
        obj.insert("source".to_string(), serde_json::json!("outline"));
    }

    // Create Agent Block event
    let event = create_event(
        block_id,
        "agent.create",
        serde_json::json!({
            "name": payload.name,
            "type": "agent",
            "owner": cmd.editor_id,
            "contents": contents,
            "children": {},
            "metadata": metadata.to_json()
        }),
        &cmd.editor_id,
        1, // Placeholder - engine actor updates with correct count
    );

    Ok(vec![event])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Command;

    #[test]
    fn test_agent_create_success() {
        let cmd = Command::new(
            "alice".to_string(),
            "agent.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "My Assistant",
                "provider": "anthropic",
                "model": "claude-sonnet-4-20250514",
                "api_key_env": "ANTHROPIC_API_KEY"
            }),
        );

        let result = handle_agent_create(&cmd, None);
        assert!(result.is_ok());

        let events = result.unwrap();
        assert_eq!(events.len(), 1);

        let event = &events[0];
        assert_eq!(event.value["name"], "My Assistant");
        assert_eq!(event.value["type"], "agent");
        assert_eq!(event.value["owner"], "alice");

        // Verify contents has AgentConfig fields
        let contents = &event.value["contents"];
        assert_eq!(contents["provider"], "anthropic");
        assert_eq!(contents["model"], "claude-sonnet-4-20250514");
        assert_eq!(contents["api_key_env"], "ANTHROPIC_API_KEY");
        assert!(contents["editor_id"].as_str().unwrap().starts_with("agent-"));
        // Verify source field for consistency with core.create
        assert_eq!(contents["source"], "outline");
    }

    #[test]
    fn test_agent_create_with_system_prompt() {
        let cmd = Command::new(
            "alice".to_string(),
            "agent.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Code Assistant",
                "provider": "anthropic",
                "model": "claude-sonnet-4-20250514",
                "api_key_env": "ANTHROPIC_API_KEY",
                "system_prompt": "You are a helpful coding assistant."
            }),
        );

        let result = handle_agent_create(&cmd, None);
        assert!(result.is_ok());

        let events = result.unwrap();
        let contents = &events[0].value["contents"];
        assert_eq!(contents["system_prompt"], "You are a helpful coding assistant.");
    }

    #[test]
    fn test_agent_create_empty_name_fails() {
        let cmd = Command::new(
            "alice".to_string(),
            "agent.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "  ",
                "provider": "anthropic",
                "model": "claude-sonnet-4-20250514",
                "api_key_env": "ANTHROPIC_API_KEY"
            }),
        );

        let result = handle_agent_create(&cmd, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("name cannot be empty"));
    }

    #[test]
    fn test_agent_create_empty_provider_fails() {
        let cmd = Command::new(
            "alice".to_string(),
            "agent.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Assistant",
                "provider": "",
                "model": "claude-sonnet-4-20250514",
                "api_key_env": "ANTHROPIC_API_KEY"
            }),
        );

        let result = handle_agent_create(&cmd, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Provider cannot be empty"));
    }

    #[test]
    fn test_agent_create_generates_unique_editor_id() {
        let cmd1 = Command::new(
            "alice".to_string(),
            "agent.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Agent 1",
                "provider": "anthropic",
                "model": "claude-sonnet-4-20250514",
                "api_key_env": "KEY"
            }),
        );

        let cmd2 = Command::new(
            "alice".to_string(),
            "agent.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Agent 2",
                "provider": "anthropic",
                "model": "claude-sonnet-4-20250514",
                "api_key_env": "KEY"
            }),
        );

        let result1 = handle_agent_create(&cmd1, None).unwrap();
        let result2 = handle_agent_create(&cmd2, None).unwrap();

        let editor_id1 = result1[0].value["contents"]["editor_id"].as_str().unwrap();
        let editor_id2 = result2[0].value["contents"]["editor_id"].as_str().unwrap();

        // Each agent should have a unique editor_id
        assert_ne!(editor_id1, editor_id2);
    }

    #[test]
    fn test_agent_create_missing_required_field_fails() {
        let cmd = Command::new(
            "alice".to_string(),
            "agent.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Assistant"
                // Missing provider, model, api_key_env
            }),
        );

        let result = handle_agent_create(&cmd, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid payload"));
    }

    #[test]
    fn test_agent_create_generates_metadata() {
        let cmd = Command::new(
            "alice".to_string(),
            "agent.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "Assistant",
                "provider": "anthropic",
                "model": "claude-sonnet-4-20250514",
                "api_key_env": "KEY"
            }),
        );

        let result = handle_agent_create(&cmd, None).unwrap();
        let metadata = &result[0].value["metadata"];

        // Should have auto-generated timestamps
        assert!(metadata["created_at"].is_string());
        assert!(metadata["updated_at"].is_string());
    }
}
