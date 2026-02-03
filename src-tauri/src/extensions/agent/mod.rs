//! Agent Extension (Phase 2 — External AI Tool Integration)
//!
//! AI assistant integration for Elfiee via external tools (Claude Code, etc.).
//!
//! ## Architecture
//!
//! Each Agent binds to an AI tool config directory path (e.g. `.claude/`, `.cursor/`).
//! Each Agent has its own MCP server on a dedicated port for correct identity routing.
//!
//! ## Capabilities
//!
//! - `agent.create` - Create Agent Block bound to an AI tool config directory
//! - `agent.enable` - Enable agent: create symlink + inject MCP config + start MCP server
//! - `agent.disable` - Disable agent: clean symlink + remove MCP config + stop MCP server
//!
//! ## Architecture Note
//!
//! Capability handlers only update block state (pure). Actual I/O operations
//! (symlink creation, MCP config injection/removal, MCP server start/stop)
//! are performed by the Tauri command layer in `commands/agent.rs`.
//!
//! ## Payload Types
//!
//! - `AgentCreatePayload` - Parameters for agent.create
//! - `AgentEnablePayload` - Parameters for agent.enable
//! - `AgentDisablePayload` - Parameters for agent.disable

use serde::{Deserialize, Serialize};
use specta::Type;

pub mod agent_create;
pub mod agent_disable;
pub mod agent_enable;
pub mod mcp_config;
pub mod settings_config;

// Re-export capability handlers for registration
pub use agent_create::*;
pub use agent_disable::*;
pub use agent_enable::*;

// --- Agent Block Contents ---

/// Agent Block contents, storing per-AI-tool integration config.
///
/// Stored in `Block.contents`.
///
/// Key change from V1: binds to `config_dir` (absolute path) instead of Dir Block ID.
/// `editor_id` is now required (not optional).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AgentContents {
    /// Agent display name (default: "elfiee")
    pub name: String,

    /// AI tool provider identifier.
    ///
    /// Examples: "claude_code", "cursor", "windsurf"
    /// Used to determine provider-specific behavior (symlink paths, MCP config format, etc.)
    #[serde(default = "default_provider")]
    pub provider: String,

    /// Absolute path to the AI tool's config directory this agent is bound to.
    ///
    /// Examples:
    /// - Claude Code: "/home/user/repo-a/.claude"
    /// - Cursor: "/home/user/repo-a/.cursor"
    ///
    /// Used to derive:
    /// - Symlink target: `{config_dir}/skills/elfiee-client/`
    /// - MCP config locations: `{config_dir.parent()}/.mcp.json` + `{config_dir}/mcp.json`
    pub config_dir: String,

    /// Agent current status
    pub status: AgentStatus,

    /// Bot editor_id associated with this agent (required).
    /// Used by per-agent MCP server to attribute operations to the correct identity.
    pub editor_id: String,
}

fn default_provider() -> String {
    "claude_code".to_string()
}

/// Agent enable/disable status
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    /// Enabled: symlink exists, MCP config injected, MCP server running
    Enabled,
    /// Disabled: symlink cleaned, MCP config removed, MCP server stopped
    Disabled,
}

// --- Payload Types ---

/// Payload for agent.create capability
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AgentCreatePayload {
    /// Agent display name (optional, default: "elfiee")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// AI tool provider identifier (optional, default: "claude_code")
    #[serde(default = "default_provider")]
    pub provider: String,

    /// Absolute path to the AI tool's config directory (required)
    pub config_dir: String,

    /// Bot editor_id to associate with this agent.
    /// If not provided, the command layer auto-creates a bot editor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub editor_id: Option<String>,
}

/// Payload for agent.enable capability
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AgentEnablePayload {
    /// Agent Block ID (required)
    pub agent_block_id: String,
}

/// Payload for agent.disable capability
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AgentDisablePayload {
    /// Agent Block ID (required)
    pub agent_block_id: String,
}

// --- Result Types ---

/// Result type for agent.create Tauri command
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AgentCreateResult {
    /// Created Agent Block ID
    pub agent_block_id: String,
    /// Agent status after creation
    pub status: AgentStatus,
    /// Whether the user needs to restart Claude Code
    pub needs_restart: bool,
    /// Human-readable message
    pub message: String,
}

/// Result type for agent.enable Tauri command
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AgentEnableResult {
    /// Agent Block ID
    pub agent_block_id: String,
    /// Agent status after enable
    pub status: AgentStatus,
    /// Whether the user needs to restart Claude Code
    pub needs_restart: bool,
    /// Human-readable message
    pub message: String,
    /// Warnings for partial failures (e.g. symlink OK but MCP config failed)
    pub warnings: Vec<String>,
}

/// Result type for agent.disable Tauri command
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AgentDisableResult {
    /// Agent Block ID
    pub agent_block_id: String,
    /// Agent status after disable
    pub status: AgentStatus,
    /// Human-readable message
    pub message: String,
    /// Warnings for partial failures
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests;
