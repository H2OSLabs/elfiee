//! Agent Extension
//!
//! AI assistant integration for Elfiee.
//!
//! ## Architecture
//!
//! - `agent_create` - Create Agent Block with Editor
//! - `agent_configure` - Configure Agent settings
//! - `agent_invoke` - Invoke LLM and generate Proposal
//! - `agent_approve` - Approve and execute Proposal
//!
//! ## Payload Types
//!
//! - `AgentCreatePayload` - Parameters for agent.create
//! - `AgentConfigurePayload` - Parameters for agent.configure
//! - `AgentInvokePayload` - Parameters for agent.invoke
//! - `AgentApprovePayload` - Parameters for agent.approve

use serde::{Deserialize, Serialize};
use specta::Type;

pub mod agent_configure;
pub mod agent_create;
pub mod context;
pub mod llm;

// Re-export capability handlers for registration
pub use agent_configure::*;
pub use agent_create::*;

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

#[cfg(test)]
mod tests;
