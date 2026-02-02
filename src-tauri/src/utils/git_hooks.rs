/// Git hooks injection and removal for Elfiee workflow.
///
/// Uses `core.hooksPath` to inject Elfiee-managed hooks into external projects.
/// Chain-call pattern: Elfiee's hook first executes original project hooks,
/// then checks for Elfiee workflow compliance.
///
/// ## Design
/// - Injects hooks at `directory.import` time (or manual trigger)
/// - Removes hooks at app close (or `agent.disable`)
/// - `ELFIEE_TASK_COMMIT=1` env var lets `task.commit` bypass the hook check
/// - Original project hooks (husky, lint-staged, etc.) are preserved via chain-call
use super::git::git_exec;
use std::path::Path;

/// Pre-commit hook 脚本模板。
///
/// 链式调用原始 hook，然后检查 ELFIEE_TASK_COMMIT 环境变量。
/// `pub` 供 `elf_meta::bootstrap_elf_meta` 创建 hook block 时使用。
pub const PRE_COMMIT_HOOK_CONTENT: &str = r#"#!/bin/sh
# Elfiee managed hook — chain to original, then check Elfiee workflow
# Auto-removed when Elfiee closes
# Bypass all hooks: git commit --no-verify

# ── Step 1: 链式调用原始 hook ──
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ORIGINAL_HOOKS_DIR=""

# 情况 A: 原项目设置了 core.hooksPath（如 husky → .husky/）
if [ -f "$SCRIPT_DIR/../original-hooks-path" ]; then
    ORIGINAL_HOOKS_DIR=$(cat "$SCRIPT_DIR/../original-hooks-path")
# 情况 B: 原项目用默认 .git/hooks/
elif [ -x ".git/hooks/pre-commit" ]; then
    ORIGINAL_HOOKS_DIR=".git/hooks"
fi

# 执行原始 hook（lint / format / test 等规则继续生效）
if [ -n "$ORIGINAL_HOOKS_DIR" ] && [ -x "$ORIGINAL_HOOKS_DIR/pre-commit" ]; then
    "$ORIGINAL_HOOKS_DIR/pre-commit" "$@"
    RESULT=$?
    if [ $RESULT -ne 0 ]; then
        exit $RESULT  # 原规则失败 → 直接拒绝，不到 Elfiee 检查
    fi
fi

# ── Step 2: Elfiee 工作流检查 ──
# 检查是否由 task.commit 发起（环境变量标记）
if [ "$ELFIEE_TASK_COMMIT" = "1" ]; then
    exit 0  # task.commit 流程，放行
fi

echo "[Elfiee] Direct commit detected outside Elfiee workflow."
echo "[Elfiee] Use task.commit in Elfiee for tracked commits."
echo "[Elfiee] Bypass: git commit --no-verify"
exit 1
"#;

/// 注入 git hooks（设置 core.hooksPath 指向 .elf/git/hooks/）
///
/// # Arguments
/// - `repo_path`: 外部项目 git repo 根目录
/// - `elf_hooks_dir`: Elfiee 管理的 hooks 目录路径（如 `.elf/git/hooks/`）
pub async fn inject_git_hooks(repo_path: &str, elf_hooks_dir: &str) -> Result<(), String> {
    // 确保 hooks 目录存在
    std::fs::create_dir_all(elf_hooks_dir)
        .map_err(|e| format!("Failed to create hooks directory: {}", e))?;

    // 检查是否有现存的 hooksPath 设置
    let existing = git_exec(
        repo_path,
        &["config", "--local", "--get", "core.hooksPath"],
        &[],
    )
    .await;

    if let Ok(ref path) = existing {
        let path = path.trim();
        if !path.is_empty() && path != elf_hooks_dir {
            // 保存原始路径到 .elf/git/original-hooks-path
            let original_path_file = Path::new(elf_hooks_dir)
                .parent()
                .unwrap_or(Path::new(elf_hooks_dir))
                .join("original-hooks-path");
            std::fs::write(&original_path_file, path)
                .map_err(|e| format!("Failed to save original hooks path: {}", e))?;
        }
    }

    // 生成 pre-commit hook
    let hook_path = Path::new(elf_hooks_dir).join("pre-commit");
    std::fs::write(&hook_path, PRE_COMMIT_HOOK_CONTENT)
        .map_err(|e| format!("Failed to write hook: {}", e))?;

    // 设置可执行权限（Unix）
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook_path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("Failed to set hook permissions: {}", e))?;
    }

    // 设置 core.hooksPath
    git_exec(
        repo_path,
        &["config", "--local", "core.hooksPath", elf_hooks_dir],
        &[],
    )
    .await?;

    log::info!("Git hooks injected: core.hooksPath → {}", elf_hooks_dir);

    Ok(())
}

