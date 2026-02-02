/// Capability: task.commit
///
/// Generates an audit event recording a task commit intent.
/// The actual I/O (file export + git operations) is performed by the
/// Tauri command layer (`commands/task.rs`), following the Split Pattern
/// used by `directory.export`.
///
/// ## Validation
/// - Block must be a task block
/// - Block must have downstream blocks via "implement" relation
///
/// ## No status check
/// Multiple commits are allowed — event history naturally records each one.
///
/// ## Auto-discover
/// No target_path parameter — the Tauri command layer auto-discovers
/// linked repos from downstream blocks' `_block_dir` metadata.
use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, Event, RELATION_IMPLEMENT};
use capability_macros::capability;

#[capability(id = "task.commit", target = "task")]
fn handle_task_commit(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for task.commit")?;

    if block.block_type != "task" {
        return Err(format!("Expected task block, got '{}'", block.block_type));
    }

    // TaskCommitPayload is empty — no deserialization needed

    // 查询 implement 下游（必须有关联 block 才能 commit）
    let downstream_ids = block
        .children
        .get(RELATION_IMPLEMENT)
        .cloned()
        .unwrap_or_default();

    if downstream_ids.is_empty() {
        return Err(
            "No downstream blocks linked via 'implement' relation. Link code/markdown blocks to this task before committing."
                .to_string(),
        );
    }

    // 审计 event（不修改 contents，无状态变更）
    let event = create_event(
        block.block_id.clone(),
        "task.commit",
        serde_json::json!({
            "downstream_block_ids": downstream_ids,
        }),
        &cmd.editor_id,
        1, // Placeholder — engine actor updates with correct count
    );

    Ok(vec![event])
}
