/// .elf/ Dir Block 初始化模块
///
/// 在 create_file 时自动创建 `.elf/` Dir Block，提供系统级目录骨架。
/// 目录骨架为虚拟目录（type: "directory"），同时通过 `template_copy` 将
/// elfiee-client Skill 模板文件（SKILL.md, mcp.json, capabilities.md）写入
/// 到 Block 物理目录中。
use crate::models::Command;
use crate::state::AppState;
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
const ELF_DIR_PATHS: &[&str] = &[
    "Agents/",
    "Agents/elfiee-client/",
    "Agents/elfiee-client/scripts/",
    "Agents/elfiee-client/assets/",
    "Agents/elfiee-client/references/",
    "Agents/session/",
    "git/",
];

/// 构造 `.elf/` Dir Block 的 entries JSON。
///
/// 所有条目为虚拟目录，使用 `"dir-{uuid}"` 作为标识符，
/// 与 `directory_import.rs` 的虚拟目录格式一致。
pub fn build_elf_entries() -> serde_json::Value {
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

    json!({
        "entries": entries,
        "source": "outline"
    })
}

/// 初始化 `.elf/` Dir Block。
///
/// 在 `create_file` 后调用，通过 process_command 发送命令序列：
/// 1. `core.create` — 创建 `.elf/` Dir Block
/// 2. `directory.write` — 写入目录骨架 entries
/// 3. `core.grant` — 授予所有人 (`"*"`) `directory.write` 权限
///
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
        "".to_string(), // core.create 不需要已有 block_id
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

    // Step 2: directory.write — 写入目录骨架 entries
    let write_cmd = Command::new(
        editor_id.clone(),
        "directory.write".to_string(),
        elf_block_id.clone(),
        build_elf_entries(),
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
    fn test_build_elf_entries_structure() {
        let entries_value = build_elf_entries();
        let obj = entries_value.as_object().unwrap();

        // 应包含 "entries" 和 "source" 字段
        assert!(obj.contains_key("entries"));
        assert_eq!(obj.get("source").unwrap(), "outline");

        let entries = obj.get("entries").unwrap().as_object().unwrap();

        // 验证所有预期路径存在
        for path in ELF_DIR_PATHS {
            assert!(
                entries.contains_key(*path),
                "Missing expected path: {}",
                path
            );
        }

        // 验证每个 entry 的结构
        for (path, entry) in entries.iter() {
            let entry_obj = entry.as_object().unwrap();
            assert!(
                entry_obj
                    .get("id")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .starts_with("dir-"),
                "Entry {} should have dir- prefixed id",
                path
            );
            assert_eq!(
                entry_obj.get("type").unwrap().as_str().unwrap(),
                "directory",
                "Entry {} should be type directory",
                path
            );
            assert_eq!(
                entry_obj.get("source").unwrap().as_str().unwrap(),
                "outline",
                "Entry {} should have source outline",
                path
            );
            assert!(
                entry_obj.contains_key("updated_at"),
                "Entry {} should have updated_at",
                path
            );
        }
    }

    #[test]
    fn test_build_elf_entries_count() {
        let entries_value = build_elf_entries();
        let entries = entries_value
            .as_object()
            .unwrap()
            .get("entries")
            .unwrap()
            .as_object()
            .unwrap();

        assert_eq!(entries.len(), ELF_DIR_PATHS.len());
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

        // 所有 id 应唯一
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
