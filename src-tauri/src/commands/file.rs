use crate::config;
use crate::services;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use specta::{specta, Type};
use std::fs;
use tauri::State;

/// File metadata for frontend display.
///
/// This structure contains all information about a file that the UI needs to display,
/// including the file name, path, collaborators (editors), and timestamps.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct FileMetadata {
    pub file_id: String,
    pub name: String,
    pub path: String,
    pub collaborators: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Ensure an active editor is set for the GUI session.
///
/// Called AFTER engine spawn. Picks one editor as the "active" editor for the UI.
/// This is GUI-specific (MCP connections use per-connection editor_id instead).
async fn ensure_active_editor(file_id: &str, state: &AppState) -> Result<(), String> {
    if state.get_active_editor(file_id).is_some() {
        return Ok(());
    }

    let handle = state
        .engine_manager
        .get_engine(file_id)
        .ok_or_else(|| format!("File '{}' is not open", file_id))?;

    let editors = handle.get_all_editors().await;

    if let Some((first_editor_id, _)) = editors.iter().next() {
        state.set_active_editor(file_id.to_string(), first_editor_id.clone());
    }

    Ok(())
}

/// Create a new .elf project and open it for editing.
///
/// # Arguments
/// * `path` - Absolute path to the project directory (will create .elf/ inside)
///
/// # Returns
/// * `Ok(file_id)` - Unique identifier for the opened project
/// * `Err(message)` - Error description if creation fails
#[tauri::command]
#[specta]
pub async fn create_file(path: String, state: State<'_, AppState>) -> Result<String, String> {
    let file_id = services::project::open_project(&path, &state).await?;
    // GUI-specific: set active editor for UI session
    ensure_active_editor(&file_id, &state).await?;
    Ok(file_id)
}

/// Open an existing .elf project for editing.
///
/// # Arguments
/// * `path` - Absolute path to the project directory (containing .elf/)
///
/// # Returns
/// * `Ok(file_id)` - Unique identifier for the opened project
/// * `Err(message)` - Error description if opening fails
#[tauri::command]
#[specta]
pub async fn open_file(path: String, state: State<'_, AppState>) -> Result<String, String> {
    let file_id = services::project::open_project(&path, &state).await?;
    // GUI-specific: set active editor for UI session
    ensure_active_editor(&file_id, &state).await?;
    Ok(file_id)
}

/// Close a project and release associated resources.
///
/// This shuts down the engine actor and removes the project from memory.
///
/// # Arguments
/// * `file_id` - Unique identifier of the project to close
///
/// # Returns
/// * `Ok(())` - Project closed successfully
/// * `Err(message)` - Error description if close fails
#[tauri::command]
#[specta]
pub async fn close_file(file_id: String, state: State<'_, AppState>) -> Result<(), String> {
    services::project::close_project_by_id(&file_id, &state).await
}

/// Get list of all currently open projects.
///
/// # Returns
/// * `Ok(Vec<file_id>)` - List of file IDs currently open
/// * `Err(message)` - Error description if retrieval fails
#[tauri::command]
#[specta]
pub async fn list_open_files(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let file_ids: Vec<String> = state
        .files
        .iter()
        .map(|entry| entry.key().clone())
        .collect();

    Ok(file_ids)
}

/// Get all events for a specific project.
///
/// This command filters events based on permissions:
/// - Block events: Only returns events for blocks where user has read permission
///   (uses `{block_type}.read` convention, e.g., document.read, task.read, session.read)
/// - Editor events: Always returned (project-level information, similar to Git collaborators)
///
/// # Arguments
/// * `file_id` - Unique identifier of the project
/// * `editor_id` - Optional editor ID (defaults to active editor)
///
/// # Returns
/// * `Ok(Vec<Event>)` - List of events the user has permission to view
/// * `Err(message)` - Error description if retrieval fails
#[tauri::command]
#[specta]
pub async fn get_all_events(
    file_id: String,
    editor_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::Event>, String> {
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

    services::event::list_events(&handle, &effective_editor_id).await
}

/// Get detailed information about a project.
///
/// Returns metadata including project name, path, collaborators (editors),
/// and timestamps.
///
/// # Arguments
/// * `file_id` - Unique identifier of the project
///
/// # Returns
/// * `Ok(FileMetadata)` - Project metadata
/// * `Err(message)` - Error description if retrieval fails
#[tauri::command]
#[specta]
pub async fn get_file_info(
    file_id: String,
    state: State<'_, AppState>,
) -> Result<FileMetadata, String> {
    // Get project info from state
    let file_info = state
        .files
        .get(&file_id)
        .ok_or_else(|| format!("File '{}' not found", file_id))?;

    let project = &file_info.project;

    // Get project path and name from config
    let path = project.project_dir().to_string_lossy().to_string();
    let name = project.config().project.name.clone();

    // Get timestamps from eventstore.db file
    let db_metadata = fs::metadata(project.db_path()).ok();

    let created_at = db_metadata
        .as_ref()
        .and_then(|m| m.created().ok())
        .and_then(|t| crate::utils::time::system_time_to_utc(t).ok())
        .unwrap_or_else(|| "Unknown".to_string());

    let updated_at = db_metadata
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| crate::utils::time::system_time_to_utc(t).ok())
        .unwrap_or_else(|| "Unknown".to_string());

    drop(file_info);

    // Get collaborators (editors) from engine
    let handle = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or_else(|| format!("File '{}' is not open", file_id))?;

    let editors_map = handle.get_all_editors().await;
    let collaborators: Vec<String> = editors_map.keys().cloned().collect();

    Ok(FileMetadata {
        file_id,
        name,
        path,
        collaborators,
        created_at,
        updated_at,
    })
}

