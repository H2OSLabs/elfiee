/// 集成测试：统一模板系统 — Template -> Block + Physical Files
///
/// 验证：
/// - bootstrap_elf_meta 后所有模板文件写入到 block 物理目录（小写路径）
/// - SKILL.md 内容完整（frontmatter、工具引用、关键约束）
/// - mcp.json 是合法 JSON 且包含 elfiee server 配置
/// - capabilities.md 包含所有已注册的 capability
/// - 目录结构完整（scripts/, assets/, references/, session/）
/// - 物理文件路径使用小写 "agents/"（非 "Agents/"）
use elfiee_lib::elf::ElfArchive;
use elfiee_lib::engine::spawn_engine;
use elfiee_lib::extensions::directory::elf_meta::{build_elf_entries_with_files, TEMPLATE_FILES};
use elfiee_lib::models::Command;
use std::fs;
use tempfile::NamedTempFile;

// ============================================================================
// 辅助函数
// ============================================================================

/// 创建 engine（基于临时 .elf 文件，物理目录可用）
async fn setup_engine() -> (
    ElfArchive,
    elfiee_lib::engine::EngineHandle,
    std::path::PathBuf,
) {
    let temp_elf = NamedTempFile::new().unwrap();
    let elf_path = temp_elf.path().to_path_buf();

    let archive = ElfArchive::new().await.unwrap();
    archive.save(&elf_path).unwrap();

    let archive = ElfArchive::open(&elf_path).unwrap();
    let event_pool = archive.event_pool().await.unwrap();
    let handle = spawn_engine("test_template".to_string(), event_pool)
        .await
        .unwrap();

    // 创建 system editor
    let cmd = Command::new(
        "system".to_string(),
        "editor.create".to_string(),
        "system".to_string(),
        serde_json::json!({
            "editor_id": "system",
            "name": "System"
        }),
    );
    handle.process_command(cmd).await.unwrap();

    (archive, handle, elf_path)
}

/// 模拟 bootstrap_elf_meta 完整流程（统一模板系统）。
///
/// 包含 Step 4: 直接写入物理文件到 _block_dir。
/// 返回 elf_block_id
async fn bootstrap_elf_meta_with_templates(
    handle: &elfiee_lib::engine::EngineHandle,
    editor_id: &str,
) -> String {
    // Step 1: core.create — .elf/ Dir Block
    let create_cmd = Command::new(
        editor_id.to_string(),
        "core.create".to_string(),
        "".to_string(),
        serde_json::json!({
            "name": ".elf",
            "block_type": "directory",
            "source": "outline",
            "metadata": {
                "description": "Elfiee system metadata directory"
            }
        }),
    );
    let events = handle.process_command(create_cmd).await.unwrap();
    let elf_block_id = events[0].entity.clone();

    // Step 2: 为每个 TemplateFile 创建 block + 写入内容
    let mut file_blocks: Vec<(&str, String)> = Vec::new();

    for tmpl in TEMPLATE_FILES {
        let create_block_cmd = Command::new(
            editor_id.to_string(),
            "core.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "name": tmpl.name,
                "block_type": tmpl.block_type,
                "source": "outline",
                "metadata": {
                    "description": tmpl.description
                }
            }),
        );
        let block_events = handle.process_command(create_block_cmd).await.unwrap();
        let block_id = block_events[0].entity.clone();

        let write_cmd = Command::new(
            editor_id.to_string(),
            tmpl.write_cap.to_string(),
            block_id.clone(),
            serde_json::json!({
                "content": tmpl.content,
            }),
        );
        handle.process_command(write_cmd).await.unwrap();

        file_blocks.push((tmpl.path, block_id));
    }

    // Step 3: directory.write
    let file_block_refs: Vec<(&str, &str)> = file_blocks
        .iter()
        .map(|(path, id)| (*path, id.as_str()))
        .collect();
    let entries = build_elf_entries_with_files(&file_block_refs);
    let write_cmd = Command::new(
        editor_id.to_string(),
        "directory.write".to_string(),
        elf_block_id.clone(),
        entries,
    );
    handle.process_command(write_cmd).await.unwrap();

    // Step 4: 写入物理文件到 _block_dir（统一模板系统）
    if let Some(elf_block) = handle.get_block(elf_block_id.clone()).await {
        if let Some(block_dir) = elf_block
            .contents
            .get("_block_dir")
            .and_then(|v| v.as_str())
        {
            let block_dir_path = std::path::Path::new(block_dir);

            for tmpl in TEMPLATE_FILES {
                let target = block_dir_path.join(tmpl.path);
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent).unwrap();
                }
                std::fs::write(&target, tmpl.content).unwrap();
            }

            // 创建额外的空目录
            let extra_dirs = [
                "session/",
                "agents/elfiee-client/scripts/",
                "agents/elfiee-client/assets/",
            ];
            for dir in &extra_dirs {
                std::fs::create_dir_all(block_dir_path.join(dir)).unwrap();
            }
        } else {
            panic!("_block_dir not found in .elf/ block contents");
        }
    } else {
        panic!(".elf/ block not found after creation");
    }

    elf_block_id
}

