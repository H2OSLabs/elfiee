//! MCP Server Implementation
//!
//! Uses rmcp's macro system for clean tool definitions.
//! All tools call EngineManager directly, no intermediate layers.

use crate::extensions::agent::AgentContents;
use crate::mcp;
use crate::models::Command;
use crate::state::AppState;
use rmcp::{
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::{
        Annotated, CallToolResult, Content, Implementation, ListResourceTemplatesResult,
        ListResourcesResult, PaginatedRequestParam, RawResource, RawResourceTemplate,
        ReadResourceRequestParam, ReadResourceResult, ResourceContents, ResourcesCapability,
        ServerCapabilities, ServerInfo, ToolsCapability,
    },
    service::{RequestContext, RoleServer},
    tool, tool_handler, tool_router, ErrorData as McpError,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use std::future::Future;
use std::sync::Arc;

/// Elfiee MCP Server
///
/// Provides MCP protocol access to Elfiee's capabilities.
/// Runs as an independent SSE server, sharing AppState with the GUI.
///
/// When `agent_block_id` is set, this server is bound to a specific agent
/// and `resolve_agent_editor_id` returns that agent's editor deterministically.
/// When `agent_block_id` is None, this is the management port (47200) and
/// falls back to the GUI active editor.
#[derive(Clone)]
pub struct ElfieeMcpServer {
    app_state: Arc<AppState>,
    tool_router: ToolRouter<Self>,
    /// Bound agent block ID (per-agent mode) or None (management port)
    agent_block_id: Option<String>,
}

// ============================================================================
// Tool Input Structures
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProjectInput {
    /// Path to the .elf project file
    pub project: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BlockInput {
    /// Path to the .elf project file
    pub project: String,
    /// ID of the block
    pub block_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BlockCreateInput {
    /// Path to the .elf project file
    pub project: String,
    /// Name of the new block
    pub name: String,
    /// Block type: markdown, code, directory, terminal
    pub block_type: String,
    /// Optional parent block ID to link to
    pub parent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BlockRenameInput {
    /// Path to the .elf project file
    pub project: String,
    /// ID of the block to rename
    pub block_id: String,
    /// New name for the block
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BlockLinkInput {
    /// Path to the .elf project file
    pub project: String,
    /// Parent block ID
    pub parent_id: String,
    /// Child block ID
    pub child_id: String,
    /// Relation type (e.g., 'contains', 'references')
    pub relation: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BlockUnlinkInput {
    /// Path to the .elf project file
    pub project: String,
    /// Parent block ID
    pub parent_id: String,
    /// Child block ID
    pub child_id: String,
    /// Relation type to remove
    pub relation: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BlockChangeTypeInput {
    /// Path to the .elf project file
    pub project: String,
    /// ID of the block
    pub block_id: String,
    /// New block type
    pub new_type: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BlockUpdateMetadataInput {
    /// Path to the .elf project file
    pub project: String,
    /// ID of the block
    pub block_id: String,
    /// Metadata object to merge
    pub metadata: serde_json::Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ContentWriteInput {
    /// Path to the .elf project file
    pub project: String,
    /// ID of the block
    pub block_id: String,
    /// Content to write
    pub content: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DirectoryCreateInput {
    /// Path to the .elf project file
    pub project: String,
    /// Directory block ID
    pub block_id: String,
    /// Virtual path (e.g., 'src/main.rs')
    pub path: String,
    /// Entry type: 'file' or 'directory'
    #[serde(rename = "type")]
    pub entry_type: String,
    /// Source category: 'outline' or 'linked'
    pub source: String,
    /// Initial content for files
    pub content: Option<String>,
    /// Block type for files: markdown, code
    pub block_type: Option<String>,
    /// Optional: use an existing block ID instead of creating a new block
    pub existing_block_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DirectoryDeleteInput {
    /// Path to the .elf project file
    pub project: String,
    /// Directory block ID
    pub block_id: String,
    /// Virtual path to delete
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DirectoryRenameInput {
    /// Path to the .elf project file
    pub project: String,
    /// Directory block ID
    pub block_id: String,
    /// Old path
    pub old_path: String,
    /// New path
    pub new_path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DirectoryWriteInput {
    /// Path to the .elf project file
    pub project: String,
    /// Directory block ID
    pub block_id: String,
    /// Directory entries to write
    pub entries: serde_json::Value,
    /// Optional source category
    pub source: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DirectoryImportInput {
    /// Path to the .elf project file
    pub project: String,
    /// Directory block ID
    pub block_id: String,
    /// Source path on filesystem to import from
    pub source_path: String,
    /// Optional target path within directory
    pub target_path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DirectoryExportInput {
    /// Path to the .elf project file
    pub project: String,
    /// Directory block ID
    pub block_id: String,
    /// Target path on filesystem to export to
    pub target_path: String,
    /// Optional source path within directory
    pub source_path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TerminalInitInput {
    /// Path to the .elf project file
    pub project: String,
    /// Terminal block ID
    pub block_id: String,
    /// Optional shell to use (e.g., 'bash', 'powershell')
    pub shell: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TerminalExecuteInput {
    /// Path to the .elf project file
    pub project: String,
    /// Terminal block ID
    pub block_id: String,
    /// Command to execute
    pub command: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TerminalSaveInput {
    /// Path to the .elf project file
    pub project: String,
    /// Terminal block ID
    pub block_id: String,
    /// Terminal session content to save
    pub content: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GrantInput {
    /// Path to the .elf project file
    pub project: String,
    /// Block ID to grant permission on
    pub block_id: String,
    /// Editor ID to grant permission to
    pub editor_id: String,
    /// Capability ID to grant (e.g., 'markdown.write')
    pub cap_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EditorInput {
    /// Path to the .elf project file
    pub project: String,
    /// Editor ID
    pub editor_id: String,
    /// Optional display name for the editor
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskCreateInput {
    /// Path to the .elf project file
    pub project: String,
    /// Name for the new task
    pub name: String,
    /// Optional description for the task
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskWriteInput {
    /// Path to the .elf project file
    pub project: String,
    /// Task block ID
    pub block_id: String,
    /// Markdown content to write
    pub content: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskCommitInput {
    /// Path to the .elf project file
    pub project: String,
    /// Task block ID to commit
    pub block_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskLinkInput {
    /// Path to the .elf project file
    pub project: String,
    /// Task block ID
    pub task_id: String,
    /// Block ID to link as implementation
    pub block_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExecInput {
    /// Path to the .elf project file
    pub project: String,
    /// Capability ID (e.g., 'markdown.write')
    pub capability: String,
    /// Target block ID
    pub block_id: Option<String>,
    /// Capability-specific payload
    pub payload: Option<serde_json::Value>,
}

// ============================================================================
// MCP Server Implementation
// ============================================================================

#[tool_router]
impl ElfieeMcpServer {
    /// Create a new MCP server instance.
    ///
    /// - `agent_block_id = None`: management port mode (fallback to GUI active editor)
    /// - `agent_block_id = Some(id)`: per-agent mode (deterministic editor identity)
    pub fn new(app_state: Arc<AppState>, agent_block_id: Option<String>) -> Self {
        Self {
            app_state,
            agent_block_id,
            tool_router: Self::tool_router(),
        }
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    /// Get file_id from project path
    fn get_file_id(&self, project: &str) -> Result<String, McpError> {
        let files = self.app_state.list_open_files();
        for (file_id, path) in &files {
            if path == project {
                return Ok(file_id.clone());
            }
        }
        Err(mcp::project_not_open(project))
    }

    /// Get active editor for a file
    fn get_editor_id(&self, file_id: &str) -> Result<String, McpError> {
        self.app_state
            .get_active_editor(file_id)
            .ok_or_else(|| mcp::no_active_editor(file_id))
    }

    /// Get engine handle for a file
    fn get_engine(&self, file_id: &str) -> Result<crate::engine::EngineHandle, McpError> {
        self.app_state
            .engine_manager
            .get_engine(file_id)
            .ok_or_else(|| mcp::engine_not_found(file_id))
    }

    /// Resolve the editor ID for MCP operations.
    ///
    /// In per-agent mode (`agent_block_id` is set), deterministically returns
    /// that agent's `editor_id` from its AgentContents.
    ///
    /// In management port mode (`agent_block_id` is None), falls back to
    /// the GUI active editor.
    async fn resolve_agent_editor_id(&self, file_id: &str) -> Result<String, McpError> {
        // Per-agent mode: deterministic lookup
        if let Some(agent_id) = &self.agent_block_id {
            let handle = self.get_engine(file_id)?;
            let block = handle
                .get_block(agent_id.clone())
                .await
                .ok_or_else(|| mcp::block_not_found(agent_id))?;
            let contents: AgentContents = serde_json::from_value(block.contents.clone())
                .map_err(|e| mcp::invalid_payload(format!("Invalid agent contents: {}", e)))?;
            return Ok(contents.editor_id);
        }

        // Management port fallback: GUI active editor
        self.get_editor_id(file_id)
    }

    /// Execute a capability and return rich result with updated state
    async fn execute_capability(
        &self,
        project: &str,
        capability: &str,
        block_id: Option<String>,
        payload: serde_json::Value,
    ) -> Result<CallToolResult, McpError> {
        let file_id = self.get_file_id(project)?;
        let editor_id = self.resolve_agent_editor_id(&file_id).await?;
        let handle = self.get_engine(&file_id)?;

        let target_block_id = block_id.clone().unwrap_or_default();
        let cmd = Command::new(
            editor_id.clone(),
            capability.to_string(),
            target_block_id.clone(),
            payload,
        );

        match handle.process_command(cmd).await {
            Ok(events) => {
                // Notify frontend of state change
                let _ = self.app_state.state_changed_tx.send(file_id.clone());

                let mut result = json!({
                    "ok": true,
                    "capability": capability,
                    "editor": editor_id,
                    "events_committed": events.len(),
                });

                // For create operations, extract the new block_id from events
                if capability == "core.create" {
                    if let Some(ev) = events.first() {
                        result["created_block_id"] = json!(ev.entity);
                        // Fetch the newly created block for full details
                        if let Some(block) = handle.get_block(ev.entity.clone()).await {
                            result["block"] = Self::format_block_summary(&block);
                        }
                    }
                } else if !target_block_id.is_empty() {
                    // For other operations, fetch the updated block state
                    if let Some(block) = handle.get_block(target_block_id.clone()).await {
                        result["block"] = Self::format_block_summary(&block);
                    }
                }

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap(),
                )]))
            }
            Err(e) => {
                let error_msg = format!("{}", e);
                let hint = Self::error_hint(capability, &error_msg);
                let result = json!({
                    "ok": false,
                    "capability": capability,
                    "error": error_msg,
                    "hint": hint,
                });
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap(),
                )]))
            }
        }
    }

    /// Format a block into a concise, informative summary
    fn format_block_summary(block: &crate::models::Block) -> serde_json::Value {
        let mut summary = json!({
            "block_id": block.block_id,
            "name": block.name,
            "block_type": block.block_type,
            "owner": block.owner,
        });

        // Content preview (type-specific)
        match block.block_type.as_str() {
            "markdown" => {
                if let Some(md) = block.contents.get("markdown").and_then(|v| v.as_str()) {
                    let preview = if md.len() > 200 {
                        format!("{}...", &md[..200])
                    } else {
                        md.to_string()
                    };
                    summary["content_preview"] = json!(preview);
                    summary["content_length"] = json!(md.len());
                }
            }
            "code" => {
                if let Some(code) = block.contents.get("text").and_then(|v| v.as_str()) {
                    let preview = if code.len() > 200 {
                        format!("{}...", &code[..200])
                    } else {
                        code.to_string()
                    };
                    summary["content_preview"] = json!(preview);
                    summary["content_length"] = json!(code.len());
                }
                if let Some(lang) = block.contents.get("language").and_then(|v| v.as_str()) {
                    summary["language"] = json!(lang);
                }
            }
            "directory" => {
                if let Some(index) = block.contents.get("index") {
                    if let Some(obj) = index.as_object() {
                        summary["entry_count"] = json!(obj.len());
                    }
                }
            }
            _ => {}
        }

        // Children relations
        if !block.children.is_empty() {
            let relations: serde_json::Value = block
                .children
                .iter()
                .map(|(rel, ids)| (rel.clone(), json!(ids)))
                .collect::<serde_json::Map<String, serde_json::Value>>()
                .into();
            summary["children"] = relations;
        }

        // Metadata
        let mut meta = json!({});
        if let Some(desc) = &block.metadata.description {
            meta["description"] = json!(desc);
        }
        if let Some(created) = &block.metadata.created_at {
            meta["created_at"] = json!(created);
        }
        if let Some(updated) = &block.metadata.updated_at {
            meta["updated_at"] = json!(updated);
        }
        if !block.metadata.custom.is_empty() {
            meta["custom"] = json!(block.metadata.custom);
        }
        summary["metadata"] = meta;

        summary
    }

    /// Capture terminal output with quiescence detection.
    ///
    /// Polls the output buffer until:
    /// - Output has been received and no new data arrives for 1 second, OR
    /// - No output at all after 5 seconds, OR
    /// - Maximum timeout of 30 seconds reached
    async fn capture_terminal_output(
        buffers: &dashmap::DashMap<String, std::sync::Arc<std::sync::Mutex<Vec<u8>>>>,
        block_id: &str,
    ) -> String {
        use std::time::{Duration, Instant};

        let start = Instant::now();
        let quiescence = Duration::from_secs(1);
        let no_output_timeout = Duration::from_secs(5);
        let max_timeout = Duration::from_secs(30);

        let mut last_len = 0usize;
        let mut last_change = Instant::now();
        let mut ever_had_output = false;

        // Initial delay for shell to start processing
        tokio::time::sleep(Duration::from_millis(100)).await;

        loop {
            tokio::time::sleep(Duration::from_millis(50)).await;

            let current_len = buffers
                .get(block_id)
                .map(|buf| buf.lock().unwrap().len())
                .unwrap_or(0);

            if current_len > last_len {
                last_len = current_len;
                last_change = Instant::now();
                ever_had_output = true;
            }

            // Quiescence: output received but stopped arriving
            if ever_had_output && last_change.elapsed() >= quiescence {
                break;
            }

            // No output at all for too long
            if !ever_had_output && start.elapsed() >= no_output_timeout {
                break;
            }

            // Hard timeout
            if start.elapsed() >= max_timeout {
                break;
            }
        }

        // Read the buffer content
        buffers
            .get(block_id)
            .map(|buf| {
                let data = buf.lock().unwrap();
                String::from_utf8_lossy(&data).to_string()
            })
            .unwrap_or_default()
    }

    /// Provide actionable hints for common errors
    fn error_hint(capability: &str, error: &str) -> String {
        let lower = error.to_lowercase();
        if lower.contains("not authorized") || lower.contains("permission") {
            return format!(
                "The current editor lacks '{}' permission. Use elfiee_grant to grant it first.",
                capability
            );
        }
        if lower.contains("not found") {
            return "The target block does not exist. Use elfiee_block_list to see available blocks."
                .to_string();
        }
        if lower.contains("type") && lower.contains("mismatch") {
            return format!(
                "This block's type doesn't support '{}'. Check the block_type with elfiee_block_get.",
                capability
            );
        }
        if lower.contains("payload") || lower.contains("invalid") {
            return "The request payload is malformed. Check the required fields for this tool."
                .to_string();
        }
        "Check the error message above. Use elfiee_block_get to inspect the block's current state."
            .to_string()
    }

    // ========================================================================
    // File Operations
    // ========================================================================

    /// List all currently open .elf files in the Elfiee GUI
    #[tool(
        description = "List all currently open .elf project files. Returns file paths, active editors, and block counts. Use the 'project' path from results as input for other tools."
    )]
    async fn elfiee_file_list(&self) -> Result<CallToolResult, McpError> {
        let files = self.app_state.list_open_files();

        if files.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&json!({
                    "files": [],
                    "count": 0,
                    "hint": "No .elf files are currently open. Open a file in the Elfiee GUI first, then retry."
                }))
                .unwrap(),
            )]));
        }

        let mut result = Vec::new();
        for (file_id, path) in &files {
            let mut file_info = json!({
                "project": path,
                "file_id": file_id,
            });

            // Add active editor info
            if let Some(editor_id) = self.app_state.get_active_editor(file_id) {
                file_info["active_editor"] = json!(editor_id);
            } else {
                file_info["active_editor"] = json!(null);
                file_info["warning"] = json!("No active editor set. Some operations may fail.");
            }

            // Add block count
            if let Some(handle) = self.app_state.engine_manager.get_engine(file_id) {
                let blocks = handle.get_all_blocks().await;
                file_info["block_count"] = json!(blocks.len());

                // Summarize block types
                let mut type_counts = std::collections::HashMap::new();
                for block in blocks.values() {
                    *type_counts
                        .entry(block.block_type.clone())
                        .or_insert(0usize) += 1;
                }
                if !type_counts.is_empty() {
                    file_info["block_types"] = json!(type_counts);
                }
            }

            result.push(file_info);
        }

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&json!({
                "files": result,
                "count": files.len(),
                "hint": "Use the 'project' value as the 'project' parameter for other elfiee tools."
            }))
            .unwrap(),
        )]))
    }

    // ========================================================================
    // Block Operations
    // ========================================================================

    /// List all blocks in a project with summaries
    #[tool(
        description = "List all blocks in a project with type, content preview, relations, and metadata. Use elfiee_block_get for full block details."
    )]
    async fn elfiee_block_list(
        &self,
        Parameters(input): Parameters<ProjectInput>,
    ) -> Result<CallToolResult, McpError> {
        let file_id = self.get_file_id(&input.project)?;
        let handle = self.get_engine(&file_id)?;

        let blocks = handle.get_all_blocks().await;
        let result: Vec<serde_json::Value> = blocks
            .values()
            .map(|block| Self::format_block_summary(block))
            .collect();

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&json!({
                "project": input.project,
                "blocks": result,
                "count": blocks.len(),
            }))
            .unwrap(),
        )]))
    }

    /// Get detailed information about a specific block
    #[tool(
        description = "Get full details of a block including all contents, children relations, metadata, and permissions."
    )]
    async fn elfiee_block_get(
        &self,
        Parameters(input): Parameters<BlockInput>,
    ) -> Result<CallToolResult, McpError> {
        let file_id = self.get_file_id(&input.project)?;
        let handle = self.get_engine(&file_id)?;

        match handle.get_block(input.block_id.clone()).await {
            Some(block) => {
                let mut result = json!({
                    "block_id": block.block_id,
                    "name": block.name,
                    "block_type": block.block_type,
                    "owner": block.owner,
                    "contents": block.contents,
                });

                // Children relations
                if block.children.is_empty() {
                    result["children"] = json!({});
                } else {
                    let relations: serde_json::Value = block
                        .children
                        .iter()
                        .map(|(rel, ids)| (rel.clone(), json!(ids)))
                        .collect::<serde_json::Map<String, serde_json::Value>>()
                        .into();
                    result["children"] = relations;
                }

                // Full metadata
                result["metadata"] = json!({
                    "description": block.metadata.description,
                    "created_at": block.metadata.created_at,
                    "updated_at": block.metadata.updated_at,
                    "custom": block.metadata.custom,
                });

                // Grants on this block
                let grants = handle.get_block_grants(block.block_id.clone()).await;
                if !grants.is_empty() {
                    let grant_list: Vec<serde_json::Value> = grants
                        .iter()
                        .map(|(editor_id, cap_id, _)| {
                            json!({ "editor": editor_id, "capability": cap_id })
                        })
                        .collect();
                    result["grants"] = json!(grant_list);
                }

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap(),
                )]))
            }
            None => {
                let blocks = handle.get_all_blocks().await;
                let available: Vec<String> = blocks
                    .values()
                    .map(|b| format!("{} ({})", b.block_id, b.name))
                    .collect();
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&json!({
                        "error": format!("Block '{}' not found", input.block_id),
                        "available_blocks": available,
                    }))
                    .unwrap(),
                )]))
            }
        }
    }

    /// Create a new block in the project
    #[tool(
        description = "Create a new block (markdown, code, directory, or terminal) in the project. Returns the created block with its generated block_id."
    )]
    async fn elfiee_block_create(
        &self,
        Parameters(input): Parameters<BlockCreateInput>,
    ) -> Result<CallToolResult, McpError> {
        let payload = json!({
            "name": input.name,
            "block_type": input.block_type
        });
        self.execute_capability(&input.project, "core.create", input.parent_id, payload)
            .await
    }

    /// Delete a block from the project
    #[tool(
        description = "Soft-delete a block. The block is marked as deleted but its history is preserved in the event store."
    )]
    async fn elfiee_block_delete(
        &self,
        Parameters(input): Parameters<BlockInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            "core.delete",
            Some(input.block_id),
            json!({}),
        )
        .await
    }

    /// Rename a block
    #[tool(description = "Rename a block")]
    async fn elfiee_block_rename(
        &self,
        Parameters(input): Parameters<BlockRenameInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            "core.rename",
            Some(input.block_id),
            json!({ "name": input.name }),
        )
        .await
    }

    /// Add a relation between two blocks
    #[tool(description = "Add a relation between two blocks (parent -> child)")]
    async fn elfiee_block_link(
        &self,
        Parameters(input): Parameters<BlockLinkInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            "core.link",
            Some(input.parent_id),
            json!({
                "target_id": input.child_id,
                "relation": input.relation
            }),
        )
        .await
    }

    /// Remove a relation between two blocks
    #[tool(description = "Remove a relation between two blocks")]
    async fn elfiee_block_unlink(
        &self,
        Parameters(input): Parameters<BlockUnlinkInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            "core.unlink",
            Some(input.parent_id),
            json!({
                "target_id": input.child_id,
                "relation": input.relation
            }),
        )
        .await
    }

    /// Change a block's type
    #[tool(description = "Change a block's type")]
    async fn elfiee_block_change_type(
        &self,
        Parameters(input): Parameters<BlockChangeTypeInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            "core.change_type",
            Some(input.block_id),
            json!({ "block_type": input.new_type }),
        )
        .await
    }

    /// Update block metadata
    #[tool(description = "Update block metadata")]
    async fn elfiee_block_update_metadata(
        &self,
        Parameters(input): Parameters<BlockUpdateMetadataInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            "core.update_metadata",
            Some(input.block_id),
            json!({ "metadata": input.metadata }),
        )
        .await
    }

    // ========================================================================
    // Markdown Operations
    // ========================================================================

    /// Read markdown content from a markdown block
    #[tool(
        description = "Read the full markdown content from a block. Returns the block name, content, and metadata."
    )]
    async fn elfiee_markdown_read(
        &self,
        Parameters(input): Parameters<BlockInput>,
    ) -> Result<CallToolResult, McpError> {
        let file_id = self.get_file_id(&input.project)?;
        let handle = self.get_engine(&file_id)?;

        match handle.get_block(input.block_id.clone()).await {
            Some(block) => {
                if block.block_type != "markdown" {
                    return Ok(CallToolResult::error(vec![Content::text(
                        serde_json::to_string_pretty(&json!({
                            "error": format!("Block '{}' is type '{}', not 'markdown'", block.name, block.block_type),
                            "hint": "Use elfiee_code_read for code blocks, or elfiee_block_get for any block type.",
                        }))
                        .unwrap(),
                    )]));
                }
                let content = block
                    .contents
                    .get("markdown")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&json!({
                        "block_id": block.block_id,
                        "name": block.name,
                        "content": content,
                        "content_length": content.len(),
                        "updated_at": block.metadata.updated_at,
                    }))
                    .unwrap(),
                )]))
            }
            None => Ok(CallToolResult::error(vec![Content::text(
                serde_json::to_string_pretty(&json!({
                    "error": format!("Block '{}' not found", input.block_id),
                    "hint": "Use elfiee_block_list to see available blocks.",
                }))
                .unwrap(),
            )])),
        }
    }

    /// Write markdown content to a markdown block
    #[tool(
        description = "Write or overwrite the full markdown content of a markdown block. Returns the updated block state."
    )]
    async fn elfiee_markdown_write(
        &self,
        Parameters(input): Parameters<ContentWriteInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            "markdown.write",
            Some(input.block_id),
            json!({ "content": input.content }),
        )
        .await
    }

    // ========================================================================
    // Code Operations
    // ========================================================================

    /// Read code content from a code block
    #[tool(
        description = "Read the full code content from a code block. Returns the block name, language, content, and metadata."
    )]
    async fn elfiee_code_read(
        &self,
        Parameters(input): Parameters<BlockInput>,
    ) -> Result<CallToolResult, McpError> {
        let file_id = self.get_file_id(&input.project)?;
        let handle = self.get_engine(&file_id)?;

        match handle.get_block(input.block_id.clone()).await {
            Some(block) => {
                if block.block_type != "code" {
                    return Ok(CallToolResult::error(vec![Content::text(
                        serde_json::to_string_pretty(&json!({
                            "error": format!("Block '{}' is type '{}', not 'code'", block.name, block.block_type),
                            "hint": "Use elfiee_markdown_read for markdown blocks, or elfiee_block_get for any block type.",
                        }))
                        .unwrap(),
                    )]));
                }
                let content = block
                    .contents
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let language = block
                    .contents
                    .get("language")
                    .and_then(|v| v.as_str())
                    .unwrap_or("plaintext");
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&json!({
                        "block_id": block.block_id,
                        "name": block.name,
                        "language": language,
                        "content": content,
                        "content_length": content.len(),
                        "updated_at": block.metadata.updated_at,
                    }))
                    .unwrap(),
                )]))
            }
            None => Ok(CallToolResult::error(vec![Content::text(
                serde_json::to_string_pretty(&json!({
                    "error": format!("Block '{}' not found", input.block_id),
                    "hint": "Use elfiee_block_list to see available blocks.",
                }))
                .unwrap(),
            )])),
        }
    }

    /// Write code content to a code block
    #[tool(
        description = "Write or overwrite the full code content of a code block. Returns the updated block state."
    )]
    async fn elfiee_code_write(
        &self,
        Parameters(input): Parameters<ContentWriteInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            "code.write",
            Some(input.block_id),
            json!({ "content": input.content }),
        )
        .await
    }

    // ========================================================================
    // Directory Operations
    // ========================================================================

    /// Create a file or directory in a directory block
    #[tool(description = "Create a file or directory in a directory block")]
    async fn elfiee_directory_create(
        &self,
        Parameters(input): Parameters<DirectoryCreateInput>,
    ) -> Result<CallToolResult, McpError> {
        let mut payload = json!({
            "path": input.path,
            "type": input.entry_type,
            "source": input.source
        });

        if let Some(content) = input.content {
            payload["content"] = json!(content);
        }
        if let Some(block_type) = input.block_type {
            payload["block_type"] = json!(block_type);
        }
        if let Some(existing_block_id) = input.existing_block_id {
            payload["existing_block_id"] = json!(existing_block_id);
        }

        self.execute_capability(
            &input.project,
            "directory.create",
            Some(input.block_id),
            payload,
        )
        .await
    }

    /// Delete a file or directory from a directory block
    #[tool(description = "Delete a file or directory from a directory block")]
    async fn elfiee_directory_delete(
        &self,
        Parameters(input): Parameters<DirectoryDeleteInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            "directory.delete",
            Some(input.block_id),
            json!({ "path": input.path }),
        )
        .await
    }

    /// Rename or move a file/directory
    #[tool(description = "Rename or move a file/directory within a directory block")]
    async fn elfiee_directory_rename(
        &self,
        Parameters(input): Parameters<DirectoryRenameInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            "directory.rename",
            Some(input.block_id),
            json!({
                "old_path": input.old_path,
                "new_path": input.new_path
            }),
        )
        .await
    }

    /// Update directory index entries
    #[tool(description = "Update directory index entries")]
    async fn elfiee_directory_write(
        &self,
        Parameters(input): Parameters<DirectoryWriteInput>,
    ) -> Result<CallToolResult, McpError> {
        let mut payload = json!({ "entries": input.entries });
        if let Some(source) = input.source {
            payload["source"] = json!(source);
        }

        self.execute_capability(
            &input.project,
            "directory.write",
            Some(input.block_id),
            payload,
        )
        .await
    }

    /// Import files from filesystem into a directory block
    #[tool(description = "Import files from filesystem into a directory block")]
    async fn elfiee_directory_import(
        &self,
        Parameters(input): Parameters<DirectoryImportInput>,
    ) -> Result<CallToolResult, McpError> {
        let mut payload = json!({ "source_path": input.source_path });
        if let Some(target_path) = input.target_path {
            payload["target_path"] = json!(target_path);
        }

        self.execute_capability(
            &input.project,
            "directory.import",
            Some(input.block_id),
            payload,
        )
        .await
    }

    /// Export directory block contents to filesystem
    #[tool(description = "Export directory block contents to filesystem")]
    async fn elfiee_directory_export(
        &self,
        Parameters(input): Parameters<DirectoryExportInput>,
    ) -> Result<CallToolResult, McpError> {
        let file_id = self.get_file_id(&input.project)?;
        let editor_id = self.resolve_agent_editor_id(&file_id).await?;

        let payload = crate::extensions::directory::DirectoryExportPayload {
            target_path: input.target_path,
            source_path: input.source_path,
        };

        match crate::commands::checkout::do_checkout_workspace(
            &self.app_state,
            &file_id,
            &editor_id,
            &input.block_id,
            &payload,
        )
        .await
        {
            Ok(()) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&json!({
                    "ok": true,
                    "capability": "directory.export",
                    "editor": editor_id,
                    "target_path": payload.target_path,
                    "message": "Directory exported successfully",
                }))
                .unwrap(),
            )])),
            Err(e) => {
                let hint = Self::error_hint("directory.export", &e);
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&json!({
                        "ok": false,
                        "capability": "directory.export",
                        "error": e,
                        "hint": hint,
                    }))
                    .unwrap(),
                )]))
            }
        }
    }

    // ========================================================================
    // Terminal Operations
    // ========================================================================

    /// Initialize a terminal session for a terminal block
    #[tool(description = "Initialize a terminal session for a terminal block")]
    async fn elfiee_terminal_init(
        &self,
        Parameters(input): Parameters<TerminalInitInput>,
    ) -> Result<CallToolResult, McpError> {
        let file_id = self.get_file_id(&input.project)?;
        let editor_id = self.resolve_agent_editor_id(&file_id).await?;

        let payload = json!({
            "cols": 80,
            "rows": 24,
            "block_id": input.block_id,
            "editor_id": editor_id,
            "file_id": file_id,
        });

        // Record terminal.init event
        let result = self
            .execute_capability(
                &input.project,
                "terminal.init",
                Some(input.block_id.clone()),
                payload,
            )
            .await?;

        // Actually start a PTY session (MCP mode — no frontend events)
        crate::extensions::terminal::commands::start_pty_for_mcp(
            &self.app_state.terminal_sessions,
            &self.app_state.terminal_output_buffers,
            &input.block_id,
            80,
            24,
            None,
        )
        .map_err(|e| mcp::invalid_payload(format!("Failed to start PTY: {}", e)))?;

        Ok(result)
    }

    /// Execute a command in a terminal block
    #[tool(description = "Execute a command in a terminal block")]
    async fn elfiee_terminal_execute(
        &self,
        Parameters(input): Parameters<TerminalExecuteInput>,
    ) -> Result<CallToolResult, McpError> {
        // Record terminal.execute event (best effort — don't fail if this errors)
        let _ = self
            .execute_capability(
                &input.project,
                "terminal.execute",
                Some(input.block_id.clone()),
                json!({ "command": input.command }),
            )
            .await;

        // Check if session exists
        {
            let sessions = self.app_state.terminal_sessions.lock().unwrap();
            if !sessions.contains_key(&input.block_id) {
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&json!({
                        "ok": false,
                        "error": "Terminal session not found",
                        "hint": "Call elfiee_terminal_init first to start a terminal session.",
                    }))
                    .unwrap(),
                )]));
            }
        }

        // Clear output buffer before sending command
        if let Some(buf) = self.app_state.terminal_output_buffers.get(&input.block_id) {
            buf.lock().unwrap().clear();
        }

        // Write command + newline to PTY
        {
            let mut sessions = self.app_state.terminal_sessions.lock().unwrap();
            if let Some(session) = sessions.get_mut(&input.block_id) {
                crate::extensions::terminal::pty::write(
                    &mut *session.writer,
                    &format!("{}\n", input.command),
                )
                .map_err(|e| mcp::invalid_payload(format!("Failed to write to PTY: {}", e)))?;
            }
        }

        // Capture output with quiescence detection
        let output =
            Self::capture_terminal_output(&self.app_state.terminal_output_buffers, &input.block_id)
                .await;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "block_id": input.block_id,
                "command": input.command,
                "output": output,
            }))
            .unwrap(),
        )]))
    }

    /// Save terminal session content
    #[tool(description = "Save terminal session content")]
    async fn elfiee_terminal_save(
        &self,
        Parameters(input): Parameters<TerminalSaveInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            "terminal.save",
            Some(input.block_id),
            json!({ "saved_content": input.content, "saved_at": crate::utils::time::now_utc() }),
        )
        .await
    }

    /// Close a terminal session
    #[tool(description = "Close a terminal session")]
    async fn elfiee_terminal_close(
        &self,
        Parameters(input): Parameters<BlockInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            "terminal.close",
            Some(input.block_id),
            json!({}),
        )
        .await
    }

    // ========================================================================
    // Permission Operations
    // ========================================================================

    /// Grant a capability to an editor on a block
    #[tool(
        description = "Grant a capability (e.g. 'markdown.write', 'code.write') to an editor on a specific block. The block owner can always perform all operations without explicit grants."
    )]
    async fn elfiee_grant(
        &self,
        Parameters(input): Parameters<GrantInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            "core.grant",
            Some(input.block_id.clone()),
            json!({
                "target_editor": input.editor_id,
                "capability": input.cap_id,
                "target_block": input.block_id
            }),
        )
        .await
    }

    /// Revoke a capability from an editor on a block
    #[tool(
        description = "Revoke a previously granted capability from an editor on a specific block."
    )]
    async fn elfiee_revoke(
        &self,
        Parameters(input): Parameters<GrantInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            "core.revoke",
            Some(input.block_id.clone()),
            json!({
                "target_editor": input.editor_id,
                "capability": input.cap_id,
                "target_block": input.block_id
            }),
        )
        .await
    }

    // ========================================================================
    // Editor Operations
    // ========================================================================

    /// Create a new editor in the project
    #[tool(description = "Create a new editor in the project")]
    async fn elfiee_editor_create(
        &self,
        Parameters(input): Parameters<EditorInput>,
    ) -> Result<CallToolResult, McpError> {
        let name = input.name.unwrap_or_else(|| input.editor_id.clone());
        let payload = json!({
            "name": name,
            "editor_id": input.editor_id,
        });

        self.execute_capability(&input.project, "core.editor_create", None, payload)
            .await
    }

    /// Delete an editor from the project
    #[tool(description = "Delete an editor from the project")]
    async fn elfiee_editor_delete(
        &self,
        Parameters(input): Parameters<EditorInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            "core.editor_delete",
            None,
            json!({ "editor_id": input.editor_id }),
        )
        .await
    }

    // ========================================================================
    // Task Operations
    // ========================================================================

    /// Create a new task block
    #[tool(
        description = "Create a new task block for tracking work. Returns the task block ID. Use elfiee_task_link to link implementation blocks, then elfiee_task_commit to commit to git."
    )]
    async fn elfiee_task_create(
        &self,
        Parameters(input): Parameters<TaskCreateInput>,
    ) -> Result<CallToolResult, McpError> {
        let file_id = self.get_file_id(&input.project)?;
        let editor_id = self.resolve_agent_editor_id(&file_id).await?;
        let handle = self.get_engine(&file_id)?;

        // 1. Create task block
        let cmd = Command::new(
            editor_id.clone(),
            "core.create".to_string(),
            String::new(),
            json!({ "name": input.name, "block_type": "task" }),
        );

        let events = handle
            .process_command(cmd)
            .await
            .map_err(|e| mcp::invalid_payload(format!("Failed to create task: {}", e)))?;

        let task_block_id = events
            .first()
            .map(|ev| ev.entity.clone())
            .unwrap_or_default();

        // 2. If description provided, update metadata (best effort)
        if let Some(description) = &input.description {
            let meta_cmd = Command::new(
                editor_id.clone(),
                "core.update_metadata".to_string(),
                task_block_id.clone(),
                json!({ "metadata": { "description": description } }),
            );
            if let Err(e) = handle.process_command(meta_cmd).await {
                log::warn!("Failed to set task description: {}", e);
            }
        }

        // 3. Return result with block details
        let mut result = json!({
            "ok": true,
            "created_block_id": task_block_id,
            "name": input.name,
            "block_type": "task",
            "editor": editor_id,
        });

        if let Some(block) = handle.get_block(task_block_id.clone()).await {
            result["block"] = Self::format_block_summary(&block);
        }
        if let Some(desc) = &input.description {
            result["description"] = json!(desc);
        }

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap(),
        )]))
    }

    /// Write content to a task block
    #[tool(
        description = "Write or overwrite the markdown content of a task block. Use this to document the task requirements or progress."
    )]
    async fn elfiee_task_write(
        &self,
        Parameters(input): Parameters<TaskWriteInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            "task.write",
            Some(input.block_id),
            json!({ "content": input.content }),
        )
        .await
    }

    /// Commit a task to git
    #[tool(
        description = "Commit a task: export linked code blocks to their git repositories, create a feature branch, and commit. Returns commit hash, branch name, and exported files."
    )]
    async fn elfiee_task_commit(
        &self,
        Parameters(input): Parameters<TaskCommitInput>,
    ) -> Result<CallToolResult, McpError> {
        let file_id = self.get_file_id(&input.project)?;
        let editor_id = self.resolve_agent_editor_id(&file_id).await?;

        match crate::commands::task::do_commit_task(
            &self.app_state,
            &file_id,
            &editor_id,
            &input.block_id,
        )
        .await
        {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&json!({
                    "ok": true,
                    "commit_hash": result.commit_hash,
                    "branch_name": result.branch_name,
                    "exported_files": result.exported_files,
                }))
                .unwrap(),
            )])),
            Err(e) => {
                let hint = Self::error_hint("task.commit", &e);
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&json!({
                        "ok": false,
                        "error": e,
                        "hint": hint,
                    }))
                    .unwrap(),
                )]))
            }
        }
    }

    /// Link a block to a task as implementation
    #[tool(
        description = "Link a code or markdown block to a task as its implementation. Uses 'implement' relation. Idempotent — safe to call multiple times for the same pair."
    )]
    async fn elfiee_task_link(
        &self,
        Parameters(input): Parameters<TaskLinkInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            "core.link",
            Some(input.task_id),
            json!({
                "target_id": input.block_id,
                "relation": "implement"
            }),
        )
        .await
    }

    // ========================================================================
    // Generic Execution
    // ========================================================================

    /// Execute any capability directly (advanced usage)
    #[tool(
        description = "Execute any capability directly (advanced usage). Use this for capabilities not covered by specific tools."
    )]
    async fn elfiee_exec(
        &self,
        Parameters(input): Parameters<ExecInput>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_capability(
            &input.project,
            &input.capability,
            input.block_id,
            input.payload.unwrap_or(json!({})),
        )
        .await
    }
}

