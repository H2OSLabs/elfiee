/// .elf/ Dir Block 初始化模块
///
/// 在 create_file 时自动创建 `.elf/` Dir Block，提供系统级目录骨架。
/// entries 包含虚拟目录和模板文件 block 引用。
///
/// ## 统一模板系统：Template -> Block + Physical Files
///
/// 所有模板内容通过 event store 管理，确保变更有审计记录：
///
/// 1. **模板文件** (`templates/elf-meta/...`) — 编译时嵌入 (`include_str!`)
/// 2. **bootstrap** 为每个模板创建 block + 写入内容 → events 记录
/// 3. **bootstrap** 同时直接写入物理文件到 `_block_dir`（供 symlink 使用）
/// 4. **注入时** 读取 block 内容 → 写入外部 repo（如 git hooks）
///
/// Dogfooding 流程：
/// 1. 在 Elfiee 中编辑 block → events 记录
/// 2. 验证效果（block 内容可读取、可展示）
/// 3. 确认无误后 → 更新模板源文件 → 重新编译
/// 4. 下次 bootstrap 使用新模板
use crate::models::Command;
use crate::state::AppState;
use crate::utils::time::now_utc;
use serde_json::json;
use std::collections::BTreeSet;
use std::path::Path;

/// `.elf/` Dir Block 的名称。
///
/// 系统初始化时创建的唯一系统级 Dir Block，用于存放 Agent 配置、Session 等元数据。
pub const ELF_META_BLOCK_NAME: &str = ".elf";

/// `.elf/` Dir Block 的描述。
pub const ELF_META_DESCRIPTION: &str = "Elfiee system metadata directory";

/// 模板文件描述，编译时嵌入内容。
///
/// 每个 TemplateFile 在 bootstrap 时会：
/// 1. 创建对应类型的 block（event sourced, 审计）
/// 2. 写入模板内容到 block
/// 3. 直接写入物理文件到 `_block_dir`（供 symlink 使用）
pub struct TemplateFile {
    /// `.elf/` 内的 entry 路径（如 `"agents/elfiee-client/SKILL.md"`）
    pub path: &'static str,
    /// 编译时嵌入的模板内容 (`include_str!`)
    pub content: &'static str,
    /// block 类型（`"markdown"` 或 `"code"`）
    pub block_type: &'static str,
    /// block 名称
    pub name: &'static str,
    /// block 描述
    pub description: &'static str,
    /// 写入 capability（`"markdown.write"` 或 `"code.write"`）
    pub write_cap: &'static str,
}

/// 所有模板文件注册表。
///
/// 路径镜像 `.elf/` 内部结构，与 `templates/elf-meta/` 目录一一对应。
pub const TEMPLATE_FILES: &[TemplateFile] = &[
    TemplateFile {
        path: "agents/elfiee-client/SKILL.md",
        content: include_str!("../../../templates/elf-meta/agents/elfiee-client/SKILL.md"),
        block_type: "markdown",
        name: "SKILL.md",
        description: "Elfiee client skill definition for Claude Code",
        write_cap: "markdown.write",
    },
    TemplateFile {
        path: "agents/elfiee-client/mcp.json",
        content: include_str!("../../../templates/elf-meta/agents/elfiee-client/mcp.json"),
        block_type: "code",
        name: "mcp.json",
        description: "MCP server configuration for Elfiee client",
        write_cap: "code.write",
    },
    TemplateFile {
        path: "agents/elfiee-client/references/capabilities.md",
        content: include_str!(
            "../../../templates/elf-meta/agents/elfiee-client/references/capabilities.md"
        ),
        block_type: "markdown",
        name: "capabilities.md",
        description: "Elfiee capabilities reference document",
        write_cap: "markdown.write",
    },
    TemplateFile {
        path: "git/hooks/pre-commit",
        content: include_str!("../../../templates/elf-meta/git/hooks/pre-commit"),
        block_type: "code",
        name: "pre-commit",
        description:
            "Elfiee pre-commit hook: chain original hooks, then verify task.commit workflow",
        write_cap: "code.write",
    },
];

