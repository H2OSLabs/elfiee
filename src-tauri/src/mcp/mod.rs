//! MCP (Model Context Protocol) Server Module
//!
//! This module implements the MCP server for Elfiee, allowing AI agents
//! like Claude Code to interact with .elf files through a standardized protocol.
//!
//! ## Architecture
//!
//! The MCP server runs as an independent SSE server on port 47200,
//! sharing AppState with the Tauri GUI (same process).
//!
//! ```text
//! elfiee.exe process:
//! +----------------------------------------------+
//! |                                               |
//! |  React  --Tauri Cmd-->  commands/*.rs         |
//! |                              |                |
//! |  Claude --MCP SSE:47200-->  mcp/server.rs     |
//! |  Code                        |                |
//! |                              v                |
//! |              Arc<AppState> (shared)            |
//! |              +-- files: { a.elf, b.elf }      |
//! |              +-- active_editors: { ... }       |
//! |              +-- engine_manager               |
//! +----------------------------------------------+
//! ```
//!
//! ## Available Tools
//!
//! - `elfiee_file_list` - List open files
//! - `elfiee_block_list` - List blocks in a project
//! - `elfiee_block_get` - Get block details
//! - `elfiee_block_create` - Create new block
//! - `elfiee_block_delete` - Delete block
//! - `elfiee_block_rename` - Rename block
//! - `elfiee_block_link` - Add block relation
//! - `elfiee_block_unlink` - Remove block relation
//! - `elfiee_block_change_type` - Change block type
//! - `elfiee_block_update_metadata` - Update block metadata
//! - `elfiee_markdown_read/write` - Read/write markdown
//! - `elfiee_code_read/write` - Read/write code
//! - `elfiee_directory_create/delete/rename/write/import/export` - Directory operations
//! - `elfiee_terminal_init/execute/save/close` - Terminal operations
//! - `elfiee_task_create/write/commit/link` - Task operations
//! - `elfiee_grant/revoke` - Permission operations
//! - `elfiee_editor_create/delete` - Editor operations
//! - `elfiee_exec` - Execute any capability

pub mod server;
pub mod transport;

pub use server::ElfieeMcpServer;
pub use transport::{start_agent_mcp_server, start_mcp_server, stop_agent_mcp_server, MCP_PORT};

use rmcp::ErrorData as McpError;

/// MCP error: project not open
pub fn project_not_open(project: &str) -> McpError {
    McpError::invalid_request(
        format!(
            "Project '{}' is not open in the Elfiee GUI. \
            Please open this .elf file in the GUI first, then retry. \
            Use elfiee_file_list to see currently open projects.",
            project
        ),
        None,
    )
}

/// MCP error: block not found
pub fn block_not_found(block_id: &str) -> McpError {
    McpError::invalid_request(
        format!(
            "Block '{}' not found. Use elfiee_block_list to see available blocks.",
            block_id
        ),
        None,
    )
}

/// MCP error: engine not found
pub fn engine_not_found(file_id: &str) -> McpError {
    McpError::invalid_request(
        format!(
            "Engine not running for file '{}'. \
            The file may have been closed. Use elfiee_file_list to check open files.",
            file_id
        ),
        None,
    )
}

/// MCP error: no active editor
pub fn no_active_editor(file_id: &str) -> McpError {
    McpError::invalid_request(
        format!(
            "No active editor set for file '{}'. \
            Open the file in the Elfiee GUI and ensure an editor is selected. \
            This usually means the file was opened but no editor session is active.",
            file_id
        ),
        None,
    )
}

/// MCP error: invalid payload
pub fn invalid_payload(err: impl std::fmt::Display) -> McpError {
    McpError::invalid_params(
        format!(
            "Invalid payload: {}. Check the tool's parameter schema for required fields.",
            err
        ),
        None,
    )
}
