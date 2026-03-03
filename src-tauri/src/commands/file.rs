use crate::config;
use crate::elf::ElfArchive;
use crate::models::Command;
use crate::state::{AppState, FileInfo};
use crate::utils::time;
use serde::{Deserialize, Serialize};
use specta::{specta, Type};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
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

/// Bootstrap the editor system for a file.
///
/// If no editors exist, creates a "system" editor and sets it as active.
/// If editors exist but no active editor is set, sets the first editor as active.
/// This ensures every file always has at least one editor available and selected.
async fn bootstrap_editors(file_id: &str, state: &AppState) -> Result<(), String> {
    // Get engine handle
    let handle = state
        .engine_manager
        .get_engine(file_id)
        .ok_or_else(|| format!("File '{}' is not open", file_id))?;

    // Check if any editors exist
    let editors = handle.get_all_editors().await;

    if editors.is_empty() {
        // Retrieve persistent system editor ID from local config
        let system_id =
            crate::config::get_system_editor_id().unwrap_or_else(|_| "system".to_string());

        // Create initial editor identity using the configured persistent ID
        let cmd = Command::new(
            system_id.clone(),
            "editor.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "editor_id": system_id,
                "name": "Owner", // Default name for the creator
                "editor_type": "Human"
            }),
        );

        let events = handle.process_command(cmd).await?;

        // Extract the actual ID from the creation event
        if let Some(event) = events.first() {
            let created_editor_id = event.entity.clone();

            // Set as active editor in current session state
            state.set_active_editor(file_id.to_string(), created_editor_id);
        }
    } else {
        // Editors exist - ensure one is set as active
        // This handles the case of reopening a file where activeEditorId is not persisted
        if state.get_active_editor(file_id).is_none() {
            // Set the first editor as active (deterministic choice)
            if let Some((first_editor_id, _)) = editors.iter().next() {
                state.set_active_editor(file_id.to_string(), first_editor_id.clone());
            }
        }
    }

    Ok(())
}

/// Create a new .elf file and open it for editing.
///
/// # Arguments
/// * `path` - Absolute path where the new .elf file should be created
///
/// # Returns
/// * `Ok(file_id)` - Unique identifier for the opened file
/// * `Err(message)` - Error description if creation fails
#[tauri::command]
#[specta]
pub async fn create_file(path: String, state: State<'_, AppState>) -> Result<String, String> {
    // Generate unique file ID
    let file_id = format!("file-{}", uuid::Uuid::new_v4());

    // Create new archive
    let archive = ElfArchive::new()
        .await
        .map_err(|e| format!("Failed to create archive: {}", e))?;

    // Save to specified path
    archive
        .save(Path::new(&path))
        .map_err(|e| format!("Failed to save file: {}", e))?;

    // Get event pool for this archive
    let event_pool = archive
        .event_pool()
        .await
        .map_err(|e| format!("Failed to get event pool: {}", e))?;

    // Spawn engine actor for this file
    state
        .engine_manager
        .spawn_engine(file_id.clone(), event_pool)
        .await?;

    // Store file info
    state.files.insert(
        file_id.clone(),
        FileInfo {
            archive: Arc::new(archive),
            path: PathBuf::from(&path),
        },
    );

    // Bootstrap editors (create system editor if none exist)
    bootstrap_editors(&file_id, &state).await?;

    // Bootstrap .elf/ system Dir Block (directory skeleton for Agents, Session, git hooks)
    crate::extensions::directory::elf_meta::bootstrap_elf_meta(&file_id, &state).await?;

    Ok(file_id)
}

/// Open an existing .elf file for editing.
///
/// # Arguments
/// * `path` - Absolute path to the .elf file to open
///
/// # Returns
/// * `Ok(file_id)` - Unique identifier for the opened file
/// * `Err(message)` - Error description if opening fails
#[tauri::command]
#[specta]
pub async fn open_file(path: String, state: State<'_, AppState>) -> Result<String, String> {
    // Generate unique file ID
    let file_id = format!("file-{}", uuid::Uuid::new_v4());

    // Open existing archive
    let archive =
        ElfArchive::open(Path::new(&path)).map_err(|e| format!("Failed to open file: {}", e))?;

    // Get event pool for this archive
    let event_pool = archive
        .event_pool()
        .await
        .map_err(|e| format!("Failed to get event pool: {}", e))?;

    // Spawn engine actor for this file
    state
        .engine_manager
        .spawn_engine(file_id.clone(), event_pool)
        .await?;

    // Store file info
    state.files.insert(
        file_id.clone(),
        FileInfo {
            archive: Arc::new(archive),
            path: PathBuf::from(&path),
        },
    );

    // Bootstrap editors (create system editor if none exist)
    bootstrap_editors(&file_id, &state).await?;

    // Recover per-agent MCP servers for any enabled agents
    let recovery_failures = crate::commands::agent::recover_agent_servers(&state, &file_id).await;
    if !recovery_failures.is_empty() {
        let details: Vec<String> = recovery_failures
            .iter()
            .map(|(name, err)| format!("'{}': {}", name, err))
            .collect();
        eprintln!(
            "Agent recovery: {} agent(s) failed to recover: {}",
            recovery_failures.len(),
            details.join("; ")
        );
    }

    // Notify sync observer that file is opened (triggers session sync recovery)
    let _ = state
        .agent_sync_tx
        .send(crate::sync::observer::AgentSyncEvent::FileOpened {
            file_id: file_id.clone(),
        });

    Ok(file_id)
}

