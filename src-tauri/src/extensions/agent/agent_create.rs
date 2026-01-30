//! Handler for agent.create capability (Phase 2)
//!
//! Creates an Agent Block for external project integration.
//! The handler generates the Block creation event; actual I/O (symlink, MCP config)
//! is performed by the Tauri command layer.

use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, BlockMetadata, Command, Event};
use capability_macros::capability;

use super::{AgentContents, AgentCreateV2Payload, AgentStatus};

/// Handler for agent.create capability (Phase 2).
///
/// Creates an Agent Block for external project integration:
/// 1. Validates the payload (target_project_id required)
/// 2. Creates an Agent Block with AgentContents in contents
/// 3. Sets initial status to Enabled (Tauri command will perform I/O)
///
/// # Payload
/// Uses `AgentCreateV2Payload` with target_project_id (required) and name (optional).
///
/// # Note
/// Uniqueness check (no duplicate agent for same project) and target project validation
/// are performed at the Tauri command layer, since the handler cannot access StateProjector.
#[capability(id = "agent.create", target = "core/*")]
fn handle_agent_create(cmd: &Command, _block: Option<&Block>) -> CapResult<Vec<Event>> {
    let payload: AgentCreateV2Payload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload for agent.create: {}", e))?;

    // Validate required fields
    if payload.target_project_id.trim().is_empty() {
        return Err("target_project_id cannot be empty".to_string());
    }

    let name = payload
        .name
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| "elfiee".to_string());

    // Create AgentContents
    let contents = AgentContents {
        name: name.clone(),
        target_project_id: payload.target_project_id.clone(),
        status: AgentStatus::Enabled,
    };

    // Generate block_id for the new Agent Block
    let block_id = uuid::Uuid::new_v4().to_string();

    // Create metadata with timestamps
    let metadata = BlockMetadata::new();

    // Serialize AgentContents and add source field for consistency with core.create
    let mut contents_json = serde_json::to_value(&contents)
        .map_err(|e| format!("Failed to serialize AgentContents: {}", e))?;
    if let Some(obj) = contents_json.as_object_mut() {
        obj.insert("source".to_string(), serde_json::json!("outline"));
    }

    // Create Agent Block event
    let event = create_event(
        block_id,
        "agent.create",
        serde_json::json!({
            "name": name,
            "type": "agent",
            "owner": cmd.editor_id,
            "contents": contents_json,
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
    fn test_agent_create_v2_success() {
        let cmd = Command::new(
            "alice".to_string(),
            "agent.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "target_project_id": "dir-block-uuid-123"
            }),
        );

        let result = handle_agent_create(&cmd, None);
        assert!(result.is_ok());

        let events = result.unwrap();
        assert_eq!(events.len(), 1);

        let event = &events[0];
        assert_eq!(event.value["name"], "elfiee"); // default name
        assert_eq!(event.value["type"], "agent");
        assert_eq!(event.value["owner"], "alice");

        // Verify contents has AgentContents fields
        let contents = &event.value["contents"];
        assert_eq!(contents["name"], "elfiee");
        assert_eq!(contents["target_project_id"], "dir-block-uuid-123");
        assert_eq!(contents["status"], "enabled");
        assert_eq!(contents["source"], "outline");
    }

    #[test]
    fn test_agent_create_v2_with_custom_name() {
        let cmd = Command::new(
            "alice".to_string(),
            "agent.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "target_project_id": "dir-block-uuid-123",
                "name": "my-agent"
            }),
        );

        let result = handle_agent_create(&cmd, None);
        assert!(result.is_ok());

        let events = result.unwrap();
        let event = &events[0];
        assert_eq!(event.value["name"], "my-agent");
        assert_eq!(event.value["contents"]["name"], "my-agent");
    }

    #[test]
    fn test_agent_create_v2_default_name_when_none() {
        let cmd = Command::new(
            "alice".to_string(),
            "agent.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "target_project_id": "dir-block-uuid-123"
            }),
        );

        let result = handle_agent_create(&cmd, None).unwrap();
        assert_eq!(result[0].value["name"], "elfiee");
    }

    #[test]
    fn test_agent_create_v2_empty_name_uses_default() {
        let cmd = Command::new(
            "alice".to_string(),
            "agent.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "target_project_id": "dir-block-uuid-123",
                "name": "  "
            }),
        );

        let result = handle_agent_create(&cmd, None).unwrap();
        assert_eq!(result[0].value["name"], "elfiee");
    }

    #[test]
    fn test_agent_create_v2_empty_target_project_id_fails() {
        let cmd = Command::new(
            "alice".to_string(),
            "agent.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "target_project_id": "  "
            }),
        );

        let result = handle_agent_create(&cmd, None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("target_project_id cannot be empty"));
    }

    #[test]
    fn test_agent_create_v2_missing_target_project_id_fails() {
        let cmd = Command::new(
            "alice".to_string(),
            "agent.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "my-agent"
            }),
        );

        let result = handle_agent_create(&cmd, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid payload"));
    }

    #[test]
    fn test_agent_create_v2_event_structure() {
        let cmd = Command::new(
            "alice".to_string(),
            "agent.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "target_project_id": "proj-123"
            }),
        );

        let result = handle_agent_create(&cmd, None).unwrap();
        let event = &result[0];

        // Event entity should be a new block_id (UUID format)
        assert!(!event.entity.is_empty());
        assert!(event.entity.contains('-')); // UUID format

        // Event attribute should be "{editor_id}/agent.create"
        assert_eq!(event.attribute, "alice/agent.create");

        // Event value should contain full Block initial state
        let value = &event.value;
        assert!(value.get("name").is_some());
        assert!(value.get("type").is_some());
        assert!(value.get("owner").is_some());
        assert!(value.get("contents").is_some());
        assert!(value.get("children").is_some());
        assert!(value.get("metadata").is_some());

        // Metadata should have timestamps
        let metadata = &value["metadata"];
        assert!(metadata.get("created_at").is_some());
        assert!(metadata.get("updated_at").is_some());
    }

    #[test]
    fn test_agent_create_v2_generates_unique_block_ids() {
        let cmd1 = Command::new(
            "alice".to_string(),
            "agent.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "target_project_id": "proj-1"
            }),
        );

        let cmd2 = Command::new(
            "alice".to_string(),
            "agent.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "target_project_id": "proj-2"
            }),
        );

        let result1 = handle_agent_create(&cmd1, None).unwrap();
        let result2 = handle_agent_create(&cmd2, None).unwrap();

        assert_ne!(result1[0].entity, result2[0].entity);
    }

    #[test]
    fn test_agent_create_v2_initial_status_is_enabled() {
        let cmd = Command::new(
            "alice".to_string(),
            "agent.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "target_project_id": "proj-123"
            }),
        );

        let result = handle_agent_create(&cmd, None).unwrap();
        assert_eq!(result[0].value["contents"]["status"], "enabled");
    }
}
