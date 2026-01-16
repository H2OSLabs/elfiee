//! Context management for Agent extension
//!
//! This module provides:
//! - `collect_context`: Collect context from related blocks
//! - `truncate_context`: Truncate context to fit token limits
//! - `format_context_as_markdown`: Format context for LLM consumption

pub mod collector;
pub mod truncator;

pub use collector::{collect_context, format_context_as_markdown, ContextItem};
pub use truncator::{count_tokens, truncate_context};
