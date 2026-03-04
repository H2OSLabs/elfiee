use crate::config;
use crate::models::{Editor, Grant};
use crate::services;
use crate::state::AppState;
use specta::specta;
use tauri::State;

/// Create a new editor for the specified file.
#[tauri::command]
#[specta]
pub async fn create_editor(
    file_id: String,
    name: String,
    editor_type: Option<String>,
    block_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Editor, String> {
    let handle = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or_else(|| format!("File '{}' is not open", file_id))?;

    let creator_editor_id = state
        .get_active_editor(&file_id)
        .unwrap_or_else(|| config::get_system_editor_id().unwrap_or_else(|_| "system".to_string()));

    // Permission check: If block_id is provided, only block owner can create editors
    if let Some(ref bid) = block_id {
        if let Some(block) = handle.get_block(bid.clone()).await {
            if block.owner != creator_editor_id {
                return Err(format!(
                    "Permission denied: Only the block owner can create editors for this block. Block owner is '{}', but current editor is '{}'",
                    block.owner, creator_editor_id
                ));
            }
        } else {
            return Err(format!("Block '{}' not found", bid));
        }
    }

    services::editor::create_editor(
        &handle,
        &creator_editor_id,
        &name,
        editor_type.as_deref(),
        None,
    )
    .await
}

/// Delete an editor identity from the specified file.
#[tauri::command]
#[specta]
pub async fn delete_editor(
    file_id: String,
    editor_id: String,
    block_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let handle = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or_else(|| format!("File '{}' is not open", file_id))?;

    let active_editor_id = state
        .get_active_editor(&file_id)
        .unwrap_or_else(|| config::get_system_editor_id().unwrap_or_else(|_| "system".to_string()));

    // Permission check: If block_id is provided, only block owner can delete editors
    if let Some(ref bid) = block_id {
        if let Some(block) = handle.get_block(bid.clone()).await {
            if block.owner != active_editor_id {
                return Err(format!(
                    "Permission denied: Only the block owner can delete editors for this block. Block owner is '{}', but current editor is '{}'",
                    block.owner, active_editor_id
                ));
            }
        } else {
            return Err(format!("Block '{}' not found", bid));
        }
    }

    services::editor::delete_editor(&handle, &active_editor_id, &editor_id).await?;

    // If we just deleted the active editor, switch to another editor
    if let Some(current_active) = state.get_active_editor(&file_id) {
        if current_active == editor_id {
            let editors = handle.get_all_editors().await;
            let new_active = editors
                .values()
                .find(|e| e.editor_id != editor_id)
                .map(|e| e.editor_id.clone())
                .or_else(|| config::get_system_editor_id().ok());

            if let Some(new_editor_id) = new_active {
                state.set_active_editor(file_id.clone(), new_editor_id);
            }
        }
    }

    Ok(())
}

/// List all editors for the specified file.
#[tauri::command]
#[specta]
pub async fn list_editors(
    file_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Editor>, String> {
    let handle = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or_else(|| format!("File '{}' is not open", file_id))?;

    Ok(services::editor::list_editors(&handle).await)
}

/// Get a specific editor by ID.
#[tauri::command]
#[specta]
pub async fn get_editor(
    file_id: String,
    editor_id: String,
    state: State<'_, AppState>,
) -> Result<Editor, String> {
    let handle = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or_else(|| format!("File '{}' is not open", file_id))?;

    services::editor::get_editor(&handle, &editor_id).await
}

/// Set the active editor for the specified file (UI state, not persisted).
#[tauri::command]
#[specta]
pub async fn set_active_editor(
    file_id: String,
    editor_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let _ = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or_else(|| format!("File '{}' is not open", file_id))?;

    state.set_active_editor(file_id, editor_id);
    Ok(())
}

/// Get the currently active editor for the specified file.
#[tauri::command]
#[specta]
pub async fn get_active_editor(
    file_id: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let _ = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or_else(|| format!("File '{}' is not open", file_id))?;

    Ok(state.get_active_editor(&file_id))
}

/// List all grants for the specified file (CBAC filtered).
#[tauri::command]
#[specta]
pub async fn list_grants(
    file_id: String,
    editor_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Grant>, String> {
    let handle = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or_else(|| format!("File '{}' is not open", file_id))?;

    let effective_editor_id = if let Some(id) = editor_id {
        id
    } else {
        state
            .get_active_editor(&file_id)
            .ok_or_else(|| "No active editor".to_string())?
    };

    Ok(services::grant::list_grants(&handle, &effective_editor_id).await)
}

/// Get grants for a specific block.
#[tauri::command]
#[specta]
pub async fn get_block_grants(
    file_id: String,
    block_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Grant>, String> {
    let handle = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or_else(|| format!("File '{}' is not open", file_id))?;

    Ok(services::grant::get_block_grants(&handle, &block_id).await)
}

#[cfg(test)]
mod tests {
    use crate::capabilities::registry::CapabilityRegistry;
    use crate::engine::{EventPoolWithPath, EventStore};
    use crate::models::{Command, Event};
    use crate::state::AppState;
    use std::collections::HashMap;

    /// Seed bootstrap events for a test editor directly to EventStore.
    async fn seed_test_editor(event_pool: &EventPoolWithPath, editor_id: &str) {
        let registry = CapabilityRegistry::new();
        let cap_ids = registry.get_grantable_cap_ids(&[]);
        let mut events = Vec::new();

        let mut ts = HashMap::new();
        ts.insert(editor_id.to_string(), 1);
        events.push(Event::new(
            editor_id.to_string(),
            format!("{}/editor.create", editor_id),
            serde_json::json!({
                "editor_id": editor_id,
                "name": editor_id,
                "editor_type": "Human"
            }),
            ts,
        ));

        for (i, cap_id) in cap_ids.iter().enumerate() {
            let mut grant_ts = HashMap::new();
            grant_ts.insert(editor_id.to_string(), (i + 2) as i64);
            events.push(Event::new(
                "*".to_string(),
                format!("{}/core.grant", editor_id),
                serde_json::json!({
                    "editor": editor_id,
                    "capability": cap_id,
                    "block": "*"
                }),
                grant_ts,
            ));
        }

        EventStore::append_events(&event_pool.pool, &events)
            .await
            .unwrap();
    }

    /// Set up a test environment with bootstrapped system editor, engine, and a block.
    async fn setup_test_environment() -> (AppState, String, String, String) {
        let event_pool = EventStore::create(":memory:")
            .await
            .expect("Failed to create test pool");

        let system_editor_id = "system".to_string();
        seed_test_editor(&event_pool, &system_editor_id).await;

        let state = AppState::new();
        let file_id = "test-file".to_string();

        state
            .engine_manager
            .spawn_engine(file_id.clone(), event_pool)
            .await
            .unwrap();

        state.set_active_editor(file_id.clone(), system_editor_id.clone());

        let handle = state.engine_manager.get_engine(&file_id).unwrap();
        let create_block_cmd = Command::new(
            system_editor_id.clone(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "block_type": "document",
                "name": "Test Block"
            }),
        );
        let block_events = handle.process_command(create_block_cmd).await.unwrap();
        let block_id = block_events[0].entity.clone();

        (state, file_id, block_id, system_editor_id)
    }

    #[tokio::test]
    async fn test_create_editor_without_block_id_allows() {
        let (state, file_id, _, _) = setup_test_environment().await;
        let handle = state.engine_manager.get_engine(&file_id).unwrap();

        let create_cmd = Command::new(
            "system".to_string(),
            "editor.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": "New Editor",
                "editor_type": "Human"
            }),
        );
        let events = handle.process_command(create_cmd).await.unwrap();
        assert!(!events.is_empty());
        assert_eq!(events[0].value["name"], "New Editor");
    }

    #[tokio::test]
    async fn test_create_editor_with_block_id_as_owner_allows() {
        let (state, file_id, block_id, owner_id) = setup_test_environment().await;
        let handle = state.engine_manager.get_engine(&file_id).unwrap();
        let block = handle.get_block(block_id.clone()).await.unwrap();
        assert_eq!(block.owner, owner_id);
    }

    #[tokio::test]
    async fn test_create_editor_with_block_id_as_non_owner_denies() {
        let (state, file_id, block_id, _) = setup_test_environment().await;
        let handle = state.engine_manager.get_engine(&file_id).unwrap();

        let create_non_owner_cmd = Command::new(
            "system".to_string(),
            "editor.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "editor_id": "non-owner",
                "name": "Non Owner",
                "editor_type": "Human"
            }),
        );
        handle.process_command(create_non_owner_cmd).await.unwrap();

        let block = handle.get_block(block_id).await.unwrap();
        assert_ne!(block.owner, "non-owner");
    }

    #[tokio::test]
    async fn test_create_editor_with_nonexistent_block_id_denies() {
        let (state, file_id, _, _) = setup_test_environment().await;
        let handle = state.engine_manager.get_engine(&file_id).unwrap();
        let block = handle.get_block("nonexistent-block".to_string()).await;
        assert!(block.is_none());
    }

    #[tokio::test]
    async fn test_delete_editor_without_block_id_allows() {
        let (state, file_id, _, _) = setup_test_environment().await;
        let handle = state.engine_manager.get_engine(&file_id).unwrap();

        let create_editor_cmd = Command::new(
            "system".to_string(),
            "editor.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "editor_id": "editor-to-delete",
                "name": "Editor To Delete",
                "editor_type": "Human"
            }),
        );
        handle.process_command(create_editor_cmd).await.unwrap();

        let delete_cmd = Command::new(
            "system".to_string(),
            "editor.delete".to_string(),
            "".to_string(),
            serde_json::json!({
                "editor_id": "editor-to-delete"
            }),
        );
        let result = handle.process_command(delete_cmd).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_editor_with_block_id_as_owner_allows() {
        let (state, file_id, block_id, owner_id) = setup_test_environment().await;
        let handle = state.engine_manager.get_engine(&file_id).unwrap();
        let block = handle.get_block(block_id).await.unwrap();
        assert_eq!(block.owner, owner_id);
    }

    #[tokio::test]
    async fn test_delete_editor_with_block_id_as_non_owner_denies() {
        let (state, file_id, block_id, _) = setup_test_environment().await;
        let handle = state.engine_manager.get_engine(&file_id).unwrap();

        let create_non_owner_cmd = Command::new(
            "system".to_string(),
            "editor.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "editor_id": "non-owner",
                "name": "Non Owner",
                "editor_type": "Human"
            }),
        );
        handle.process_command(create_non_owner_cmd).await.unwrap();

        let block = handle.get_block(block_id).await.unwrap();
        assert_ne!(block.owner, "non-owner");
    }

    #[tokio::test]
    async fn test_delete_editor_with_nonexistent_block_id_denies() {
        let (state, file_id, _, _) = setup_test_environment().await;
        let handle = state.engine_manager.get_engine(&file_id).unwrap();
        let block = handle.get_block("nonexistent-block".to_string()).await;
        assert!(block.is_none());
    }
}
