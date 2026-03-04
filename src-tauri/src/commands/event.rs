use crate::models::{Block, Grant};
use crate::services;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use specta::specta;
use specta::Type;
use tauri::State;

/// Full state snapshot at a specific point in time.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct StateSnapshot {
    /// Block state at that event
    pub block: Block,
    /// All grants in the system at that event
    pub grants: Vec<Grant>,
}

/// Get the full state snapshot (block + grants) at a specific event.
#[tauri::command]
#[specta]
pub async fn get_state_at_event(
    file_id: String,
    block_id: String,
    event_id: String,
    state: State<'_, AppState>,
) -> Result<StateSnapshot, String> {
    let handle = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or_else(|| format!("File '{}' is not open", file_id))?;

    let editor_id = state
        .get_active_editor(&file_id)
        .ok_or_else(|| "No active editor".to_string())?;

    let (block, grants) =
        services::event::get_state_at_event(&handle, &editor_id, &block_id, &event_id).await?;

    Ok(StateSnapshot { block, grants })
}

#[cfg(test)]
mod tests {
    use crate::engine::StateProjector;
    use crate::models::Event;
    use std::collections::HashMap;

    #[test]
    fn test_replay_events_to_target_point() {
        let mut projector = StateProjector::new();

        let events = vec![
            Event::new(
                "block1".to_string(),
                "system/core.create".to_string(),
                serde_json::json!({
                    "name": "Test Block",
                    "type": "document",
                    "owner": "system",
                    "contents": { "content": "Initial content" },
                    "children": {}
                }),
                {
                    let mut ts = HashMap::new();
                    ts.insert("system".to_string(), 1);
                    ts
                },
            ),
            Event::new(
                "block1".to_string(),
                "system/document.write".to_string(),
                serde_json::json!({
                    "contents": { "content": "Updated content v1" }
                }),
                {
                    let mut ts = HashMap::new();
                    ts.insert("system".to_string(), 2);
                    ts
                },
            ),
            Event::new(
                "block1".to_string(),
                "system/document.write".to_string(),
                serde_json::json!({
                    "contents": { "content": "Updated content v2" }
                }),
                {
                    let mut ts = HashMap::new();
                    ts.insert("system".to_string(), 3);
                    ts
                },
            ),
        ];

        projector.replay(events[..2].to_vec());

        let block = projector.get_block("block1").unwrap();
        let content = block
            .contents
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(content, "Updated content v1");
        assert_ne!(content, "Updated content v2");
    }

    #[test]
    fn test_find_event_by_id() {
        let events = vec![
            Event::new(
                "block1".to_string(),
                "system/core.create".to_string(),
                serde_json::json!({}),
                HashMap::new(),
            ),
            Event::new(
                "block1".to_string(),
                "system/document.write".to_string(),
                serde_json::json!({}),
                HashMap::new(),
            ),
        ];

        let target_id = &events[0].event_id;
        let index = events.iter().position(|e| &e.event_id == target_id);
        assert_eq!(index, Some(0));

        let target_id = &events[1].event_id;
        let index = events.iter().position(|e| &e.event_id == target_id);
        assert_eq!(index, Some(1));

        let index = events.iter().position(|e| &e.event_id == "nonexistent");
        assert_eq!(index, None);
    }
}