/// 撤销 git hooks（恢复 core.hooksPath）
///
/// # Arguments
/// - `repo_path`: 外部项目 git repo 根目录
/// - `elf_hooks_dir`: Elfiee 管理的 hooks 目录路径
pub async fn remove_git_hooks(repo_path: &str, elf_hooks_dir: &str) -> Result<(), String> {
    let original_path_file = Path::new(elf_hooks_dir)
        .parent()
        .unwrap_or(Path::new(elf_hooks_dir))
        .join("original-hooks-path");

    if original_path_file.exists() {
        // 恢复原始 hooksPath
        let original = std::fs::read_to_string(&original_path_file)
            .map_err(|e| format!("Failed to read original hooks path: {}", e))?;
        git_exec(
            repo_path,
            &["config", "--local", "core.hooksPath", original.trim()],
            &[],
        )
        .await?;
        let _ = std::fs::remove_file(&original_path_file);
    } else {
        // 移除设置（允许失败，可能已经被手动清理）
        let _ = git_exec(
            repo_path,
            &["config", "--local", "--unset", "core.hooksPath"],
            &[],
        )
        .await;
    }

    // 清理 hook 文件
    let hook_path = Path::new(elf_hooks_dir).join("pre-commit");
    let _ = std::fs::remove_file(&hook_path);

    log::info!("Git hooks removed for {}", repo_path);

    Ok(())
}