/// 额外的空目录（没有模板文件，不会自动从 TEMPLATE_FILES 推导出来）。
const EXTRA_DIRS: &[&str] = &[
    "session/",
    "agents/elfiee-client/scripts/",
    "agents/elfiee-client/assets/",
];

/// 从 TEMPLATE_FILES 的文件路径自动推导所有父目录路径。
///
/// 例如 `"agents/elfiee-client/SKILL.md"` 会推导出：
/// - `"agents/"`
/// - `"agents/elfiee-client/"`
///
/// 返回去重排序的目录路径列表（含 EXTRA_DIRS）。
pub fn derive_dir_paths() -> Vec<String> {
    let mut dirs = BTreeSet::new();

    for tmpl in TEMPLATE_FILES {
        let mut current = String::new();
        // 拆分路径，取除最后一段（文件名）之外的所有段作为目录
        let parts: Vec<&str> = tmpl.path.split('/').collect();
        for part in &parts[..parts.len() - 1] {
            current.push_str(part);
            current.push('/');
            dirs.insert(current.clone());
        }
    }

    for extra in EXTRA_DIRS {
        dirs.insert(extra.to_string());
    }

    dirs.into_iter().collect()
}

/// 构造 `.elf/` Dir Block 的 entries JSON（仅目录骨架）。
///
/// 目录路径从 TEMPLATE_FILES 自动推导 + EXTRA_DIRS。
/// 如需包含文件 block 引用，使用 `build_elf_entries_with_files`。
pub fn build_elf_entries() -> serde_json::Value {
    build_elf_entries_with_files(&[])
}

