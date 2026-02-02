/// Capability: task.write
///
/// Writes markdown content to a task block's contents.
/// Contents structure: `{ "markdown": "..." }` — same model as markdown blocks.
/// Title is stored in `block.name`, description in `metadata.description`.
///
/// Automatically updates metadata.updated_at timestamp.
use super::TaskWritePayload;
use crate::capabilities::core::{create_event, CapResult};
use crate::models::{Block, Command, Event};
use capability_macros::capability;

#[capability(id = "task.write", target = "task")]
fn handle_task_write(cmd: &Command, block: Option<&Block>) -> CapResult<Vec<Event>> {
    let block = block.ok_or("Block required for task.write")?;

    if block.block_type != "task" {
        return Err(format!("Expected task block, got '{}'", block.block_type));
    }

    let payload: TaskWritePayload = serde_json::from_value(cmd.payload.clone())
        .map_err(|e| format!("Invalid payload for task.write: {}", e))?;

    // 更新 contents：写入 markdown（与 markdown block 相同模型）
    let mut new_contents = if let Some(obj) = block.contents.as_object() {
        obj.clone()
    } else {
        serde_json::Map::new()
    };
    new_contents.insert("markdown".to_string(), serde_json::json!(payload.content));

    // 更新 metadata.updated_at
    let mut new_metadata = block.metadata.clone();
    new_metadata.touch();

    let event = create_event(
        block.block_id.clone(),
        "task.write",
        serde_json::json!({
            "contents": new_contents,
            "metadata": new_metadata.to_json()
        }),
        &cmd.editor_id,
        1, // Placeholder — engine actor updates with correct count
    );

    Ok(vec![event])
}
