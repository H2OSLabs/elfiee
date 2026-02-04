/// 集成测试：.elf/ Dir Block 初始化
///
/// 验证：
/// - create_file 流程中 .elf/ Dir Block 自动创建
/// - entries 包含所有预期目录路径和模板文件 block 引用（4 个文件 block）
/// - 权限通过协作者机制管理（无 wildcard grant）
/// - 所有模板 block 包含正确内容
use elfiee_lib::engine::{spawn_engine, EventStore};
use elfiee_lib::extensions::directory::elf_meta::{
    build_elf_entries_with_files, derive_dir_paths, TEMPLATE_FILES,
};
use elfiee_lib::models::Command;

/// 辅助函数：创建内存 engine + 注册 editor
async fn setup_engine() -> elfiee_lib::engine::EngineHandle {
    let event_pool = EventStore::create(":memory:").await.unwrap();
    spawn_engine("test_elf_meta".to_string(), event_pool)
        .await
        .unwrap()
}

/// 辅助函数：创建 editor
async fn create_editor(handle: &elfiee_lib::engine::EngineHandle, editor_id: &str) -> String {
    let cmd = Command::new(
        editor_id.to_string(),
        "editor.create".to_string(),
        "".to_string(),
        serde_json::json!({
            "editor_id": editor_id,
            "name": editor_id,
            "editor_type": "Human"
        }),
    );
    let events = handle.process_command(cmd).await.unwrap();
    events[0].entity.clone()
}

/// 辅助函数：模拟 bootstrap_elf_meta 的完整流程（统一模板系统）。
///
/// 返回 (elf_block_id, Vec<(path, block_id)>)
async fn bootstrap_elf_meta(
    handle: &elfiee_lib::engine::EngineHandle,
    editor_id: &str,
) -> (String, Vec<(String, String)>) {
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
    let mut file_blocks: Vec<(String, String)> = Vec::new();

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

        file_blocks.push((tmpl.path.to_string(), block_id));
    }

    // Step 3: directory.write — entries 含所有文件 block 引用
    let file_block_refs: Vec<(&str, &str)> = file_blocks
        .iter()
        .map(|(path, id)| (path.as_str(), id.as_str()))
        .collect();
    let entries = build_elf_entries_with_files(&file_block_refs);
    let write_cmd = Command::new(
        editor_id.to_string(),
        "directory.write".to_string(),
        elf_block_id.clone(),
        entries,
    );
    handle.process_command(write_cmd).await.unwrap();

    (elf_block_id, file_blocks)
}

// ============================================================================
// 测试用例
// ============================================================================

#[tokio::test]
async fn test_elf_block_created() {
    let handle = setup_engine().await;
    create_editor(&handle, "system").await;
    let (elf_id, _) = bootstrap_elf_meta(&handle, "system").await;

    let block = handle.get_block(elf_id.clone()).await;
    assert!(block.is_some(), ".elf/ block should exist after bootstrap");

    let block = block.unwrap();
    assert_eq!(block.name, ".elf");
    assert_eq!(block.block_type, "directory");
    assert_eq!(block.owner, "system");
}

#[tokio::test]
async fn test_elf_block_entries_structure() {
    let handle = setup_engine().await;
    create_editor(&handle, "system").await;
    let (elf_id, file_blocks) = bootstrap_elf_meta(&handle, "system").await;

    let block = handle.get_block(elf_id).await.unwrap();
    let contents = block.contents.as_object().unwrap();
    let entries = contents.get("entries").unwrap().as_object().unwrap();

    let dir_paths = derive_dir_paths();

    // 验证所有自动推导的目录路径
    for path in &dir_paths {
        assert!(
            entries.contains_key(path.as_str()),
            "Missing dir entry: {}",
            path
        );
        let entry = entries.get(path.as_str()).unwrap().as_object().unwrap();
        assert_eq!(entry.get("type").unwrap().as_str().unwrap(), "directory");
        assert_eq!(entry.get("source").unwrap().as_str().unwrap(), "outline");
    }

    // 验证所有模板文件 block 引用
    for (path, block_id) in &file_blocks {
        let entry = entries
            .get(path.as_str())
            .unwrap_or_else(|| panic!("Missing file entry: {}", path))
            .as_object()
            .unwrap();
        assert_eq!(entry.get("type").unwrap().as_str().unwrap(), "file");
        assert_eq!(entry.get("id").unwrap().as_str().unwrap(), block_id);
        assert_eq!(entry.get("source").unwrap().as_str().unwrap(), "outline");
    }

    // 总数 = 目录数 + 文件数（4 个模板文件）
    assert_eq!(
        entries.len(),
        dir_paths.len() + TEMPLATE_FILES.len(),
        "entries should have {} dirs + {} files",
        dir_paths.len(),
        TEMPLATE_FILES.len()
    );
}