/// 获取 .elf/ block 的物理目录路径
async fn get_elf_block_dir(
    handle: &elfiee_lib::engine::EngineHandle,
    elf_block_id: &str,
) -> String {
    let block = handle
        .get_block(elf_block_id.to_string())
        .await
        .expect(".elf/ block should exist");
    block
        .contents
        .get("_block_dir")
        .and_then(|v| v.as_str())
        .expect("_block_dir should be set")
        .to_string()
}

// ============================================================================
// 测试：目录结构（小写路径）
// ============================================================================

#[tokio::test]
async fn test_template_directory_structure() {
    let (_archive, handle, _elf_path) = setup_engine().await;
    let elf_id = bootstrap_elf_meta_with_templates(&handle, "system").await;
    let block_dir = get_elf_block_dir(&handle, &elf_id).await;
    let base = std::path::Path::new(&block_dir)
        .join("agents")
        .join("elfiee-client");

    assert!(base.exists(), "agents/elfiee-client/ should exist");
    assert!(base.join("scripts").is_dir(), "scripts/ should exist");
    assert!(base.join("assets").is_dir(), "assets/ should exist");
    assert!(base.join("references").is_dir(), "references/ should exist");
    assert!(
        std::path::Path::new(&block_dir).join("session").is_dir(),
        "session/ should exist"
    );

    handle.shutdown().await;
}

#[tokio::test]
async fn test_template_empty_dirs_are_empty() {
    let (_archive, handle, _elf_path) = setup_engine().await;
    let elf_id = bootstrap_elf_meta_with_templates(&handle, "system").await;
    let block_dir = get_elf_block_dir(&handle, &elf_id).await;
    let base = std::path::Path::new(&block_dir)
        .join("agents")
        .join("elfiee-client");

    assert_eq!(
        fs::read_dir(base.join("scripts")).unwrap().count(),
        0,
        "scripts/ should be empty"
    );
    assert_eq!(
        fs::read_dir(base.join("assets")).unwrap().count(),
        0,
        "assets/ should be empty"
    );

    handle.shutdown().await;
}

// ============================================================================
// 测试：SKILL.md
// ============================================================================

#[tokio::test]
async fn test_skill_md_exists_and_nonempty() {
    let (_archive, handle, _elf_path) = setup_engine().await;
    let elf_id = bootstrap_elf_meta_with_templates(&handle, "system").await;
    let block_dir = get_elf_block_dir(&handle, &elf_id).await;

    let skill_path = std::path::Path::new(&block_dir)
        .join("agents")
        .join("elfiee-client")
        .join("SKILL.md");

    assert!(skill_path.exists(), "SKILL.md should exist");

    let content = fs::read_to_string(&skill_path).unwrap();
    assert!(
        content.len() > 500,
        "SKILL.md should have substantial content (got {} bytes)",
        content.len()
    );

    handle.shutdown().await;
}

