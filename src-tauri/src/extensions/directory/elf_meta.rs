/// .elf/ Dir Block 初始化模块
///
/// 在 create_file 时自动创建 `.elf/` Dir Block，提供系统级目录骨架。
/// entries 包含虚拟目录和 hook 模板文件（真实 code block）。
///
/// Hook 模板流程（类似 skill）：
/// 1. 模板定义在 `git_hooks::PRE_COMMIT_HOOK_CONTENT`
/// 2. `bootstrap_elf_meta` 根据模板创建 code block 并写入内容
/// 3. `commit_task` 注入该 block 的快照到外部 repo
///
/// 后续模块（F7 Skills, F1 Agent, F10 Session, F16 Task）负责填充实际内容。
use crate::models::Command;
use crate::state::AppState;
use crate::utils::git_hooks::PRE_COMMIT_HOOK_CONTENT;
use crate::utils::template_copy;
use crate::utils::time::now_utc;
use serde_json::json;
use std::path::Path;

/// `.elf/` Dir Block 的名称。
///
/// 系统初始化时创建的唯一系统级 Dir Block，用于存放 Agent 配置、Session 等元数据。
pub const ELF_META_BLOCK_NAME: &str = ".elf";

/// `.elf/` Dir Block 的描述。
pub const ELF_META_DESCRIPTION: &str = "Elfiee system metadata directory";

/// `.elf/` 目录骨架中的虚拟目录路径列表。
///
/// 每个路径会生成一个 `type: "directory"` 的 entry。
pub const ELF_DIR_PATHS: &[&str] = &[
    "agents/",
    "agents/elfiee-client/",
    "agents/elfiee-client/scripts/",
    "agents/elfiee-client/assets/",
    "agents/elfiee-client/references/",
    "session/",
    "git/",
    "git/hooks/",
];

/// 构造 `.elf/` Dir Block 的 entries JSON（仅目录骨架）。
///
/// 所有条目为虚拟目录，使用 `"dir-{uuid}"` 作为标识符，
/// 与 `directory_import.rs` 的虚拟目录格式一致。
///
/// 如需包含 hook 文件 block，使用 `build_elf_entries_with_hooks`。
pub fn build_elf_entries() -> serde_json::Value {
    build_elf_entries_with_hooks(&[])
}

/// 构造 `.elf/` Dir Block 的 entries JSON（含 hook 文件 block 引用）。
///
/// # Arguments
/// - `hook_blocks`: `[(entry_path, block_id)]` — hook 模板文件的路径和对应的真实 block ID
pub fn build_elf_entries_with_hooks(hook_blocks: &[(&str, &str)]) -> serde_json::Value {
    let now = now_utc();
    let mut entries = serde_json::Map::new();

    for path in ELF_DIR_PATHS {
        let dir_id = format!("dir-{}", uuid::Uuid::new_v4());
        entries.insert(
            path.to_string(),
            json!({
                "id": dir_id,
                "type": "directory",
                "source": "outline",
                "updated_at": now
            }),
        );
    }

    // Hook 模板文件引用真实 code block
    for (path, block_id) in hook_blocks {
        entries.insert(
            path.to_string(),
            json!({
                "id": *block_id,
                "type": "file",
                "source": "outline",
                "updated_at": now
            }),
        );
    }

    json!({
        "entries": entries,
        "source": "outline"
    })
}

