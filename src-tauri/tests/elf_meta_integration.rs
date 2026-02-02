/// 集成测试：.elf/ Dir Block 初始化 (I10-01)
///
/// 验证：
/// - create_file 流程中 .elf/ Dir Block 自动创建
/// - entries 包含所有预期目录路径和 hook 模板文件（真实 code block）
/// - 权限通过协作者机制管理（无 wildcard grant）
/// - pre-commit hook block 包含模板内容
use elfiee_lib::engine::{spawn_engine, EventStore};
use elfiee_lib::extensions::directory::elf_meta::{build_elf_entries_with_hooks, ELF_DIR_PATHS};
use elfiee_lib::models::Command;
use elfiee_lib::utils::git_hooks::PRE_COMMIT_HOOK_CONTENT;

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

/// 辅助函数：模拟 bootstrap_elf_meta 的完整流程。
///
/// 返回 (elf_block_id, hook_block_id)
async fn bootstrap_elf_meta(
    handle: &elfiee_lib::engine::EngineHandle,
    editor_id: &str,
) -> (String, String) {
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

    // Step 2: core.create — pre-commit code block
    let create_hook_cmd = Command::new(
        editor_id.to_string(),
        "core.create".to_string(),
        "".to_string(),
        serde_json::json!({
            "name": "pre-commit",
            "block_type": "code",
            "source": "outline",
            "metadata": {
                "description": "Elfiee pre-commit hook: chain original hooks, then verify task.commit workflow"
            }
        }),
    );
    let hook_events = handle.process_command(create_hook_cmd).await.unwrap();
    let hook_block_id = hook_events[0].entity.clone();

    // Step 3: code.write — 写入 hook 模板内容
    let write_hook_cmd = Command::new(
        editor_id.to_string(),
        "code.write".to_string(),
        hook_block_id.clone(),
        serde_json::json!({
            "content": PRE_COMMIT_HOOK_CONTENT,
        }),
    );
    handle.process_command(write_hook_cmd).await.unwrap();

    // Step 4: directory.write — entries 含 hook block 引用
    let entries = build_elf_entries_with_hooks(&[("git/hooks/pre-commit", &hook_block_id)]);
    let write_cmd = Command::new(
        editor_id.to_string(),
        "directory.write".to_string(),
        elf_block_id.clone(),
        entries,
    );
    handle.process_command(write_cmd).await.unwrap();

    (elf_block_id, hook_block_id)
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
    let (elf_id, _) = bootstrap_elf_meta(&handle, "system").await;

    let block = handle.get_block(elf_id).await.unwrap();
    let contents = block.contents.as_object().unwrap();
    let entries = contents.get("entries").unwrap().as_object().unwrap();

    // 验证所有预期目录路径
    for path in ELF_DIR_PATHS {
        assert!(entries.contains_key(*path), "Missing dir entry: {}", path);
        let entry = entries.get(*path).unwrap().as_object().unwrap();
        assert_eq!(entry.get("type").unwrap().as_str().unwrap(), "directory");
        assert_eq!(entry.get("source").unwrap().as_str().unwrap(), "outline");
    }

    // 验证 hook 模板文件（真实 block 引用）
    let hook_entry = entries
        .get("git/hooks/pre-commit")
        .expect("Missing hook file entry: git/hooks/pre-commit")
        .as_object()
        .unwrap();
    assert_eq!(hook_entry.get("type").unwrap().as_str().unwrap(), "file");
    // ID 不是 "hook-" 或 "dir-" 前缀，而是真实 block UUID
    let hook_id = hook_entry.get("id").unwrap().as_str().unwrap();
    assert!(
        !hook_id.starts_with("hook-") && !hook_id.starts_with("dir-"),
        "Hook file entry should reference a real block ID, got: {}",
        hook_id
    );

    assert_eq!(entries.len(), ELF_DIR_PATHS.len() + 1); // dirs + 1 hook file
}

#[tokio::test]
async fn test_hook_block_has_template_content() {
    let handle = setup_engine().await;
    create_editor(&handle, "system").await;
    let (_, hook_block_id) = bootstrap_elf_meta(&handle, "system").await;

    // hook block 应存在且包含模板内容
    let hook_block = handle
        .get_block(hook_block_id)
        .await
        .expect("pre-commit hook block should exist");
    assert_eq!(hook_block.name, "pre-commit");
    assert_eq!(hook_block.block_type, "code");

    // code.write 将内容存储在 "text" key 下
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