#[tokio::test]
async fn test_all_template_blocks_have_content() {
    let handle = setup_engine().await;
    create_editor(&handle, "system").await;
    let (_, file_blocks) = bootstrap_elf_meta(&handle, "system").await;

    assert_eq!(
        file_blocks.len(),
        TEMPLATE_FILES.len(),
        "Should have {} template blocks",
        TEMPLATE_FILES.len()
    );

    for (i, (path, block_id)) in file_blocks.iter().enumerate() {
        let tmpl = &TEMPLATE_FILES[i];
        let block = handle
            .get_block(block_id.clone())
            .await
            .unwrap_or_else(|| panic!("Block not found for template: {}", path));

        assert_eq!(block.name, tmpl.name, "Block name mismatch for {}", path);
        assert_eq!(
            block.block_type, tmpl.block_type,
            "Block type mismatch for {}",
            path
        );

        // 内容存储 key: markdown block → "markdown", code block → "text"
        let content_key = if tmpl.block_type == "markdown" {
            "markdown"
        } else {
            "text"
        };
        let content = block
            .contents
            .get(content_key)
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            !content.is_empty(),
            "Block content should not be empty for {} (key: {})",
            path,
            content_key
        );
    }
}

#[tokio::test]
async fn test_hook_block_has_template_content() {
    let handle = setup_engine().await;
    create_editor(&handle, "system").await;
    let (_, file_blocks) = bootstrap_elf_meta(&handle, "system").await;

    // 找到 pre-commit hook block
    let hook_entry = file_blocks
        .iter()
        .find(|(path, _)| path == "git/hooks/pre-commit")
        .expect("pre-commit hook should be in file_blocks");

    let hook_block = handle
        .get_block(hook_entry.1.clone())
        .await
        .expect("pre-commit hook block should exist");
    assert_eq!(hook_block.name, "pre-commit");
    assert_eq!(hook_block.block_type, "code");

    let content = hook_block
        .contents
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        content.contains("ELFIEE_TASK_COMMIT"),
        "Hook block should contain the pre-commit template"
    );
}

#[tokio::test]
async fn test_skill_block_has_template_content() {
    let handle = setup_engine().await;
    create_editor(&handle, "system").await;
    let (_, file_blocks) = bootstrap_elf_meta(&handle, "system").await;

    let skill_entry = file_blocks
        .iter()
        .find(|(path, _)| path == "agents/elfiee-client/SKILL.md")
        .expect("SKILL.md should be in file_blocks");

    let skill_block = handle
        .get_block(skill_entry.1.clone())
        .await
        .expect("SKILL.md block should exist");
    assert_eq!(skill_block.name, "SKILL.md");
    assert_eq!(skill_block.block_type, "markdown");

    // markdown.write 存储在 "markdown" key 下
    let content = skill_block
        .contents
        .get("markdown")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        content.contains("name: elfiee-client"),
        "SKILL block should contain frontmatter"
    );
}

#[tokio::test]
async fn test_elf_block_source_outline() {
    let handle = setup_engine().await;
    create_editor(&handle, "system").await;
    let (elf_id, _) = bootstrap_elf_meta(&handle, "system").await;

    let block = handle.get_block(elf_id).await.unwrap();
    let contents = block.contents.as_object().unwrap();
    assert_eq!(
        contents.get("source").unwrap().as_str().unwrap(),
        "outline",
        ".elf/ block source should be outline (internal)"
    );
}

#[tokio::test]
async fn test_elf_block_no_write_without_grant() {
    let handle = setup_engine().await;
    create_editor(&handle, "system").await;

    // 创建 .elf/ block 但 不 grant
    let create_cmd = Command::new(
        "system".to_string(),
        "core.create".to_string(),
        "".to_string(),
        serde_json::json!({
            "name": ".elf",
            "block_type": "directory",
            "source": "outline"
        }),
    );
    let events = handle.process_command(create_cmd).await.unwrap();
    let elf_id = events[0].entity.clone();

    // 写入 entries（system 是 owner，可以写）
    let entries = elfiee_lib::extensions::directory::elf_meta::build_elf_entries();
    let write_cmd = Command::new(
        "system".to_string(),
        "directory.write".to_string(),
        elf_id.clone(),
        entries,
    );
    handle.process_command(write_cmd).await.unwrap();

    // bob 没有 grant，不能写
    create_editor(&handle, "bob").await;
    let write_cmd = Command::new(
        "bob".to_string(),
        "directory.write".to_string(),
        elf_id.clone(),
        serde_json::json!({
            "entries": {},
            "source": "outline"
        }),
    );

    let result = handle.process_command(write_cmd).await;
    assert!(
        result.is_err(),
        "Non-owner without grant should not be able to write"
    );
}

#[tokio::test]
async fn test_elf_block_metadata() {
    let handle = setup_engine().await;
    create_editor(&handle, "system").await;
    let (elf_id, _) = bootstrap_elf_meta(&handle, "system").await;

    let block = handle.get_block(elf_id).await.unwrap();

    // metadata 应包含 description
    let desc = block.metadata.description.as_deref().unwrap_or("");
    assert_eq!(desc, "Elfiee system metadata directory");
}