/// 初始化 `.elf/` Dir Block。
///
/// 在 `create_file` 后调用，通过 process_command 发送命令序列：
/// 1. `core.create` — 创建 `.elf/` Dir Block
/// 2. `core.create` — 创建 `pre-commit` code block（hook 模板）
/// 3. `code.write` — 写入 hook 脚本内容
/// 4. `directory.write` — 写入目录骨架 entries（hook file 引用真实 block ID）
///
/// .elf/ block 权限通过协作者机制管理，不再授予 wildcard write。
/// 通过 Engine 处理命令保证 capability 检查、vector clock、快照一致性。
pub async fn bootstrap_elf_meta(file_id: &str, state: &AppState) -> Result<(), String> {
    let handle = state
        .engine_manager
        .get_engine(file_id)
        .ok_or_else(|| format!("File '{}' is not open", file_id))?;

    let editor_id = state
        .get_active_editor(file_id)
        .ok_or("No active editor for .elf/ initialization")?;

    // Step 1: core.create — 创建 .elf/ Dir Block
    let create_cmd = Command::new(
        editor_id.clone(),
        "core.create".to_string(),
        "".to_string(),
        json!({
            "name": ELF_META_BLOCK_NAME,
            "block_type": "directory",
            "source": "outline",
            "metadata": {
                "description": ELF_META_DESCRIPTION
            }
        }),
    );
    let events = handle.process_command(create_cmd).await?;
    let elf_block_id = events
        .first()
        .ok_or("No event returned from core.create for .elf/")?
        .entity
        .clone();

    // Step 2: core.create — 创建 pre-commit hook code block
    let create_hook_cmd = Command::new(
        editor_id.clone(),
        "core.create".to_string(),
        "".to_string(),
        json!({
            "name": "pre-commit",
            "block_type": "code",
            "source": "outline",
            "metadata": {
                "description": "Elfiee pre-commit hook: chain original hooks, then verify task.commit workflow"
            }
        }),
    );
    let hook_events = handle.process_command(create_hook_cmd).await?;
    let hook_block_id = hook_events
        .first()
        .ok_or("No event returned from core.create for pre-commit hook")?
        .entity
        .clone();

    // Step 3: code.write — 写入 hook 模板内容
    let write_hook_cmd = Command::new(
        editor_id.clone(),
        "code.write".to_string(),
        hook_block_id.clone(),
        json!({
            "content": PRE_COMMIT_HOOK_CONTENT,
        }),
    );
    handle.process_command(write_hook_cmd).await?;

    // Step 4: directory.write — 写入目录骨架 + hook 文件引用
    let entries = build_elf_entries_with_hooks(&[("git/hooks/pre-commit", &hook_block_id)]);
    let write_cmd = Command::new(
        editor_id.clone(),
        "directory.write".to_string(),
        elf_block_id.clone(),
        entries,
    );
    handle.process_command(write_cmd).await?;

    // Step 3: core.grant — 所有人可写
    let grant_cmd = Command::new(
        editor_id.clone(),
        "core.grant".to_string(),
        elf_block_id.clone(),
        json!({
            "target_editor": "*",
            "capability": "directory.write",
            "target_block": elf_block_id,
        }),
    );
    handle.process_command(grant_cmd).await?;

    // Step 4: Initialize elfiee-client skill templates into the block directory.
    //
    // After the .elf/ block is created, `inject_block_dir` has set `_block_dir`
    // in the block's contents. We read it back to get the physical path, then
    // write SKILL.md, mcp.json, and capabilities.md into the directory.
    if let Some(elf_block) = handle.get_block(elf_block_id.clone()).await {
        if let Some(block_dir) = elf_block
            .contents
            .get("_block_dir")
            .and_then(|v| v.as_str())
        {
            // elf_path is not needed for current SSE mode, pass empty string
            if let Err(e) = template_copy::init_elfiee_client(Path::new(block_dir), "") {
                // Log warning but don't fail bootstrap — templates can be retried
                eprintln!(
                    "Warning: Failed to initialize elfiee-client templates: {}",
                    e
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_elf_entries_dirs_only() {
        let entries_value = build_elf_entries();
        let obj = entries_value.as_object().unwrap();

        assert!(obj.contains_key("entries"));
        assert_eq!(obj.get("source").unwrap(), "outline");

        let entries = obj.get("entries").unwrap().as_object().unwrap();

        // 仅包含目录条目
        assert_eq!(entries.len(), ELF_DIR_PATHS.len());

        for path in ELF_DIR_PATHS {
            assert!(entries.contains_key(*path), "Missing dir path: {}", path);
            let entry_obj = entries.get(*path).unwrap().as_object().unwrap();
            assert_eq!(
                entry_obj.get("type").unwrap().as_str().unwrap(),
                "directory"
            );
            assert_eq!(
                entry_obj.get("source").unwrap().as_str().unwrap(),
                "outline"
            );
            assert!(entry_obj.contains_key("updated_at"));
        }
    }

    #[test]
    fn test_build_elf_entries_with_hooks() {
        let entries_value =
            build_elf_entries_with_hooks(&[("git/hooks/pre-commit", "block-abc-123")]);
        let entries = entries_value
            .as_object()
            .unwrap()
            .get("entries")
            .unwrap()
            .as_object()
            .unwrap();

        // 目录 + 1 个 hook 文件
        assert_eq!(entries.len(), ELF_DIR_PATHS.len() + 1);

        // hook 文件引用真实 block ID
        let hook_entry = entries
            .get("git/hooks/pre-commit")
            .unwrap()
            .as_object()
            .unwrap();
        assert_eq!(
            hook_entry.get("id").unwrap().as_str().unwrap(),
            "block-abc-123"
        );
        assert_eq!(hook_entry.get("type").unwrap().as_str().unwrap(), "file");
        assert_eq!(
            hook_entry.get("source").unwrap().as_str().unwrap(),
            "outline"
        );
    }

    #[test]
    fn test_build_elf_entries_unique_ids() {
        let entries_value = build_elf_entries();
        let entries = entries_value
            .as_object()
            .unwrap()
            .get("entries")
            .unwrap()
            .as_object()
            .unwrap();

        let ids: Vec<&str> = entries
            .values()
            .map(|v| v.as_object().unwrap().get("id").unwrap().as_str().unwrap())
            .collect();

        let mut unique_ids = ids.clone();
        unique_ids.sort();
        unique_ids.dedup();
        assert_eq!(
            ids.len(),
            unique_ids.len(),
            "All entry IDs should be unique"
        );
    }
}
