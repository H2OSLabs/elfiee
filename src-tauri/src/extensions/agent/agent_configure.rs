//! Handler for agent.configure capability

use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, Event};
use capability_macros::capability;
use std::env;

use super::{AgentConfig, AgentConfigurePayload};

/// Handler for agent.configure capability.
///
/// Updates Agent configuration with validation:
/// - Validates API key environment variable exists (if provided)
/// - Supports partial updates (only provided fields are updated)
/// - editor_id cannot be modified
///
/// # Payload
/// Uses `AgentConfigurePayload` with optional fields: provider, model, api_key_env, system_prompt.
#[capability(id = "agent.configure", target = "agent")]
fn handle_agent_configure(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for agent.configure")?;

    // Verify block type
    if block.block_type != "agent" {
        return Err(format!(
            "agent.configure requires agent block, got '{}'",
            block.block_type
        ));
    }

    let payload: AgentConfigurePayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload for agent.configure: {}", e))?;

    // Parse existing config
    let mut config: AgentConfig = serde_json::from_value(block.contents.clone())
        .map_err(|e| format!("Invalid AgentConfig in block: {}", e))?;

    // Validate API key env var if provided
    if let Some(ref api_key_env) = payload.api_key_env {
        if api_key_env.trim().is_empty() {
            return Err("API key environment variable name cannot be empty".to_string());
        }
        if env::var(api_key_env).is_err() {
            return Err(format!(
                "Environment variable '{}' not found. Please set it before configuring.",
                api_key_env
            ));
        }
        config.api_key_env = api_key_env.clone();
    }

    // Apply partial updates
    if let Some(provider) = payload.provider {
        if provider.trim().is_empty() {
            return Err("Provider cannot be empty".to_string());
        }
        config.provider = provider;
    }

    if let Some(model) = payload.model {
        if model.trim().is_empty() {
            return Err("Model cannot be empty".to_string());
        }
        config.model = model;
    }

    if let Some(system_prompt) = payload.system_prompt {
        config.system_prompt = system_prompt;
    }

    // Update metadata timestamp
    let mut new_metadata = block.metadata.clone();
    new_metadata.touch();

    // Create event
    let event = create_event(
        block.block_id.clone(),
        "agent.configure",
        serde_json::json!({
            "contents": config,
            "metadata": new_metadata.to_json()
        }),
        &cmd.editor_id,
        1, // Placeholder - engine actor updates with correct count
    );

    Ok(vec![event])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Block, BlockMetadata, Command};
    use std::collections::HashMap;

    fn create_test_agent_block() -> Block {
        let config = AgentConfig {
            editor_id: "agent-12345".to_string(),
            provider: "anthropic".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            api_key_env: "ANTHROPIC_API_KEY".to_string(),
            system_prompt: "You are helpful.".to_string(),
        };

        Block {
            block_id: "block-agent-1".to_string(),
            name: "Test Agent".to_string(),
            block_type: "agent".to_string(),
            owner: "alice".to_string(),
            contents: serde_json::to_value(&config).unwrap(),
            children: HashMap::new(),
            metadata: BlockMetadata::new(),
        }
    }

    #[test]
    fn test_configure_update_model() {
        let block = create_test_agent_block();

        let cmd = Command::new(
            "alice".to_string(),
            "agent.configure".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "model": "claude-opus-4-20250514"
            }),
        );

        let result = handle_agent_configure(&cmd, Some(&block));
        assert!(result.is_ok());

        let events = result.unwrap();
        let contents = &events[0].value["contents"];
        assert_eq!(contents["model"], "claude-opus-4-20250514");
        // Other fields should be preserved
        assert_eq!(contents["provider"], "anthropic");
        assert_eq!(contents["editor_id"], "agent-12345");
    }

    #[test]
    fn test_configure_update_system_prompt() {
        let block = create_test_agent_block();

        let cmd = Command::new(
            "alice".to_string(),
            "agent.configure".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "system_prompt": "You are a coding assistant specialized in Rust."
            }),
        );

        let result = handle_agent_configure(&cmd, Some(&block));
        assert!(result.is_ok());

        let events = result.unwrap();
        let contents = &events[0].value["contents"];
        assert_eq!(
            contents["system_prompt"],
            "You are a coding assistant specialized in Rust."
        );
    }

    #[test]
    fn test_configure_empty_model_fails() {
        let block = create_test_agent_block();

        let cmd = Command::new(
            "alice".to_string(),
            "agent.configure".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "model": "  "
            }),
        );

        let result = handle_agent_configure(&cmd, Some(&block));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Model cannot be empty"));
    }

    #[test]
    fn test_configure_empty_provider_fails() {
        let block = create_test_agent_block();

        let cmd = Command::new(
            "alice".to_string(),
            "agent.configure".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "provider": ""
            }),
        );

        let result = handle_agent_configure(&cmd, Some(&block));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Provider cannot be empty"));
    }

    #[test]
    fn test_configure_preserves_editor_id() {
        let block = create_test_agent_block();
        let original_editor_id = "agent-12345";

        let cmd = Command::new(
            "alice".to_string(),
            "agent.configure".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "model": "new-model",
                "provider": "openai"
            }),
        );

        let result = handle_agent_configure(&cmd, Some(&block)).unwrap();
        let contents = &result[0].value["contents"];

        // editor_id should not be changed
        assert_eq!(contents["editor_id"], original_editor_id);
    }

    #[test]
    fn test_configure_no_block_fails() {
        let cmd = Command::new(
            "alice".to_string(),
            "agent.configure".to_string(),
            "block-123".to_string(),
            serde_json::json!({
                "model": "new-model"
            }),
        );

        let result = handle_agent_configure(&cmd, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Block required"));
    }

    #[test]
    fn test_configure_wrong_block_type_fails() {
        let mut block = create_test_agent_block();
        block.block_type = "markdown".to_string();

        let cmd = Command::new(
            "alice".to_string(),
            "agent.configure".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "model": "new-model"
            }),
        );

        let result = handle_agent_configure(&cmd, Some(&block));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("requires agent block"));
    }

    #[test]
    fn test_configure_updates_metadata_timestamp() {
        let mut block = create_test_agent_block();
        // Set a known old timestamp
        block.metadata.updated_at = Some("2020-01-01T00:00:00Z".to_string());
        let original_updated = block.metadata.updated_at.clone();

        let cmd = Command::new(
            "alice".to_string(),
            "agent.configure".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "system_prompt": "Updated prompt"
            }),
        );

        let result = handle_agent_configure(&cmd, Some(&block)).unwrap();
        let metadata = &result[0].value["metadata"];

        // updated_at should be changed (different from the old timestamp)
        let new_updated = metadata["updated_at"].as_str();
        assert!(new_updated.is_some());
        assert_ne!(new_updated, original_updated.as_deref());
        // Should be a recent timestamp, not the old one
        assert!(!new_updated.unwrap().starts_with("2020"));
    }

    #[test]
    fn test_configure_empty_payload_succeeds() {
        // Empty payload means no changes, but should still succeed
        let block = create_test_agent_block();

        let cmd = Command::new(
            "alice".to_string(),
            "agent.configure".to_string(),
            block.block_id.clone(),
            serde_json::json!({}),
        );

        let result = handle_agent_configure(&cmd, Some(&block));
        assert!(result.is_ok());

        // All original values should be preserved
        let events = result.unwrap();
        let contents = &events[0].value["contents"];
        assert_eq!(contents["provider"], "anthropic");
        assert_eq!(contents["model"], "claude-sonnet-4-20250514");
    }

    #[test]
    fn test_configure_multiple_fields() {
        let block = create_test_agent_block();

        let cmd = Command::new(
            "alice".to_string(),
            "agent.configure".to_string(),
            block.block_id.clone(),
            serde_json::json!({
                "provider": "openai",
                "model": "gpt-4",
                "system_prompt": "New prompt"
            }),
        );

        let result = handle_agent_configure(&cmd, Some(&block)).unwrap();
        let contents = &result[0].value["contents"];

        assert_eq!(contents["provider"], "openai");
        assert_eq!(contents["model"], "gpt-4");
        assert_eq!(contents["system_prompt"], "New prompt");
        // editor_id unchanged
        assert_eq!(contents["editor_id"], "agent-12345");
    }
}