/// Rename a project directory.
///
/// This updates the project directory name both on the filesystem and in the application state.
///
/// # Arguments
/// * `file_id` - Unique identifier of the project
/// * `new_name` - New name for the project directory
///
/// # Returns
/// * `Ok(())` - Project renamed successfully
/// * `Err(message)` - Error description if rename fails
#[tauri::command]
#[specta]
pub async fn rename_file(
    file_id: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Validate new name
    if new_name.is_empty() {
        return Err("File name cannot be empty".to_string());
    }

    if new_name.contains(&['/', '\\', ':', '*', '?', '"', '<', '>', '|'][..]) {
        return Err("File name contains invalid characters".to_string());
    }

    // Prevent path traversal attacks
    if new_name.contains("..") || new_name.starts_with('.') {
        return Err("File name cannot contain relative path components".to_string());
    }

    // For directory-based projects, renaming the project directory is complex
    // (requires re-opening the project). For now, just update the config name.
    let file_info = state
        .files
        .get(&file_id)
        .ok_or_else(|| format!("File '{}' not found", file_id))?;

    let config_path = file_info.project.elf_dir().join("config.toml");
    drop(file_info);

    // Update config.toml with new project name
    let mut config = crate::elf_project::config::ProjectConfig::load(&config_path)?;
    config.project.name = new_name;
    config.save(&config_path)?;

    Ok(())
}

/// Get the global system editor ID from config.
///
/// This returns the persistent system editor ID that is stored in
/// the user's home directory config file (`$USER_HOME/.elf/config.json`).
///
/// # Returns
/// * `Ok(String)` - The system editor ID (UUID)
/// * `Err(message)` - Error if config cannot be read
#[tauri::command]
#[specta]
pub async fn get_system_editor_id_from_config() -> Result<String, String> {
    config::get_system_editor_id()
}

#[cfg(test)]
mod tests {
    /// Helper function to validate filename
    fn validate_filename(name: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("File name cannot be empty".to_string());
        }

        if name.contains(&['/', '\\', ':', '*', '?', '"', '<', '>', '|'][..]) {
            return Err("File name contains invalid characters".to_string());
        }

        // Prevent path traversal attacks
        if name.contains("..") || name.starts_with('.') {
            return Err("File name cannot contain relative path components".to_string());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_rename_file_validation() {
        // Test empty name
        let result = validate_filename("");
        assert!(result.is_err(), "Empty name should be rejected");
        assert_eq!(result.unwrap_err(), "File name cannot be empty");

        // Test invalid characters
        let invalid_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
        for ch in invalid_chars {
            let name = format!("test{}name", ch);
            let result = validate_filename(&name);
            assert!(result.is_err(), "Name with '{}' should be rejected", ch);
            assert!(result.unwrap_err().contains("invalid characters"));
        }

        // Test path traversal attempts - names containing ".."
        let double_dot_names = vec!["..", "file..name", "..file", "file.."];
        for name in double_dot_names {
            let result = validate_filename(name);
            assert!(
                result.is_err(),
                "Name with '..' ('{}') should be rejected",
                name
            );
            let err_msg = result.unwrap_err();
            assert!(
                err_msg.contains("relative path components"),
                "Expected 'relative path components' error for '{}', got: {}",
                name,
                err_msg
            );
        }

        // Test path traversal attempts - names starting with "."
        let dot_names = vec![".hidden", ".config", "."];
        for name in dot_names {
            let result = validate_filename(name);
            assert!(
                result.is_err(),
                "Name starting with '.' ('{}') should be rejected",
                name
            );
            let err_msg = result.unwrap_err();
            assert!(
                err_msg.contains("relative path components"),
                "Expected 'relative path components' error for '{}', got: {}",
                name,
                err_msg
            );
        }

        // Test valid name
        let result = validate_filename("valid-file_name123");
        assert!(result.is_ok(), "Valid name should be accepted");
    }
}