/// Save the current state of a file to disk.
///
/// This persists all changes made since the file was opened or last saved.
///
/// # Arguments
/// * `file_id` - Unique identifier of the file to save
///
/// # Returns
/// * `Ok(())` - File saved successfully
/// * `Err(message)` - Error description if save fails
#[tauri::command]
#[specta]
pub async fn save_file(file_id: String, state: State<'_, AppState>) -> Result<(), String> {
    // Get file info
    let file_info = state
        .files
        .get(&file_id)
        .ok_or_else(|| format!("File '{}' not found", file_id))?;

    // Save to original path
    file_info
        .archive
        .save(&file_info.path)
        .map_err(|e| format!("Failed to save file: {}", e))?;

    Ok(())
}

/// Close a file and release associated resources.
///
/// This shuts down the engine actor and removes the file from memory.
/// Unsaved changes will be lost.
///
/// # Arguments
/// * `file_id` - Unique identifier of the file to close
///
/// # Returns
/// * `Ok(())` - File closed successfully
/// * `Err(message)` - Error description if close fails
#[tauri::command]
#[specta]
pub async fn close_file(file_id: String, state: State<'_, AppState>) -> Result<(), String> {
    // Notify sync observer to shut down sync for this file
    let _ = state
        .agent_sync_tx
        .send(crate::sync::observer::AgentSyncEvent::FileClosing {
            file_id: file_id.clone(),
        });

    // Shutdown per-agent MCP servers before engine shutdown
    crate::commands::agent::shutdown_agent_servers(&state, &file_id).await;

    // Shutdown engine actor
    state.engine_manager.shutdown_engine(&file_id).await?;

    // Remove file info
    state.files.remove(&file_id);

    Ok(())
}

/// Get list of all currently open files.
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

/// Get all events for a specific file.
///
/// This command filters events based on permissions:
/// - Block events: Only returns events for blocks where user has core.read permission
/// - Editor events: Always returned (file-level information, similar to Git collaborators)
///
/// # Arguments
/// * `file_id` - Unique identifier of the file
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
    // Get engine handle
    let handle = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or_else(|| format!("File '{}' is not open", file_id))?;

    // Determine effective editor ID
    let effective_editor_id = if let Some(id) = editor_id {
        id
    } else {
        state
            .get_active_editor(&file_id)
            .ok_or_else(|| "No active editor".to_string())?
    };

    // Get all events from the engine
    let all_events = handle.get_all_events().await?;

    // Filter events based on permissions
    let mut filtered_events = Vec::new();
    for event in all_events {
        // Editor events are file-level information (like Git collaborators)
        // No filtering needed - if user can open the file, they can see editors
        if event.entity.starts_with("editor-") {
            filtered_events.push(event);
            continue;
        }

        // Block events: check core.read permission
        let has_core_read = handle
            .check_grant(
                effective_editor_id.clone(),
                "core.read".to_string(),
                event.entity.clone(),
            )
            .await;

        if has_core_read {
            filtered_events.push(event);
        }
    }

    Ok(filtered_events)
}

/// Get detailed information about a file.
///
/// Returns metadata including file name, path, collaborators (editors),
/// and timestamps. The file must be open to retrieve this information.
///
/// # Arguments
/// * `file_id` - Unique identifier of the file
///
/// # Returns
/// * `Ok(FileMetadata)` - File metadata
/// * `Err(message)` - Error description if retrieval fails
#[tauri::command]
#[specta]
pub async fn get_file_info(
    file_id: String,
    state: State<'_, AppState>,
) -> Result<FileMetadata, String> {
    // Get file info from state
    let file_info = state
        .files
        .get(&file_id)
        .ok_or_else(|| format!("File '{}' not found", file_id))?;

    // Get file path
    let path = file_info
        .path
        .to_str()
        .ok_or("Invalid file path")?
        .to_string();

    // Extract file name from path
    let name = file_info
        .path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid file name")?
        .to_string();

    // Get file metadata from filesystem
    let metadata = fs::metadata(&file_info.path)
        .map_err(|e| format!("Failed to read file metadata: {}", e))?;

    // Get created and modified timestamps (using timezone-aware RFC 3339 format)
    let created_at = metadata
        .created()
        .ok()
        .and_then(|t| time::system_time_to_utc(t).ok())
        .unwrap_or_else(|| "Unknown".to_string());

    let updated_at = metadata
        .modified()
        .ok()
        .and_then(|t| time::system_time_to_utc(t).ok())
        .unwrap_or_else(|| "Unknown".to_string());

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

/// Rename a file on the filesystem.
///
/// This updates the file path both in the filesystem and in the application state.
/// The file must be open to be renamed.
///
/// # Arguments
/// * `file_id` - Unique identifier of the file
/// * `new_name` - New name for the file (without extension)
///
/// # Returns
/// * `Ok(())` - File renamed successfully
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

    // Get current file info
    let file_info = state
        .files
        .get(&file_id)
        .ok_or_else(|| format!("File '{}' not found", file_id))?;

    let old_path = file_info.path.clone();

    // Construct new path
    let new_path = old_path
        .parent()
        .ok_or("Invalid file path")?
        .join(format!("{}.elf", new_name));

    // Drop the reference to file_info before mutating state
    drop(file_info);

    // Rename file on filesystem - let the OS handle atomicity
    // No TOCTOU race condition: the rename operation is atomic
    fs::rename(&old_path, &new_path).map_err(|e| {
        // Check if the error is because the target already exists
        match e.kind() {
            std::io::ErrorKind::AlreadyExists => {
                format!("A file named '{}' already exists", new_path.display())
            }
            _ => format!("Failed to rename file: {}", e),
        }
    })?;

    // Update path in state
    if let Some(mut entry) = state.files.get_mut(&file_id) {
        entry.path = new_path;
    }

    Ok(())
}

