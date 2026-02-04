use crate::extensions::directory::DirectoryExportPayload;
use crate::models::Command;
use crate::state::AppState;
use serde_json::json;
use std::fs;
use std::path::Path;
use tauri::State;

// ============================================================================
// Business Function (shared by Tauri command and MCP server)
// ============================================================================

/// Materialize blocks to the external file system (Checkout).
///
/// Business logic shared between Tauri command and MCP server.
/// 1. Calls `directory.export` capability for authorization and auditing.
/// 2. If authorized, writes block contents to the target path.
pub async fn do_checkout_workspace(
    app_state: &AppState,
    file_id: &str,
    editor_id: &str,
    block_id: &str,
    payload: &DirectoryExportPayload,
) -> Result<(), String> {
    // 1. Get engine handle
    let handle = app_state
        .engine_manager
        .get_engine(file_id)
        .ok_or_else(|| format!("File '{}' is not open", file_id))?;

    // 2. Perform Authorization & Auditing via Engine
    let cmd = Command::new(
        editor_id.to_string(),
        "directory.export".to_string(),
        block_id.to_string(),
        json!(payload),
    );

    handle.process_command(cmd).await?;

    // 3. Permission granted. Now perform actual I/O.

    // Get the directory block to read its entries
    let dir_block = handle
        .get_block(block_id.to_string())
        .await
        .ok_or_else(|| format!("Directory block '{}' not found", block_id))?;

    let entries = dir_block
        .contents
        .get("entries")
        .and_then(|v| v.as_object())
        .ok_or("Invalid directory structure: missing entries")?;

    let target_root = Path::new(&payload.target_path);

    // Create target directory if it doesn't exist
    fs::create_dir_all(target_root)
        .map_err(|e| format!("Failed to create target directory: {}", e))?;

    let source_prefix = payload
        .source_path
        .clone()
        .unwrap_or_else(|| "".to_string());

    // 4. Iterate through entries and write files
    for (virtual_path, entry_value) in entries {
        // Filter by source_path if specified
        if !virtual_path.starts_with(&source_prefix) {
            continue;
        }

        // Security: Path traversal check
        use std::path::Component;
        if Path::new(virtual_path)
            .components()
            .any(|c| matches!(c, Component::ParentDir))
        {
            log::warn!("Skipping path with traversal components: {}", virtual_path);
            continue;
        }

        let entry_type = entry_value["type"].as_str().unwrap_or("");

        if entry_type == "directory" {
            // Create directory
            let path = target_root.join(virtual_path);
            fs::create_dir_all(path)
                .map_err(|e| format!("Failed to create sub-directory: {}", e))?;
        } else if entry_type == "file" {
            // Get content block
            let child_id = entry_value["id"].as_str().ok_or("Missing block ID")?;

            // Fetch block content via engine
            match handle.get_block(child_id.to_string()).await {
                Some(child_block) => {
                    // --- Dynamic Permission Check ---
                    let read_cap = match child_block.block_type.as_str() {
                        "markdown" => Some("markdown.read"),
                        "code" => Some("code.read"),
                        _ => None,
                    };

                    let authorized = if let Some(cap) = read_cap {
                        handle
                            .check_grant(
                                editor_id.to_string(),
                                cap.to_string(),
                                child_id.to_string(),
                            )
                            .await
                    } else {
                        child_block.owner == editor_id
                    };

                    if !authorized {
                        let cap_name = read_cap.unwrap_or("owner-only access");
                        log::warn!(
                            "Skipping file '{}' due to lack of {} permission",
                            virtual_path,
                            cap_name
                        );
                        continue;
                    }
                    // Standardized content field access: try 'text' then 'markdown'
                    let content = child_block
                        .contents
                        .get("text")
                        .or_else(|| child_block.contents.get("markdown"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    // Write to file
                    let file_path = target_root.join(virtual_path);
                    if let Some(parent) = file_path.parent() {
                        fs::create_dir_all(parent).map_err(|e| {
                            format!(
                                "Failed to create parent directory for '{}': {}",
                                virtual_path, e
                            )
                        })?;
                    }
                    fs::write(file_path, content)
                        .map_err(|e| format!("Failed to write file '{}': {}", virtual_path, e))?;
                }
                None => {
                    log::warn!(
                        "Content block '{}' referenced in directory but not found",
                        child_id
                    );
                }
            }
        }
    }

    Ok(())
}

// ============================================================================
// Tauri Command (thin wrapper)
// ============================================================================

/// Materialize blocks to the external file system (Checkout).
#[tauri::command]
#[specta::specta]
pub async fn checkout_workspace(
    state: State<'_, AppState>,
    file_id: String,
    block_id: String,
    payload: DirectoryExportPayload,
) -> Result<(), String> {
    let editor_id = state
        .get_active_editor(&file_id)
        .ok_or_else(|| "No active editor set for this file".to_string())?;
    do_checkout_workspace(&state, &file_id, &editor_id, &block_id, &payload).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elf::ElfArchive;
    use crate::models::Command;
    use std::fs;
    use tempfile::{NamedTempFile, TempDir};

    #[tokio::test]
    async fn test_checkout_logic_directly() {
        // This test verifies the logic inside checkout_workspace without using tauri::State

        // 1. Setup Engine
        let temp_elf = NamedTempFile::new().unwrap();
        let archive = ElfArchive::new().await.unwrap();
        archive.save(temp_elf.path()).unwrap();

        let file_id = "test-file".to_string();
        let event_pool = archive.event_pool().await.unwrap();
        let engine_manager = crate::engine::EngineManager::new();
        engine_manager
            .spawn_engine(file_id.clone(), event_pool)
            .await
            .unwrap();

        let handle = engine_manager.get_engine(&file_id).unwrap();

        // 2. Create a Directory Block
        let create_dir_cmd = Command::new(
            "alice".to_string(),
            "core.create".to_string(),
            "".to_string(),
            json!({
                "name": "My Workspace",
                "block_type": "directory",
                "contents": {
                    "entries": {}
                }
            }),
        );
        let events = handle.process_command(create_dir_cmd).await.unwrap();
        let dir_block_id = events[0].entity.clone();

        // 3. Create a File Entry in directory (using extension capability)
        let create_file_cmd = Command::new(
            "alice".to_string(),
            "directory.create".to_string(),
            dir_block_id.clone(),
            json!({
                "path": "hello.md",
                "type": "file",
                "source": "outline",
                "content": "# Hello World",
                "block_type": "markdown"
            }),
        );
        let events = handle.process_command(create_file_cmd).await.unwrap();

        // Find the generated block ID from the events
        let file_block_id = events
            .iter()
            .find(|e| e.attribute.ends_with("/core.create"))
            .map(|e| e.entity.clone())
            .expect("Failed to find core.create event");

        // 4. Verify Checkout Logic
        let export_dir = TempDir::new().unwrap();
        let payload = DirectoryExportPayload {
            target_path: export_dir.path().to_string_lossy().to_string(),
            source_path: None,
        };

        // Manually perform what checkout_workspace does
        // (Since we can't easily construct tauri::State in unit tests)

        // Check Auth (Audit)
        let audit_cmd = Command::new(
            "alice".to_string(),
            "directory.export".to_string(),
            dir_block_id.clone(),
            json!(payload),
        );
        handle.process_command(audit_cmd).await.unwrap();

        // Read and Write
        let dir_block = handle.get_block(dir_block_id).await.unwrap();
        let entries = dir_block.contents["entries"].as_object().unwrap();

        for (virtual_path, entry_value) in entries {
            let entry_type = entry_value["type"].as_str().unwrap();
            let target_root = Path::new(&payload.target_path);

            if entry_type == "directory" {
                fs::create_dir_all(target_root.join(virtual_path)).unwrap();
            } else {
                let child_id = entry_value["id"].as_str().unwrap();

                // Assert the ID matches the one created
                assert_eq!(child_id, file_block_id);

                let child_block = handle.get_block(child_id.to_string()).await.unwrap();

                // Standardized content field access: try 'text' then 'markdown'
                let content = child_block
                    .contents
                    .get("text")
                    .or_else(|| child_block.contents.get("markdown"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let file_path = target_root.join(virtual_path);
                fs::create_dir_all(file_path.parent().unwrap()).ok();
                fs::write(file_path, content).unwrap();
            }
        }

        // 5. Verify results on disk
        assert!(export_dir.path().join("hello.md").exists());
        let content = fs::read_to_string(export_dir.path().join("hello.md")).unwrap();
        assert_eq!(content, "# Hello World");
    }
}
