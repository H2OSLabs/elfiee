/// Task Extension
///
/// Provides capabilities for managing task blocks in Elfiee.
/// Tasks represent units of work that can be committed to external git repos.
///
/// ## Capabilities
///
/// - `task.write`: Write markdown content to a task block (same model as markdown.write)
/// - `task.read`: Read task content (permission gate + audit)
/// - `task.commit`: Generate audit event for committing task's downstream blocks
///
/// ## Design Decision: No Explicit TaskStatus
///
/// Task state is derived from event history (Event Sourcing implicit state):
/// - No `task.commit` event → Pending (has implement downstream → InProgress)
/// - Has `task.commit` event → Committed
///
/// ## Contents Structure
///
/// Task blocks store content in the same format as markdown blocks:
/// ```json
/// {
///   "markdown": "# 实现登录功能\n\n## 需求\n\n添加 OAuth 登录..."
/// }
/// ```
/// Title is `block.name`, description is `metadata.description`.
use serde::{Deserialize, Serialize};
use specta::Type;

// ============================================================================
// Module Exports
// ============================================================================

pub mod git;
pub mod git_hooks;
pub mod task_write;
pub use task_write::*;

pub mod task_read;
pub use task_read::*;

pub mod task_commit;
pub use task_commit::*;

// ============================================================================
// Payload Definitions
// ============================================================================

/// Payload for task.write capability
///
/// Contains markdown content for a task block.
/// Stored in `contents` as `{ "markdown": "..." }` — same model as markdown blocks.
/// Title is stored in `block.name`, description in `metadata.description`.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TaskWritePayload {
    /// Markdown 内容
    pub content: String,
}

/// Payload for task.read capability
///
/// task.read is a permission-only capability, similar to markdown.read.
/// No payload fields needed — the empty JSON object `{}` is accepted.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TaskReadPayload {}

/// Payload for task.commit capability
///
/// Empty payload — target repo is auto-discovered from downstream blocks'
/// `_block_dir` metadata, which links to external git repositories.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TaskCommitPayload {}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests;
