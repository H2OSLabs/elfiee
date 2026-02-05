//! Session Block writer
//!
//! Writes parsed session documents into Elfiee Markdown Blocks as JSON content
//! using the engine's command pipeline (event sourcing).
//!
//! Each JSONL session file maps to one Markdown Block. On first sync, the block
//! is created via `core.create` then written with `markdown.write`. On subsequent
//! syncs, new messages are appended to the JSON structure and metadata updated.

use crate::engine::EngineHandle;
use crate::models::Command;
use crate::sync::parser::{create_session_data, SessionData, SessionDocument, SessionMetadata};
use crate::utils::time::now_utc;
use std::collections::HashMap;

/// Session Block writer — maps session_id → block_id and writes JSON content.
pub struct SessionWriter {
    /// session_id → block_id cache
    session_blocks: HashMap<String, String>,
}

impl SessionWriter {
    pub fn new() -> Self {
        Self {
            session_blocks: HashMap::new(),
        }
    }

    /// Write a SessionDocument to a Markdown Block as JSON content.
    ///
    /// Creates a new block if this is the first sync for the session, otherwise
    /// appends new messages to the existing JSON structure.
    pub async fn write_session(
        &mut self,
        handle: &EngineHandle,
        editor_id: &str,
        elf_block_id: &str,
        project_name: &str,
        config_dir: &str,
        doc: SessionDocument,
        source_file: &str,
        source_offset: u64,
    ) -> Result<(), String> {
        let block_id = if let Some(id) = self.session_blocks.get(&doc.session_id) {
            id.clone()
        } else {
            // Before creating a new block, check if one already exists in .elf/ directory
            // This prevents duplicates if restore_offsets missed this session
            if let Some(existing_id) = self
                .find_existing_session_block(handle, elf_block_id, &doc.session_id)
                .await
            {
                log::info!(
                    "Found existing session block {} for session {} (was not in cache)",
                    existing_id,
                    doc.session_id
                );
                self.session_blocks
                    .insert(doc.session_id.clone(), existing_id.clone());
                existing_id
            } else {
                // Create new session block
                let id = self
                    .create_session_block(
                        handle,
                        editor_id,
                        elf_block_id,
                        &doc.session_id,
                        project_name,
                        config_dir,
                        source_file,
                        &doc.metadata,
                    )
                    .await?;
                self.session_blocks
                    .insert(doc.session_id.clone(), id.clone());
                id
            }
        };

        // Read existing block state (JSON content + message count)
        let (existing_content, existing_msg_count) = self.read_block_state(handle, &block_id).await;

        log::debug!(
            "Session write: session_id={}, block_id={}, existing_len={}, new_messages={}, msg_count={}",
            doc.session_id,
            block_id,
            existing_content.len(),
            doc.new_messages.len(),
            existing_msg_count
        );

        // Build or update SessionData structure
        let mut session_data = if existing_content.is_empty() {
            // First write — create new SessionData with metadata
            create_session_data(&doc.metadata, &doc.session_id, project_name)
        } else {
            // Parse existing JSON content
            SessionData::from_json(&existing_content).unwrap_or_else(|| {
                log::warn!(
                    "Failed to parse existing JSON for session {}, creating fresh",
                    doc.session_id
                );
                create_session_data(&doc.metadata, &doc.session_id, project_name)
            })
        };

        // Append new messages
        session_data.messages.extend(doc.new_messages);

        // Serialize to JSON
        let new_content = session_data.to_json();

        // Write content via markdown.write
        let write_cmd = Command::new(
            editor_id.to_string(),
            "markdown.write".to_string(),
            block_id.clone(),
            serde_json::json!({ "content": new_content }),
        );
        handle.process_command(write_cmd).await?;

        // Update metadata with sync progress (message_count is cumulative)
        let meta_cmd = Command::new(
            editor_id.to_string(),
            "core.update_metadata".to_string(),
            block_id,
            serde_json::json!({
                "metadata": {
                    "sync_offset": source_offset,
                    "message_count": existing_msg_count + doc.new_message_count as u64,
                    "last_synced_at": now_utc()
                }
            }),
        );
        handle.process_command(meta_cmd).await?;

        Ok(())
    }