// ============================================================================
// ServerHandler Implementation
// ============================================================================

#[tool_handler]
impl rmcp::handler::server::ServerHandler for ElfieeMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: crate::commands::agent::MCP_SERVER_NAME.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some(
                "Elfiee MCP Server for .elf file operations. \
                Use elfiee_file_list to see open files, then use other tools to interact with blocks. \
                Elfiee GUI must be running with files open for MCP to work."
                    .to_string(),
            ),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability::default()),
                resources: Some(ResourcesCapability::default()),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListResourcesResult, McpError>> + Send + '_ {
        async {
            let files = self.app_state.list_open_files();
            let mut resources = Vec::new();

            // Helper to wrap raw resource
            let resource = |raw: RawResource| Annotated {
                raw,
                annotations: None,
            };

            // Static resource: list of open files
            resources.push(resource(RawResource {
                uri: "elfiee://files".to_string(),
                name: "Open Files".to_string(),
                description: Some("List of currently open .elf project files".to_string()),
                mime_type: Some("application/json".to_string()),
                size: None,
            }));

            // Dynamic resources: per-project blocks and grants
            for (file_id, path) in &files {
                resources.push(resource(RawResource {
                    uri: format!("elfiee://{}/blocks", path),
                    name: format!("Blocks in {}", path),
                    description: Some(format!("All blocks in project {}", path)),
                    mime_type: Some("application/json".to_string()),
                    size: None,
                }));

                resources.push(resource(RawResource {
                    uri: format!("elfiee://{}/grants", path),
                    name: format!("Grants in {}", path),
                    description: Some(format!("Permission grants in project {}", path)),
                    mime_type: Some("application/json".to_string()),
                    size: None,
                }));

                resources.push(resource(RawResource {
                    uri: format!("elfiee://{}/events", path),
                    name: format!("Events in {}", path),
                    description: Some(format!("Event log for project {}", path)),
                    mime_type: Some("application/json".to_string()),
                    size: None,
                }));

                // Individual block resources
                if let Some(handle) = self.app_state.engine_manager.get_engine(file_id) {
                    let blocks = handle.get_all_blocks().await;
                    for block in blocks.values() {
                        let mime = match block.block_type.as_str() {
                            "markdown" => "text/markdown",
                            "code" => "text/plain",
                            _ => "application/json",
                        };
                        resources.push(resource(RawResource {
                            uri: format!("elfiee://{}/block/{}", path, block.block_id),
                            name: block.name.clone(),
                            description: Some(format!("[{}] {}", block.block_type, block.name)),
                            mime_type: Some(mime.to_string()),
                            size: None,
                        }));
                    }
                }
            }

            Ok(ListResourcesResult {
                resources,
                next_cursor: None,
            })
        }
    }

    fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListResourceTemplatesResult, McpError>> + Send + '_ {
        async {
            let template = |raw: RawResourceTemplate| Annotated {
                raw,
                annotations: None,
            };

            Ok(ListResourceTemplatesResult {
                resource_templates: vec![
                    template(RawResourceTemplate {
                        uri_template: "elfiee://{project}/blocks".to_string(),
                        name: "Project Blocks".to_string(),
                        description: Some("List all blocks in a project".to_string()),
                        mime_type: Some("application/json".to_string()),
                    }),
                    template(RawResourceTemplate {
                        uri_template: "elfiee://{project}/block/{block_id}".to_string(),
                        name: "Block Content".to_string(),
                        description: Some("Read a specific block's full content".to_string()),
                        mime_type: Some("application/json".to_string()),
                    }),
                    template(RawResourceTemplate {
                        uri_template: "elfiee://{project}/grants".to_string(),
                        name: "Project Grants".to_string(),
                        description: Some("Permission grants in a project".to_string()),
                        mime_type: Some("application/json".to_string()),
                    }),
                    template(RawResourceTemplate {
                        uri_template: "elfiee://{project}/events".to_string(),
                        name: "Event Log".to_string(),
                        description: Some("Event sourcing log for a project".to_string()),
                        mime_type: Some("application/json".to_string()),
                    }),
                ],
                next_cursor: None,
            })
        }
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ReadResourceResult, McpError>> + Send + '_ {
        async move {
            let uri = &request.uri;

            // Handle static resource: elfiee://files
            if uri == "elfiee://files" {
                let files = self.app_state.list_open_files();
                let result: Vec<serde_json::Value> = files
                    .iter()
                    .map(|(file_id, path)| {
                        json!({
                            "file_id": file_id,
                            "project": path,
                        })
                    })
                    .collect();

                return Ok(ReadResourceResult {
                    contents: vec![ResourceContents::TextResourceContents {
                        uri: uri.clone(),
                        mime_type: Some("application/json".to_string()),
                        text: serde_json::to_string_pretty(&json!({
                            "files": result,
                            "count": files.len(),
                        }))
                        .unwrap(),
                    }],
                });
            }

            // Parse URI: elfiee://{project}/...
            let stripped = uri
                .strip_prefix("elfiee://")
                .ok_or_else(|| mcp::invalid_payload("URI must start with elfiee://"))?;

            // Find which project this URI refers to
            let files = self.app_state.list_open_files();
            let (file_id, project, remainder) = files
                .iter()
                .filter_map(|(fid, path)| {
                    stripped
                        .strip_prefix(path.as_str())
                        .map(|rest| (fid.clone(), path.clone(), rest.to_string()))
                })
                .next()
                .ok_or_else(|| mcp::project_not_open(&format!("(parsed from URI: {})", uri)))?;

            let remainder = remainder.trim_start_matches('/');
            let handle = self.get_engine(&file_id)?;

            match remainder {
                // elfiee://{project}/blocks
                "blocks" => {
                    let blocks = handle.get_all_blocks().await;
                    let result: Vec<serde_json::Value> = blocks
                        .values()
                        .map(|block| Self::format_block_summary(block))
                        .collect();

                    Ok(ReadResourceResult {
                        contents: vec![ResourceContents::TextResourceContents {
                            uri: uri.clone(),
                            mime_type: Some("application/json".to_string()),
                            text: serde_json::to_string_pretty(&json!({
                                "project": project,
                                "blocks": result,
                                "count": blocks.len(),
                            }))
                            .unwrap(),
                        }],
                    })
                }

                // elfiee://{project}/grants
                "grants" => {
                    let blocks = handle.get_all_blocks().await;
                    let mut all_grants = Vec::new();
                    for block in blocks.values() {
                        let grants = handle.get_block_grants(block.block_id.clone()).await;
                        for (editor_id, cap_id, _) in &grants {
                            all_grants.push(json!({
                                "block_id": block.block_id,
                                "block_name": block.name,
                                "editor": editor_id,
                                "capability": cap_id,
                            }));
                        }
                    }

                    Ok(ReadResourceResult {
                        contents: vec![ResourceContents::TextResourceContents {
                            uri: uri.clone(),
                            mime_type: Some("application/json".to_string()),
                            text: serde_json::to_string_pretty(&json!({
                                "project": project,
                                "grants": all_grants,
                                "count": all_grants.len(),
                            }))
                            .unwrap(),
                        }],
                    })
                }

                // elfiee://{project}/events
                "events" => {
                    let events = handle
                        .get_all_events()
                        .await
                        .map_err(|e| mcp::invalid_payload(format!("Failed to read events: {}", e)))?;
                    let result: Vec<serde_json::Value> = events
                        .iter()
                        .map(|ev| {
                            json!({
                                "event_id": ev.event_id,
                                "entity": ev.entity,
                                "attribute": ev.attribute,
                                "value": ev.value,
                                "timestamp": ev.timestamp,
                                "created_at": ev.created_at,
                            })
                        })
                        .collect();

                    Ok(ReadResourceResult {
                        contents: vec![ResourceContents::TextResourceContents {
                            uri: uri.clone(),
                            mime_type: Some("application/json".to_string()),
                            text: serde_json::to_string_pretty(&json!({
                                "project": project,
                                "events": result,
                                "count": result.len(),
                            }))
                            .unwrap(),
                        }],
                    })
                }

                // elfiee://{project}/block/{block_id}
                rest if rest.starts_with("block/") => {
                    let block_id = rest.strip_prefix("block/").unwrap();
                    match handle.get_block(block_id.to_string()).await {
                        Some(block) => {
                            // Return raw content for content-type blocks
                            let (text, mime) = match block.block_type.as_str() {
                                "markdown" => {
                                    let content = block
                                        .contents
                                        .get("markdown")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("");
                                    (content.to_string(), "text/markdown".to_string())
                                }
                                "code" => {
                                    let content = block
                                        .contents
                                        .get("text")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("");
                                    (content.to_string(), "text/plain".to_string())
                                }
                                _ => {
                                    let full = json!({
                                        "block_id": block.block_id,
                                        "name": block.name,
                                        "block_type": block.block_type,
                                        "owner": block.owner,
                                        "contents": block.contents,
                                        "children": block.children,
                                        "metadata": {
                                            "description": block.metadata.description,
                                            "created_at": block.metadata.created_at,
                                            "updated_at": block.metadata.updated_at,
                                            "custom": block.metadata.custom,
                                        },
                                    });
                                    (
                                        serde_json::to_string_pretty(&full).unwrap(),
                                        "application/json".to_string(),
                                    )
                                }
                            };

                            Ok(ReadResourceResult {
                                contents: vec![ResourceContents::TextResourceContents {
                                    uri: uri.clone(),
                                    mime_type: Some(mime),
                                    text,
                                }],
                            })
                        }
                        None => Err(mcp::block_not_found(block_id)),
                    }
                }

                _ => Err(mcp::invalid_payload(format!(
                    "Unknown resource path: '{}'. Valid paths: blocks, block/{{id}}, grants, events",
                    remainder
                ))),
            }
        }
    }
}
