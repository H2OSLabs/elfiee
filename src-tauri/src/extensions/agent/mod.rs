//! Agent Extension
//!
//! AI assistant integration for Elfiee.
//!
//! ## Architecture
//!
//! ### Phase 1 (LLM Direct)
//! - `agent_create` - Create Agent Block with Editor
//! - `agent_configure` - Configure Agent settings
//! - `agent_invoke` - Invoke LLM and generate Proposal (future)
//! - `agent_approve` - Approve and execute Proposal (future)
//!
//! ### Phase 2 (External AI Tool Integration)
//! - `agent.create` - Create Agent Block for external project + auto-enable
//! - `agent.enable` - Enable agent: create symlink + inject MCP config
//! - `agent.disable` - Disable agent: clean symlink + remove MCP config
//!
//! ## Payload Types
//!
//! ### Phase 1
//! - `AgentCreatePayload` - Parameters for agent.create (LLM mode)
//! - `AgentConfigurePayload` - Parameters for agent.configure
//! - `AgentInvokePayload` - Parameters for agent.invoke
//! - `AgentApprovePayload` - Parameters for agent.approve
//!
//! ### Phase 2
//! - `AgentCreateV2Payload` - Parameters for agent.create (project integration mode)
//! - `AgentEnablePayload` - Parameters for agent.enable
//! - `AgentDisablePayload` - Parameters for agent.disable

use serde::{Deserialize, Serialize};
use specta::Type;

pub mod agent_configure;
pub mod agent_create;
pub mod agent_disable;
pub mod agent_enable;
pub mod context;
pub mod llm;

// Re-export capability handlers for registration
pub use agent_configure::*;
pub use agent_create::*;
pub use agent_disable::*;
pub use agent_enable::*;

// --- Core Types ---

/// Agent configuration stored in Block.contents
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AgentConfig {
    /// Associated Editor ID for the agent (format: "agent-{uuid}")
    pub editor_id: String,
    /// LLM provider (e.g., "anthropic", "openai")
    pub provider: String,
    /// Model name (e.g., "claude-sonnet-4-20250514")
    pub model: String,
    /// Environment variable name for API key
    pub api_key_env: String,
    /// System prompt for the agent
    pub system_prompt: String,
}

/// A proposed command from LLM output
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ProposedCommand {
    /// Capability ID (e.g., "code.write", "terminal.execute")
    pub cap_id: String,
    /// Target block ID
    pub block_id: String,
    /// Command payload
    pub payload: serde_json::Value,
    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Proposal status
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProposalStatus {
    Pending,
    Approved,
    Rejected,
}

/// Proposal structure stored in Event.value
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Proposal {
    /// List of proposed commands
    pub proposed_commands: Vec<ProposedCommand>,
    /// Current status
    pub status: ProposalStatus,
    /// Original user prompt
    pub prompt: String,
    /// Raw LLM response (for debugging)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_response: Option<String>,
}

// --- Payload Types ---

/// Payload for agent.create capability
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AgentCreatePayload {
    /// Agent display name
    pub name: String,
    /// LLM provider
    pub provider: String,
    /// Model name
    pub model: String,
    /// Environment variable name for API key
    pub api_key_env: String,
    /// System prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
}

/// Payload for agent.configure capability
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AgentConfigurePayload {
    /// Optional: Update provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Optional: Update model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Optional: Update API key env var
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    /// Optional: Update system prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
}

/// Payload for agent.invoke capability
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AgentInvokePayload {
    /// User prompt/message
    pub prompt: String,
    /// Optional: Max tokens for context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_context_tokens: Option<u32>,
    /// Optional: Related block IDs for context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_block_ids: Option<Vec<String>>,
}

/// Payload for agent.approve capability
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AgentApprovePayload {
    /// Event ID of the Proposal to approve
    pub proposal_event_id: String,
    /// Approve or reject
    pub approved: bool,
}

// --- Phase 2 Types (External AI Tool Integration) ---

/// Phase 2 Agent Block contents, storing project-level AI integration config.
///
/// Coexists with Phase 1's `AgentConfig` (LLM direct call config).
/// Stored in `Block.contents`.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AgentContents {
    /// Agent display name (default: "elfiee")
    pub name: String,

    /// Associated external project Dir Block ID.
    ///
    /// Used to look up the Dir Block in StateProjector,
    /// then get the physical path from `metadata.custom["external_root_path"]`.
    pub target_project_id: String,

    /// Agent current status
    pub status: AgentStatus,
}

/// Agent enable/disable status
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    /// Enabled: symlink exists, MCP config injected
    Enabled,
    /// Disabled: symlink cleaned, MCP config removed
    Disabled,
}

/// Payload for Phase 2 agent.create capability
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AgentCreateV2Payload {
    /// Agent display name (optional, default: "elfiee")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Associated external project Dir Block ID (required)
    pub target_project_id: String,
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