/// 构造 `.elf/` Dir Block 的 entries JSON（含文件 block 引用）。
///
/// # Arguments
/// - `file_blocks`: `[(entry_path, block_id)]` — 模板文件的 entry 路径和对应的 block ID
pub fn build_elf_entries_with_files(file_blocks: &[(&str, &str)]) -> serde_json::Value {
    let now = now_utc();
    let mut entries = serde_json::Map::new();

    let dir_paths = derive_dir_paths();
    for path in &dir_paths {
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

    // 模板文件引用真实 block
    for (path, block_id) in file_blocks {
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
///
/// 1. `core.create` — 创建 `.elf/` Dir Block
/// 2. 对每个 TemplateFile:
///    - `core.create` — 创建 block (markdown/code)
///    - `{type}.write` — 写入模板内容 → events 记录
/// 3. `directory.write` — entries = 自动推导目录 + 文件 block 引用
/// 4. 写入物理文件 — 从模板内容直接写入 `_block_dir`
///
/// 物理文件来源于模板（bootstrap 时写入），Block 用于 events 审计和编辑验证。
/// .elf/ block 权限通过协作者机制管理。
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

    // Step 2: 为每个 TemplateFile 创建 block + 写入内容
    let mut file_blocks: Vec<(&str, String)> = Vec::new();

    for tmpl in TEMPLATE_FILES {
        // core.create — 创建 block
        let create_block_cmd = Command::new(
            editor_id.clone(),
            "core.create".to_string(),
            "".to_string(),
            json!({
                "name": tmpl.name,
                "block_type": tmpl.block_type,
                "source": "outline",
                "metadata": {
                    "description": tmpl.description
                }
            }),
        );
        let block_events = handle.process_command(create_block_cmd).await?;
        let block_id = block_events
            .first()
            .ok_or_else(|| format!("No event returned from core.create for {}", tmpl.name))?
            .entity
            .clone();

        // {type}.write — 写入模板内容
        let write_cmd = Command::new(
            editor_id.clone(),
            tmpl.write_cap.to_string(),
            block_id.clone(),
            json!({
                "content": tmpl.content,
            }),
        );
        handle.process_command(write_cmd).await?;

        file_blocks.push((tmpl.path, block_id));
    }

    // Step 3: directory.write — 写入目录骨架 + 文件 block 引用
    let file_block_refs: Vec<(&str, &str)> = file_blocks
        .iter()
        .map(|(path, id)| (*path, id.as_str()))
        .collect();
    let entries = build_elf_entries_with_files(&file_block_refs);
    let write_cmd = Command::new(
        editor_id.clone(),
        "directory.write".to_string(),
        elf_block_id.clone(),
        entries,
    );
    handle.process_command(write_cmd).await?;

    // Step 4: 写入物理文件到 _block_dir（从模板内容直接写入，不从 block 读取）
    if let Some(elf_block) = handle.get_block(elf_block_id.clone()).await {
        if let Some(block_dir) = elf_block
            .contents
            .get("_block_dir")
            .and_then(|v| v.as_str())
        {
            let block_dir_path = Path::new(block_dir);

            for tmpl in TEMPLATE_FILES {
                let target = block_dir_path.join(tmpl.path);
                if let Some(parent) = target.parent() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        eprintln!(
                            "Warning: Failed to create directory {}: {}",
                            parent.display(),
                            e
                        );
                        continue;
                    }
                }
                if let Err(e) = std::fs::write(&target, tmpl.content) {
                    eprintln!(
                        "Warning: Failed to write template file {}: {}",
                        target.display(),
                        e
                    );
                }
            }

            // 创建额外的空目录
            for dir in EXTRA_DIRS {
                let dir_path = block_dir_path.join(dir);
                if let Err(e) = std::fs::create_dir_all(&dir_path) {
                    eprintln!(
                        "Warning: Failed to create extra directory {}: {}",
                        dir_path.display(),
                        e
                    );
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_dir_paths_contains_all_parent_dirs() {
        let dirs = derive_dir_paths();

        // 从 TEMPLATE_FILES 路径推导的目录
        assert!(dirs.contains(&"agents/".to_string()), "Missing agents/");
        assert!(
            dirs.contains(&"agents/elfiee-client/".to_string()),
            "Missing agents/elfiee-client/"
        );
        assert!(
            dirs.contains(&"agents/elfiee-client/references/".to_string()),
            "Missing agents/elfiee-client/references/"
        );
        assert!(dirs.contains(&"git/".to_string()), "Missing git/");
        assert!(
            dirs.contains(&"git/hooks/".to_string()),
            "Missing git/hooks/"
        );

        // EXTRA_DIRS
        assert!(dirs.contains(&"session/".to_string()), "Missing session/");
        assert!(
            dirs.contains(&"agents/elfiee-client/scripts/".to_string()),
            "Missing agents/elfiee-client/scripts/"
        );
        assert!(
            dirs.contains(&"agents/elfiee-client/assets/".to_string()),
            "Missing agents/elfiee-client/assets/"
        );
    }

    #[test]
    fn test_derive_dir_paths_no_duplicates() {
        let dirs = derive_dir_paths();
        let mut unique = dirs.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(
            dirs.len(),
            unique.len(),
            "derive_dir_paths should have no duplicates"
        );
    }

    #[test]
    fn test_derive_dir_paths_sorted() {
        let dirs = derive_dir_paths();
        let mut sorted = dirs.clone();
        sorted.sort();
        assert_eq!(dirs, sorted, "derive_dir_paths should be sorted (BTreeSet)");
    }

    #[test]
    fn test_template_files_count() {
        assert_eq!(TEMPLATE_FILES.len(), 4, "Should have 4 template files");
    }

    #[test]
    fn test_template_files_content_not_empty() {
        for tmpl in TEMPLATE_FILES {
            assert!(
                !tmpl.content.is_empty(),
                "Template content should not be empty: {}",
                tmpl.path
            );
        }
    }

    #[test]
    fn test_template_files_paths_unique() {
        let paths: Vec<&str> = TEMPLATE_FILES.iter().map(|t| t.path).collect();
        let mut unique = paths.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(
            paths.len(),
            unique.len(),
            "Template file paths should be unique"
        );
    }

    #[test]
    fn test_template_files_valid_block_types() {
        for tmpl in TEMPLATE_FILES {
            assert!(
                tmpl.block_type == "markdown" || tmpl.block_type == "code",
                "Invalid block_type '{}' for {}",
                tmpl.block_type,
                tmpl.path
            );
        }
    }

    #[test]
    fn test_template_files_write_cap_matches_block_type() {
        for tmpl in TEMPLATE_FILES {
            let expected_cap = format!("{}.write", tmpl.block_type);
            assert_eq!(
                tmpl.write_cap, expected_cap,
                "write_cap mismatch for {}: expected {}, got {}",
                tmpl.path, expected_cap, tmpl.write_cap
            );
        }
    }

    #[test]
    fn test_build_elf_entries_dirs_only() {
        let entries_value = build_elf_entries();
        let obj = entries_value.as_object().unwrap();

        assert!(obj.contains_key("entries"));
        assert_eq!(obj.get("source").unwrap(), "outline");

        let entries = obj.get("entries").unwrap().as_object().unwrap();
        let dir_paths = derive_dir_paths();

        // 仅包含目录条目
        assert_eq!(entries.len(), dir_paths.len());

        for path in &dir_paths {
            assert!(
                entries.contains_key(path.as_str()),
                "Missing dir path: {}",
                path
            );
            let entry_obj = entries.get(path.as_str()).unwrap().as_object().unwrap();
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
    fn test_build_elf_entries_with_files() {
        let file_blocks: Vec<(&str, &str)> = TEMPLATE_FILES
            .iter()
            .enumerate()
            .map(|(i, t)| {
                (
                    t.path,
                    if i == 0 {
                        "block-aaa"
                    } else if i == 1 {
                        "block-bbb"
                    } else if i == 2 {
                        "block-ccc"
                    } else {
                        "block-ddd"
                    },
                )
            })
            .collect();

        let entries_value = build_elf_entries_with_files(&file_blocks);
        let entries = entries_value
            .as_object()
            .unwrap()
            .get("entries")
            .unwrap()
            .as_object()
            .unwrap();

        let dir_paths = derive_dir_paths();

        // 目录 + 4 个文件
        assert_eq!(entries.len(), dir_paths.len() + TEMPLATE_FILES.len());

        // 验证文件引用
        for (path, block_id) in &file_blocks {
            let entry = entries.get(*path).unwrap().as_object().unwrap();
            assert_eq!(entry.get("id").unwrap().as_str().unwrap(), *block_id);
            assert_eq!(entry.get("type").unwrap().as_str().unwrap(), "file");
            assert_eq!(entry.get("source").unwrap().as_str().unwrap(), "outline");
        }
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

    #[test]
    fn test_pre_commit_hook_content_accessible() {
        // 验证 pre-commit hook 模板可以通过 TEMPLATE_FILES 访问
        let hook = TEMPLATE_FILES
            .iter()
            .find(|t| t.path == "git/hooks/pre-commit");
        assert!(
            hook.is_some(),
            "pre-commit hook should be in TEMPLATE_FILES"
        );
        let hook = hook.unwrap();
        assert!(
            hook.content.contains("ELFIEE_TASK_COMMIT"),
            "pre-commit hook should contain ELFIEE_TASK_COMMIT check"
        );
        assert_eq!(hook.block_type, "code");
        assert_eq!(hook.write_cap, "code.write");
    }

    #[test]
    fn test_skill_md_content_valid() {
        let skill = TEMPLATE_FILES
            .iter()
            .find(|t| t.path == "agents/elfiee-client/SKILL.md");
        assert!(skill.is_some(), "SKILL.md should be in TEMPLATE_FILES");
        let skill = skill.unwrap();
        assert!(skill.content.contains("name: elfiee-client"));
        assert_eq!(skill.block_type, "markdown");
    }

    #[test]
    fn test_mcp_json_content_valid() {
        let mcp = TEMPLATE_FILES
            .iter()
            .find(|t| t.path == "agents/elfiee-client/mcp.json");
        assert!(mcp.is_some(), "mcp.json should be in TEMPLATE_FILES");
        let mcp = mcp.unwrap();
        let json: serde_json::Value =
            serde_json::from_str(mcp.content).expect("mcp.json template should be valid JSON");
        assert!(json.get("mcpServers").is_some());
        assert_eq!(mcp.block_type, "code");
    }

    #[test]
    fn test_capabilities_md_content_valid() {
        let cap = TEMPLATE_FILES
            .iter()
            .find(|t| t.path == "agents/elfiee-client/references/capabilities.md");
        assert!(cap.is_some(), "capabilities.md should be in TEMPLATE_FILES");
        let cap = cap.unwrap();
        assert!(cap.content.contains("core.create"));
        assert!(cap.content.contains("markdown.write"));
        assert_eq!(cap.block_type, "markdown");
    }
}