#[tokio::test]
async fn test_skill_md_has_yaml_frontmatter() {
    let (_archive, handle, _elf_path) = setup_engine().await;
    let elf_id = bootstrap_elf_meta_with_templates(&handle, "system").await;
    let block_dir = get_elf_block_dir(&handle, &elf_id).await;

    let skill_path = std::path::Path::new(&block_dir)
        .join("agents")
        .join("elfiee-client")
        .join("SKILL.md");
    let content = fs::read_to_string(&skill_path).unwrap();

    assert!(
        content.starts_with("---"),
        "SKILL.md should start with YAML frontmatter"
    );
    assert!(
        content.contains("name: elfiee-client"),
        "SKILL.md should contain name: elfiee-client"
    );
    assert!(
        content.contains("description:"),
        "SKILL.md should contain description field"
    );

    handle.shutdown().await;
}

#[tokio::test]
async fn test_skill_md_has_critical_constraint() {
    let (_archive, handle, _elf_path) = setup_engine().await;
    let elf_id = bootstrap_elf_meta_with_templates(&handle, "system").await;
    let block_dir = get_elf_block_dir(&handle, &elf_id).await;

    let skill_path = std::path::Path::new(&block_dir)
        .join("agents")
        .join("elfiee-client")
        .join("SKILL.md");
    let content = fs::read_to_string(&skill_path).unwrap();

    assert!(
        content.contains("NEVER do these"),
        "SKILL.md should contain the critical constraint about prohibited actions"
    );

    handle.shutdown().await;
}

#[tokio::test]
async fn test_skill_md_references_all_mcp_tools() {
    let (_archive, handle, _elf_path) = setup_engine().await;
    let elf_id = bootstrap_elf_meta_with_templates(&handle, "system").await;
    let block_dir = get_elf_block_dir(&handle, &elf_id).await;

    let skill_path = std::path::Path::new(&block_dir)
        .join("agents")
        .join("elfiee-client")
        .join("SKILL.md");
    let content = fs::read_to_string(&skill_path).unwrap();

    // All 33 MCP tools from mcp/server.rs
    let tools = [
        "elfiee_file_list",
        "elfiee_block_list",
        "elfiee_block_get",
        "elfiee_block_create",
        "elfiee_block_delete",
        "elfiee_block_rename",
        "elfiee_block_change_type",
        "elfiee_block_update_metadata",
        "elfiee_block_link",
        "elfiee_block_unlink",
        "elfiee_markdown_read",
        "elfiee_markdown_write",
        "elfiee_code_read",
        "elfiee_code_write",
        "elfiee_directory_create",
        "elfiee_directory_delete",
        "elfiee_directory_rename",
        "elfiee_directory_write",
        "elfiee_directory_import",
        "elfiee_directory_export",
        "elfiee_terminal_init",
        "elfiee_terminal_execute",
        "elfiee_terminal_save",
        "elfiee_terminal_close",
        "elfiee_task_create",
        "elfiee_task_write",
        "elfiee_task_commit",
        "elfiee_task_link",
        "elfiee_grant",
        "elfiee_revoke",
        "elfiee_editor_create",
        "elfiee_editor_delete",
        "elfiee_exec",
    ];

    for tool in &tools {
        assert!(
            content.contains(tool),
            "SKILL.md should reference MCP tool: {}",
            tool
        );
    }

    handle.shutdown().await;
}

