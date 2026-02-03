/// Capability: create
///
/// Creates a new file or directory inside the Directory Block.
use super::DirectoryCreatePayload;
use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, Event};
use crate::utils::time::now_utc;
use crate::utils::validate_virtual_path;
use capability_macros::capability;
use serde_json::json;

/// Handler for directory.create capability.
///
/// Creates a new file or directory in the virtual file system.
/// For files, creates a new Content Block and adds entry to Directory.
/// For directories, only adds a virtual entry (no Block created).
///
/// # Arguments
/// * `cmd` - The command containing the payload
/// * `block` - The Directory Block to operate on
///
/// # Returns
/// * `Ok(Vec<Event>)` - Events to be committed (core.create for files + directory.write)
/// * `Err(String)` - Error description
///
#[capability(id = "directory.create", target = "directory")]
fn handle_create(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    // Step 1: Validate block exists and is directory type
    let block = block.ok_or("Block required for directory.create")?;

    if block.block_type != "directory" {
        return Err(format!(
            "Expected block_type 'directory', got '{}'",
            block.block_type
        ));
    }

    // Step 2: Deserialize payload
    let payload: DirectoryCreatePayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload for directory.create: {}", e))?;

    // Step 3: Validate path format and security
    validate_virtual_path(&payload.path)?;

    // Step 4: Parse current Directory Block contents
    let mut contents: serde_json::Map<String, serde_json::Value> =
        block.contents.as_object().cloned().unwrap_or_default();

    // Initialize entries if not exists
    if !contents.contains_key("entries") {
        contents.insert("entries".to_string(), json!({}));
    }

    let entries = contents
        .get_mut("entries")
        .and_then(|v| v.as_object_mut())
        .ok_or("Invalid directory structure: entries must be an object")?;

    // Step 5: Check if path already exists
    if entries.contains_key(&payload.path) {
        return Err(format!("Path already exists: {}", payload.path));
    }

    let mut events = Vec::new();

    // Step 6: Handle creation based on entry_type
    if payload.entry_type == "file" {
        let file_block_id = if let Some(existing_id) = payload.existing_block_id {
            // Use existing block — skip core.create, just register in directory index
            existing_id
        } else {
            // Create file: generate new Content Block + add entry
            let new_id = uuid::Uuid::new_v4().to_string();
            let file_name = payload
                .path
                .split('/')
                .last()
                .unwrap_or(&payload.path)
                .to_string();
            let block_type = payload.block_type.unwrap_or_else(|| "markdown".to_string());

            // Unified field logic
            let contents = if block_type == "markdown" {
                json!({
                    "markdown": payload.content.unwrap_or_default(),
                    "source": payload.source
                })
            } else {
                json!({
                    "text": payload.content.unwrap_or_default(),
                    "source": payload.source
                })
            };

            // Event 1: Create Content Block (core.create)
            // NOTE: count=1 is a placeholder. Engine actor will update it with correct vector clock.
            events.push(create_event(
                new_id.clone(),
                "core.create",
                json!({
                    "name": file_name,
                    "type": block_type,
                    "owner": cmd.editor_id,
                    "contents": contents,
                    "children": {},
                    "metadata": {}
                }),
                &cmd.editor_id,
                1,
            ));

            new_id
        };

        // Add entry to Directory
        entries.insert(
            payload.path.clone(),
            json!({
                "id": file_block_id,
                "type": "file",
                "source": payload.source,
                "updated_at": now_utc()
            }),
        );
    } else if payload.entry_type == "directory" {
        // Create directory: only add virtual entry (no Block)
        let dir_id = format!("dir-{}", uuid::Uuid::new_v4());

        entries.insert(
            payload.path.clone(),
            json!({
                "id": dir_id,
                "type": "directory",
                "source": payload.source,
                "updated_at": now_utc()
            }),
        );
    } else {
        return Err(format!(
            "Invalid entry_type: '{}'. Must be 'file' or 'directory'",
            payload.entry_type
        ));
    }

    // Step 7: Update Directory Block contents
    events.push(create_event(
        block.block_id.clone(),
        "directory.write",
        json!({ "contents": { "entries": entries } }),
        &cmd.editor_id,
        1,
    ));

    Ok(events)
}