    /// Create a new Markdown Block for a session.
    async fn create_session_block(
        &self,
        handle: &EngineHandle,
        editor_id: &str,
        elf_block_id: &str,
        session_id: &str,
        project_name: &str,
        config_dir: &str,
        source_file: &str,
        metadata: &SessionMetadata,
    ) -> Result<String, String> {
        let description = format!(
            "Claude Code session for {} ({})",
            project_name,
            metadata.git_branch.as_deref().unwrap_or("unknown")
        );

        let create_cmd = Command::new(
            editor_id.to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": format!("session_{}", session_id),
                "block_type": "markdown",
                "source": "outline",
                "metadata": {
                    "description": description,
                    "session_id": session_id,
                    "project_name": project_name,
                    "config_dir": config_dir,
                    "source_file": source_file,
                    "sync_offset": 0,
                    "model": metadata.model,
                    "git_branch": metadata.git_branch,
                    "message_count": 0,
                    "last_synced_at": now_utc()
                }
            }),
        );

        let events = handle.process_command(create_cmd).await?;
        let block_id = events
            .first()
            .ok_or("No event returned from core.create")?
            .entity
            .clone();

        // Add entry to .elf/ session directory
        self.add_session_entry(
            handle,
            editor_id,
            elf_block_id,
            project_name,
            session_id,
            &block_id,
        )
        .await?;

        Ok(block_id)
    }

    /// Add a session entry to the .elf/ directory's entries map.
    async fn add_session_entry(
        &self,
        handle: &EngineHandle,
        editor_id: &str,
        elf_block_id: &str,
        project_name: &str,
        session_id: &str,
        block_id: &str,
    ) -> Result<(), String> {
        // Get current .elf/ directory entries
        let elf_block = handle
            .get_block(elf_block_id.to_string())
            .await
            .ok_or("Cannot find .elf/ directory block")?;

        let mut entries = elf_block
            .contents
            .get("entries")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));

        let entries_obj = entries.as_object_mut().ok_or("entries is not an object")?;

        // Ensure session/{project}/ directory entry exists
        let session_dir_key = format!("session/{}/", project_name);
        if !entries_obj.contains_key(&session_dir_key) {
            entries_obj.insert(
                session_dir_key,
                serde_json::json!({
                    "id": format!("dir-session-{}", project_name),
                    "type": "directory"
                }),
            );
        }

        // Add session file entry
        let file_key = format!("session/{}/session_{}.json", project_name, session_id);
        entries_obj.insert(
            file_key,
            serde_json::json!({
                "id": block_id,
                "type": "file",
                "source": "outline",
                "updated_at": now_utc()
            }),
        );

        // Write updated entries
        let write_cmd = Command::new(
            editor_id.to_string(),
            "directory.write".to_string(),
            elf_block_id.to_string(),
            serde_json::json!({
                "entries": entries
            }),
        );
        handle.process_command(write_cmd).await?;

        Ok(())
    }

    /// Find an existing session block by session_id in the .elf/ directory entries.
    ///
    /// This is a defensive check to prevent creating duplicate blocks if the
    /// session_blocks cache was not populated correctly by restore_offsets.
    async fn find_existing_session_block(
        &self,
        handle: &EngineHandle,
        elf_block_id: &str,
        session_id: &str,
    ) -> Option<String> {
        let elf_block = handle.get_block(elf_block_id.to_string()).await?;
        let entries = elf_block.contents.get("entries")?.as_object()?;

        for (path, entry) in entries {
            // Match pattern: session/*/session_{session_id}.json (or .md)
            if !path.starts_with("session/") {
                continue;
            }
            let is_json = path.ends_with(".json");
            let is_md = path.ends_with(".md");
            if !is_json && !is_md {
                continue;
            }

            // Check if this entry matches our session_id
            let expected_suffix_json = format!("/session_{}.json", session_id);
            let expected_suffix_md = format!("/session_{}.md", session_id);
            if path.ends_with(&expected_suffix_json) || path.ends_with(&expected_suffix_md) {
                return entry.get("id").and_then(|v| v.as_str()).map(String::from);
            }
        }

        None
    }

    /// Read current content and message count from a Markdown Block.
    ///
    /// Note: markdown.write stores content under `contents["markdown"]`, not `contents["content"]`.
    async fn read_block_state(&self, handle: &EngineHandle, block_id: &str) -> (String, u64) {
        if let Some(block) = handle.get_block(block_id.to_string()).await {
            // markdown.write stores content under "markdown" key
            let content = block
                .contents
                .get("markdown")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let msg_count = block
                .metadata
                .custom
                .get("message_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            (content, msg_count)
        } else {
            (String::new(), 0)
        }
    }

    /// Load existing session block mappings from the .elf/ directory.
    ///
    /// Scans .elf/ entries for `session/*/session_*.md` patterns and populates
    /// the session_id → block_id cache. Also restores parser offsets from
    /// block metadata.
    pub async fn load_existing_sessions(
        &mut self,
        handle: &EngineHandle,
        elf_block_id: &str,
    ) -> Result<HashMap<String, u64>, String> {
        let mut offset_map: HashMap<String, u64> = HashMap::new();

        let elf_block = match handle.get_block(elf_block_id.to_string()).await {
            Some(b) => b,
            None => return Ok(offset_map),
        };

        let entries = match elf_block
            .contents
            .get("entries")
            .and_then(|e| e.as_object())
        {
            Some(e) => e,
            None => return Ok(offset_map),
        };

        for (path, entry) in entries {
            // Match pattern: session/{project}/session_{uuid}.json (or .md for backwards compat)
            if !path.starts_with("session/") {
                continue;
            }
            let is_json = path.ends_with(".json");
            let is_md = path.ends_with(".md");
            if !is_json && !is_md {
                continue;
            }
            let filename = path.rsplit('/').next().unwrap_or("");
            if !filename.starts_with("session_") {
                continue;
            }

            // Extract session_id: "session_abc-123.json" → "abc-123"
            let session_id = filename
                .strip_prefix("session_")
                .and_then(|s| s.strip_suffix(".json").or_else(|| s.strip_suffix(".md")))
                .unwrap_or("")
                .to_string();

            if session_id.is_empty() {
                continue;
            }

            if let Some(block_id) = entry.get("id").and_then(|v| v.as_str()) {
                self.session_blocks
                    .insert(session_id.clone(), block_id.to_string());

                // Try to restore sync_offset from block metadata.custom
                if let Some(block) = handle.get_block(block_id.to_string()).await {
                    if let Some(offset) = block
                        .metadata
                        .custom
                        .get("sync_offset")
                        .and_then(|v| v.as_u64())
                    {
                        if let Some(source_file) = block
                            .metadata
                            .custom
                            .get("source_file")
                            .and_then(|v| v.as_str())
                        {
                            offset_map.insert(source_file.to_string(), offset);
                        }
                    }
                }
            }
        }

        Ok(offset_map)
    }
}