#[tokio::test]
async fn test_skill_md_has_resource_uris() {
    let (_archive, handle, _elf_path) = setup_engine().await;
    let elf_id = bootstrap_elf_meta_with_templates(&handle, "system").await;
    let block_dir = get_elf_block_dir(&handle, &elf_id).await;

    let skill_path = std::path::Path::new(&block_dir)
        .join("agents")
        .join("elfiee-client")
        .join("SKILL.md");
    let content = fs::read_to_string(&skill_path).unwrap();

    assert!(
        content.contains("elfiee://files"),
        "SKILL.md should reference elfiee://files resource"
    );
    assert!(
        content.contains("elfiee://{project}/blocks"),
        "SKILL.md should reference blocks resource"
    );
    assert!(
        content.contains("elfiee://{project}/grants"),
        "SKILL.md should reference grants resource"
    );
    assert!(
        content.contains("elfiee://{project}/events"),
        "SKILL.md should reference events resource"
    );

    handle.shutdown().await;
}

// mcp.json template tests removed: template was deleted (Fix 5).
// Actual MCP config (.mcp.json) is dynamically generated by perform_enable_io.

// ============================================================================
// 测试：capabilities.md
// ============================================================================

#[tokio::test]
async fn test_capabilities_md_exists_and_nonempty() {
    let (_archive, handle, _elf_path) = setup_engine().await;
    let elf_id = bootstrap_elf_meta_with_templates(&handle, "system").await;
    let block_dir = get_elf_block_dir(&handle, &elf_id).await;

    let cap_path = std::path::Path::new(&block_dir)
        .join("agents")
        .join("elfiee-client")
        .join("references")
        .join("capabilities.md");

    assert!(cap_path.exists(), "capabilities.md should exist");

    let content = fs::read_to_string(&cap_path).unwrap();
    assert!(
        content.len() > 200,
        "capabilities.md should have substantial content"
    );

    handle.shutdown().await;
}

#[tokio::test]
async fn test_capabilities_md_references_registered_capabilities() {
    let (_archive, handle, _elf_path) = setup_engine().await;
    let elf_id = bootstrap_elf_meta_with_templates(&handle, "system").await;
    let block_dir = get_elf_block_dir(&handle, &elf_id).await;

    let cap_path = std::path::Path::new(&block_dir)
        .join("agents")
        .join("elfiee-client")
        .join("references")
        .join("capabilities.md");
    let content = fs::read_to_string(&cap_path).unwrap();

    // Core capabilities
    let caps = [
        "core.create",
        "core.read",
        "core.link",
        "core.unlink",
        "core.delete",
        "core.grant",
        "core.revoke",
        // Content
        "markdown.write",
        "markdown.read",
        "code.write",
        "code.read",
        // Directory
        "directory.create",
        "directory.delete",
        "directory.rename",
        "directory.write",
        "directory.import",
        "directory.export",
        // Terminal
        "terminal.init",
        "terminal.execute",
        "terminal.save",
        "terminal.close",
        // Agent
        "agent.create",
        "agent.enable",
        "agent.disable",
    ];

    for cap in &caps {
        assert!(
            content.contains(cap),
            "capabilities.md should reference: {}",
            cap
        );
    }

    handle.shutdown().await;
}

// ============================================================================
// 测试：幂等性（重复写入物理文件）
// ============================================================================

#[tokio::test]
async fn test_template_init_idempotent() {
    let (_archive, handle, _elf_path) = setup_engine().await;
    let elf_id = bootstrap_elf_meta_with_templates(&handle, "system").await;
    let block_dir_str = get_elf_block_dir(&handle, &elf_id).await;
    let block_dir = std::path::Path::new(&block_dir_str);

    // 重复写入模板文件（模拟幂等）
    for tmpl in TEMPLATE_FILES {
        let target = block_dir.join(tmpl.path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&target, tmpl.content).unwrap();
    }

    // Files should still be valid
    let skill_path = block_dir
        .join("agents")
        .join("elfiee-client")
        .join("SKILL.md");
    let content = fs::read_to_string(&skill_path).unwrap();
    assert!(
        content.contains("name: elfiee-client"),
        "SKILL.md should remain valid after re-init"
    );

    handle.shutdown().await;
}