/// 检查当前 core.hooksPath 是否指向 Elfiee 管理的目录
pub async fn is_hooks_injected(repo_path: &str, elf_hooks_dir: &str) -> bool {
    match git_exec(
        repo_path,
        &["config", "--local", "--get", "core.hooksPath"],
        &[],
    )
    .await
    {
        Ok(path) => path.trim() == elf_hooks_dir,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_git_repo() -> tempfile::TempDir {
        let temp = tempfile::TempDir::new().unwrap();
        let repo_path = temp.path().to_str().unwrap();

        git_exec(repo_path, &["init"], &[]).await.unwrap();
        git_exec(repo_path, &["config", "user.email", "test@test.com"], &[])
            .await
            .unwrap();
        git_exec(repo_path, &["config", "user.name", "Test"], &[])
            .await
            .unwrap();

        // 初始 commit
        std::fs::write(temp.path().join("README.md"), "# Test").unwrap();
        git_exec(repo_path, &["add", "."], &[]).await.unwrap();
        git_exec(repo_path, &["commit", "-m", "initial"], &[])
            .await
            .unwrap();

        temp
    }

    #[tokio::test]
    async fn test_inject_git_hooks() {
        let temp = setup_git_repo().await;
        let repo_path = temp.path().to_str().unwrap();
        let hooks_dir = temp.path().join(".elf/git/hooks");
        let hooks_dir_str = hooks_dir.to_str().unwrap();

        inject_git_hooks(repo_path, hooks_dir_str).await.unwrap();

        // 验证 core.hooksPath 已设置
        let hooks_path = git_exec(
            repo_path,
            &["config", "--local", "--get", "core.hooksPath"],
            &[],
        )
        .await
        .unwrap();
        assert_eq!(hooks_path.trim(), hooks_dir_str);

        // 验证 pre-commit hook 存在且可执行
        let hook_path = hooks_dir.join("pre-commit");
        assert!(hook_path.exists());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::metadata(&hook_path).unwrap().permissions();
            assert!(perms.mode() & 0o111 != 0, "Hook should be executable");
        }
    }

    #[tokio::test]
    async fn test_remove_git_hooks() {
        let temp = setup_git_repo().await;
        let repo_path = temp.path().to_str().unwrap();
        let hooks_dir = temp.path().join(".elf/git/hooks");
        let hooks_dir_str = hooks_dir.to_str().unwrap();

        // 注入
        inject_git_hooks(repo_path, hooks_dir_str).await.unwrap();
        assert!(is_hooks_injected(repo_path, hooks_dir_str).await);

        // 撤销
        remove_git_hooks(repo_path, hooks_dir_str).await.unwrap();
        assert!(!is_hooks_injected(repo_path, hooks_dir_str).await);

        // Hook 文件已删除
        assert!(!hooks_dir.join("pre-commit").exists());
    }

    #[tokio::test]
    async fn test_inject_preserves_original_hooks_path() {
        let temp = setup_git_repo().await;
        let repo_path = temp.path().to_str().unwrap();

        // 模拟原项目已有 hooksPath（如 husky）
        git_exec(
            repo_path,
            &["config", "--local", "core.hooksPath", ".husky"],
            &[],
        )
        .await
        .unwrap();

        let hooks_dir = temp.path().join(".elf/git/hooks");
        let hooks_dir_str = hooks_dir.to_str().unwrap();

        // 注入 Elfiee hooks
        inject_git_hooks(repo_path, hooks_dir_str).await.unwrap();

        // 验证原始路径被保存
        let original_path_file = temp.path().join(".elf/git/original-hooks-path");
        assert!(original_path_file.exists());
        let saved = std::fs::read_to_string(&original_path_file).unwrap();
        assert_eq!(saved.trim(), ".husky");

        // 撤销后恢复原始路径
        remove_git_hooks(repo_path, hooks_dir_str).await.unwrap();
        let restored = git_exec(
            repo_path,
            &["config", "--local", "--get", "core.hooksPath"],
            &[],
        )
        .await
        .unwrap();
        assert_eq!(restored.trim(), ".husky");
    }

    #[tokio::test]
    async fn test_is_hooks_injected() {
        let temp = setup_git_repo().await;
        let repo_path = temp.path().to_str().unwrap();
        let hooks_dir = temp.path().join(".elf/git/hooks");
        let hooks_dir_str = hooks_dir.to_str().unwrap();

        assert!(!is_hooks_injected(repo_path, hooks_dir_str).await);

        inject_git_hooks(repo_path, hooks_dir_str).await.unwrap();
        assert!(is_hooks_injected(repo_path, hooks_dir_str).await);
    }

    #[tokio::test]
    async fn test_hook_blocks_direct_commit() {
        let temp = setup_git_repo().await;
        let repo_path = temp.path().to_str().unwrap();
        let hooks_dir = temp.path().join(".elf/git/hooks");
        let hooks_dir_str = hooks_dir.to_str().unwrap();

        inject_git_hooks(repo_path, hooks_dir_str).await.unwrap();

        // 创建文件
        std::fs::write(temp.path().join("new.txt"), "content").unwrap();
        git_exec(repo_path, &["add", "."], &[]).await.unwrap();

        // 直接 commit（不设置 ELFIEE_TASK_COMMIT）应该被拦截
        let result = git_exec(repo_path, &["commit", "-m", "direct commit"], &[]).await;
        assert!(result.is_err(), "Direct commit should be blocked by hook");
    }

    #[tokio::test]
    async fn test_hook_allows_elfiee_task_commit() {
        let temp = setup_git_repo().await;
        let repo_path = temp.path().to_str().unwrap();
        let hooks_dir = temp.path().join(".elf/git/hooks");
        let hooks_dir_str = hooks_dir.to_str().unwrap();

        inject_git_hooks(repo_path, hooks_dir_str).await.unwrap();

        // 创建文件
        std::fs::write(temp.path().join("task.txt"), "task content").unwrap();
        git_exec(repo_path, &["add", "."], &[]).await.unwrap();

        // 设置 ELFIEE_TASK_COMMIT=1 → 应该放行
        let result = git_exec(
            repo_path,
            &["commit", "-m", "task commit"],
            &[("ELFIEE_TASK_COMMIT", "1")],
        )
        .await;
        assert!(
            result.is_ok(),
            "ELFIEE_TASK_COMMIT=1 should bypass hook: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_hook_allows_no_verify() {
        let temp = setup_git_repo().await;
        let repo_path = temp.path().to_str().unwrap();
        let hooks_dir = temp.path().join(".elf/git/hooks");
        let hooks_dir_str = hooks_dir.to_str().unwrap();

        inject_git_hooks(repo_path, hooks_dir_str).await.unwrap();

        std::fs::write(temp.path().join("bypass.txt"), "bypass").unwrap();
        git_exec(repo_path, &["add", "."], &[]).await.unwrap();

        // --no-verify 应该绕过所有 hooks
        let result = git_exec(
            repo_path,
            &["commit", "--no-verify", "-m", "bypass commit"],
            &[],
        )
        .await;
        assert!(result.is_ok(), "Should bypass with --no-verify");
    }
}
