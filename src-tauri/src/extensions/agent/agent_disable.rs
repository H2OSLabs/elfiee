//! Handler for agent.disable capability
//!
//! Updates an Agent Block's status to Disabled.
//! The handler generates the state change event; actual I/O (symlink cleanup, MCP config removal)
//! is performed by the Tauri command layer.

use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, Event};
use crate::utils::time::now_utc;
use capability_macros::capability;

use super::{AgentContents, AgentStatus};

/// Handler for agent.disable capability.
///
/// Updates an Agent Block's status to Disabled:
/// 1. Validates that the block exists and is of type "agent"
/// 2. Deserializes current AgentContents
/// 3. Updates status to Disabled (even if already disabled — idempotent)
/// 4. Records last_disabled_at in metadata
///
/// Idempotent: disabling an already-disabled agent succeeds silently.
#[capability(id = "agent.disable", target = "agent")]
fn handle_agent_disable(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for agent.disable")?;

    if block.block_type != "agent" {
        return Err(format!(
            "Expected block_type 'agent', got '{}'",
            block.block_type
        ));
    }

    // Deserialize current contents
    let mut contents: AgentContents = serde_json::from_value(block.contents.clone())
        .map_err(|e| format!("Invalid AgentContents in block: {}", e))?;

    // Update status
    contents.status = AgentStatus::Disabled;

    let now = now_utc();

    // Build updated metadata
    let mut new_metadata = block.metadata.clone();
    new_metadata.touch();

    let mut custom = new_metadata.custom.clone();
    custom.insert("last_disabled_at".to_string(), serde_json::json!(now));

    // Generate event
    let event = create_event(
        block.block_id.clone(),
        "agent.disable",
        serde_json::json!({
            "contents": serde_json::to_value(&contents).unwrap(),
            "metadata": {
                "updated_at": now,
                "custom": custom,
            }
        }),
        &cmd.editor_id,
        1,
    );

    Ok(vec![event])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::agent::{AgentContents, AgentStatus};
    use crate::models::{Block, Command};

    fn make_agent_block(status: AgentStatus) -> Block {
        let contents = AgentContents {
            name: "elfiee".to_string(),
            target_project_id: "proj-456".to_string(),
            status,
        };

        let mut block = Block::new(
            "Test Agent".to_string(),
            "agent".to_string(),
            "alice".to_string(),
        );
        block.contents = serde_json::to_value(&contents).unwrap();
        block
    }

    #[test]
    fn test_disable_success_from_enabled() {
        let block = make_agent_block(AgentStatus::Enabled);
        let cmd = Command::new(
            "alice".to_string(),
            "agent.disable".to_string(),
            block.block_id.clone(),
            serde_json::json!({}),
        );

        let result = handle_agent_disable(&cmd, Some(&block));
        assert!(result.is_ok());

        let events = result.unwrap();
        assert_eq!(events.len(), 1);

        let event = &events[0];
        assert_eq!(event.entity, block.block_id);
        assert_eq!(event.value["contents"]["status"], "disabled");
    }

    #[test]
    fn test_disable_idempotent_already_disabled() {
        let block = make_agent_block(AgentStatus::Disabled);
        let cmd = Command::new(
            "alice".to_string(),
            "agent.disable".to_string(),
            block.block_id.clone(),
            serde_json::json!({}),
        );

        let result = handle_agent_disable(&cmd, Some(&block));
        assert!(result.is_ok());

        let events = result.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].value["contents"]["status"], "disabled");
    }

    #[test]
    fn test_disable_block_not_found() {
        let cmd = Command::new(
            "alice".to_string(),
            "agent.disable".to_string(),
            "nonexistent".to_string(),
            serde_json::json!({}),
        );

        let result = handle_agent_disable(&cmd, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Block required"));
    }

    #[test]
    fn test_disable_wrong_block_type() {
        let block = Block::new(
            "Not Agent".to_string(),
            "markdown".to_string(),
            "alice".to_string(),
        );
        let cmd = Command::new(
            "alice".to_string(),
            "agent.disable".to_string(),
            block.block_id.clone(),
            serde_json::json!({}),
        );

        let result = handle_agent_disable(&cmd, Some(&block));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Expected block_type 'agent'"));
    }

    #[test]
    fn test_disable_metadata_update() {
        let block = make_agent_block(AgentStatus::Enabled);
        let cmd = Command::new(
            "alice".to_string(),
            "agent.disable".to_string(),
            block.block_id.clone(),
            serde_json::json!({}),
        );

        let result = handle_agent_disable(&cmd, Some(&block)).unwrap();
        let event = &result[0];

        let metadata = &event.value["metadata"];
        assert!(metadata.get("updated_at").is_some());
        assert!(metadata["custom"].get("last_disabled_at").is_some());
    }

    #[test]
    fn test_disable_preserves_agent_contents_fields() {
        let block = make_agent_block(AgentStatus::Enabled);
        let cmd = Command::new(
            "alice".to_string(),
            "agent.disable".to_string(),
            block.block_id.clone(),
            serde_json::json!({}),
        );

        let result = handle_agent_disable(&cmd, Some(&block)).unwrap();
        let contents = &result[0].value["contents"];

        assert_eq!(contents["name"], "elfiee");
        assert_eq!(contents["target_project_id"], "proj-456");
        assert_eq!(contents["status"], "disabled");
    }
}
