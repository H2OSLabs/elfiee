//! LLM integration for Agent extension
//!
//! This module provides:
//! - `AnthropicClient`: Client for Anthropic Claude API
//! - `parse_structured_output`: Parser for XML-formatted command proposals
//! - `LlmError`: Error types for LLM operations

pub mod anthropic;
pub mod error;
pub mod parser;

pub use anthropic::AnthropicClient;
pub use error::LlmError;
pub use parser::parse_structured_output;