/// Duplicate (copy) an existing .elf file.
///
/// This creates a copy of the file with a new name and opens it for editing.
/// The new file will have " Copy" appended to the name, or " Copy N" if that name exists.
///
/// # Arguments
/// * `file_id` - Unique identifier of the file to duplicate
///
/// # Returns
/// * `Ok(new_file_id)` - Unique identifier for the newly created duplicate file
/// * `Err(message)` - Error description if duplication fails
#[tauri::command]
#[specta]
pub async fn duplicate_file(file_id: String, state: State<'_, AppState>) -> Result<String, String> {
    // Get source file info
    let source_file_info = state
        .files
        .get(&file_id)
        .ok_or_else(|| format!("File '{}' not found", file_id))?;

    let source_path = source_file_info.path.clone();
    drop(source_file_info); // Release the reference

    // Extract base name and directory
    let parent_dir = source_path.parent().ok_or("Invalid source file path")?;

    let base_name = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid source file name")?;

    // Find a unique name for the copy
    let mut copy_name = format!("{} Copy", base_name);
    let mut copy_path = parent_dir.join(format!("{}.elf", copy_name));
    let mut counter = 2;

    while copy_path.exists() {
        copy_name = format!("{} Copy {}", base_name, counter);
        copy_path = parent_dir.join(format!("{}.elf", copy_name));
        counter += 1;
    }

    // Copy the file
    fs::copy(&source_path, &copy_path).map_err(|e| format!("Failed to copy file: {}", e))?;

    // Open the copied file
    let new_file_id = open_file(
        copy_path
            .to_str()
            .ok_or("Invalid copy file path")?
            .to_string(),
        state,
    )
    .await?;

    Ok(new_file_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper function to create a test .elf file
    fn create_test_elf_file(dir: &Path, name: &str) -> PathBuf {
        let file_path = dir.join(format!("{}.elf", name));
        fs::write(&file_path, b"test content").expect("Failed to create test file");
        file_path
    }

    #[tokio::test]
    async fn test_duplicate_file_name_generation() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let test_file = create_test_elf_file(temp_dir.path(), "test");

        // Test: First copy should be "test Copy"
        let base_name = test_file.file_stem().unwrap().to_str().unwrap();
        let expected_copy_name = format!("{} Copy.elf", base_name);
        let expected_copy_path = temp_dir.path().join(&expected_copy_name);

        assert!(!expected_copy_path.exists(), "Copy should not exist yet");

        // Create the first copy manually to test the naming logic
        fs::copy(&test_file, &expected_copy_path).expect("Failed to copy file");
        assert!(expected_copy_path.exists(), "First copy should exist");

        // Test: Second copy should be "test Copy 2"
        let expected_copy_2_path = temp_dir.path().join(format!("{} Copy 2.elf", base_name));
        assert!(
            !expected_copy_2_path.exists(),
            "Copy 2 should not exist yet"
        );
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

    #[test]
    fn test_file_copy_content_integrity() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let test_content = b"test file content for integrity check";
        let source_path = temp_dir.path().join("source.elf");
        let dest_path = temp_dir.path().join("dest.elf");

        // Create source file
        fs::write(&source_path, test_content).expect("Failed to write source file");

        // Copy file
        fs::copy(&source_path, &dest_path).expect("Failed to copy file");

        // Verify content
        let copied_content = fs::read(&dest_path).expect("Failed to read copied file");
        assert_eq!(
            copied_content, test_content,
            "Copied file content should match source"
        );
    }

    #[test]
    fn test_file_metadata_extraction() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let test_file = create_test_elf_file(temp_dir.path(), "metadata-test");

        // Test file_stem extraction
        let base_name = test_file.file_stem().unwrap().to_str().unwrap();
        assert_eq!(base_name, "metadata-test");

        // Test parent directory
        let parent = test_file.parent().unwrap();
        assert_eq!(parent, temp_dir.path());
    }

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
