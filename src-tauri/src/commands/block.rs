use crate::models::{Block, Command, Event};
use crate::services;
use crate::state::AppState;
use specta::specta;
use tauri::State;

/// Execute a command on a block in the specified file.
#[tauri::command]
#[specta]
pub async fn execute_command(
    file_id: String,
    cmd: Command,
    state: State<'_, AppState>,
) -> Result<Vec<Event>, String> {
    let handle = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or_else(|| format!("File '{}' is not open", file_id))?;

    let result = services::block::execute_command(&handle, cmd).await;

    // Notify frontend of state change on success
    if result.is_ok() {
        let _ = state.state_changed_tx.send(file_id);
    }

    result
}

/// Get a specific block by ID from a file (CBAC: {block_type}.read).
#[tauri::command]
#[specta]
pub async fn get_block(
    file_id: String,
    block_id: String,
    editor_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Block, String> {
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

    services::block::get_block(&handle, &effective_editor_id, &block_id).await
}

/// Get all blocks from a file (CBAC filtered).
#[tauri::command]
#[specta]
pub async fn get_all_blocks(
    file_id: String,
    editor_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Block>, String> {
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

    Ok(services::block::list_blocks(&handle, &effective_editor_id).await)
}

/// Rename a block (via services layer).
#[tauri::command]
#[specta]
pub async fn rename_block(
    file_id: String,
    block_id: String,
    name: String,
    state: State<'_, AppState>,
) -> Result<Vec<Event>, String> {
    let handle = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or_else(|| format!("File '{}' is not open", file_id))?;

    let editor_id = state
        .get_active_editor(&file_id)
        .ok_or_else(|| "No active editor".to_string())?;

    services::block::rename_block(&handle, &editor_id, &block_id, &name).await
}

/// Check if current editor has permission for a capability on a block.
#[tauri::command]
#[specta]
pub async fn check_permission(
    file_id: String,
    block_id: String,
    capability: String,
    editor_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<bool, String> {
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

    Ok(handle
        .check_grant(effective_editor_id, capability, block_id)
        .await)
}
