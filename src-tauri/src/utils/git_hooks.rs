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

/// Pre-commit hook script template.
///
/// Chain-calls original hook, then checks ELFIEE_TASK_COMMIT environment variable.
/// `pub` for use by `elf_meta::bootstrap_elf_meta` when creating the hook block.
pub const PRE_COMMIT_HOOK_CONTENT: &str = r#"#!/bin/sh
# Elfiee managed hook — chain to original, then check Elfiee workflow
# Auto-removed when Elfiee closes
# Bypass all hooks: git commit --no-verify

# ── Step 1: Chain-call original hook ──
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ORIGINAL_HOOKS_DIR=""

# Case A: Project has custom core.hooksPath (e.g. husky → .husky/)
if [ -f "$SCRIPT_DIR/../original-hooks-path" ]; then
    ORIGINAL_HOOKS_DIR=$(cat "$SCRIPT_DIR/../original-hooks-path")
# Case B: Project uses default .git/hooks/
elif [ -x ".git/hooks/pre-commit" ]; then
    ORIGINAL_HOOKS_DIR=".git/hooks"
fi

# Execute original hook (lint / format / test rules still apply)
if [ -n "$ORIGINAL_HOOKS_DIR" ] && [ -x "$ORIGINAL_HOOKS_DIR/pre-commit" ]; then
    "$ORIGINAL_HOOKS_DIR/pre-commit" "$@"
    RESULT=$?
    if [ $RESULT -ne 0 ]; then
        exit $RESULT  # Original hook failed → reject, skip Elfiee check
    fi
fi

# ── Step 2: Elfiee workflow check ──
# Check if commit was initiated by task.commit (env var marker)
if [ "$ELFIEE_TASK_COMMIT" = "1" ]; then
    exit 0  # task.commit flow, allow
fi

echo "[Elfiee] Direct commit detected outside Elfiee workflow."
echo "[Elfiee] Use task.commit in Elfiee for tracked commits."
echo "[Elfiee] Bypass: git commit --no-verify"
exit 1
"#;

/// Inject git hooks (set core.hooksPath to .elf/git/hooks/).
///
/// # Arguments
/// - `repo_path`: External project git repository root
/// - `elf_hooks_dir`: Elfiee-managed hooks directory path (e.g. `.elf/git/hooks/`)
pub async fn inject_git_hooks(repo_path: &str, elf_hooks_dir: &str) -> Result<(), String> {
    // Ensure hooks directory exists
    std::fs::create_dir_all(elf_hooks_dir)
        .map_err(|e| format!("Failed to create hooks directory: {}", e))?;

    // Check for existing hooksPath setting
    let existing = git_exec(
        repo_path,
        &["config", "--local", "--get", "core.hooksPath"],
        &[],
    )
    .await;

    if let Ok(ref path) = existing {
        let path = path.trim();
        if !path.is_empty() && path != elf_hooks_dir {
            // Save original path to .elf/git/original-hooks-path
            let original_path_file = Path::new(elf_hooks_dir)
                .parent()
                .unwrap_or(Path::new(elf_hooks_dir))
                .join("original-hooks-path");
            std::fs::write(&original_path_file, path)
                .map_err(|e| format!("Failed to save original hooks path: {}", e))?;
        }
    }

    // Write pre-commit hook
    let hook_path = Path::new(elf_hooks_dir).join("pre-commit");
    std::fs::write(&hook_path, PRE_COMMIT_HOOK_CONTENT)
        .map_err(|e| format!("Failed to write hook: {}", e))?;

    // Set executable permissions (Unix)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook_path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("Failed to set hook permissions: {}", e))?;
    }

    // Set core.hooksPath
    git_exec(
        repo_path,
        &["config", "--local", "core.hooksPath", elf_hooks_dir],
        &[],
    )
    .await?;

    log::info!("Git hooks injected: core.hooksPath → {}", elf_hooks_dir);

    Ok(())
}

/// Remove git hooks (restore core.hooksPath).
///
/// # Arguments
/// - `repo_path`: External project git repository root
/// - `elf_hooks_dir`: Elfiee-managed hooks directory path
pub async fn remove_git_hooks(repo_path: &str, elf_hooks_dir: &str) -> Result<(), String> {
    let original_path_file = Path::new(elf_hooks_dir)
        .parent()
        .unwrap_or(Path::new(elf_hooks_dir))
        .join("original-hooks-path");

    if original_path_file.exists() {
        // Restore original hooksPath
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
        // Unset config (allow failure, may have been manually cleaned)
        let _ = git_exec(
            repo_path,
            &["config", "--local", "--unset", "core.hooksPath"],
            &[],
        )
        .await;
    }

    // Clean up hook files
    let hook_path = Path::new(elf_hooks_dir).join("pre-commit");
    let _ = std::fs::remove_file(&hook_path);

    log::info!("Git hooks removed for {}", repo_path);

    Ok(())
}

/// Check if core.hooksPath currently points to the Elfiee-managed directory.
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

        // Initial commit
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

        // Verify core.hooksPath is set
        let hooks_path = git_exec(
            repo_path,
            &["config", "--local", "--get", "core.hooksPath"],
            &[],
        )
        .await
        .unwrap();
        assert_eq!(hooks_path.trim(), hooks_dir_str);

        // Verify pre-commit hook exists and is executable
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

        // Inject
        inject_git_hooks(repo_path, hooks_dir_str).await.unwrap();
        assert!(is_hooks_injected(repo_path, hooks_dir_str).await);

        // Remove
        remove_git_hooks(repo_path, hooks_dir_str).await.unwrap();
        assert!(!is_hooks_injected(repo_path, hooks_dir_str).await);

        // Hook files should be deleted
        assert!(!hooks_dir.join("pre-commit").exists());
    }

    #[tokio::test]
    async fn test_inject_preserves_original_hooks_path() {
        let temp = setup_git_repo().await;
        let repo_path = temp.path().to_str().unwrap();

        // Simulate existing hooksPath (e.g. husky)
        git_exec(
            repo_path,
            &["config", "--local", "core.hooksPath", ".husky"],
            &[],
        )
        .await
        .unwrap();

        let hooks_dir = temp.path().join(".elf/git/hooks");
        let hooks_dir_str = hooks_dir.to_str().unwrap();

        // Inject Elfiee hooks
        inject_git_hooks(repo_path, hooks_dir_str).await.unwrap();

        // Verify original path was saved
        let original_path_file = temp.path().join(".elf/git/original-hooks-path");
        assert!(original_path_file.exists());
        let saved = std::fs::read_to_string(&original_path_file).unwrap();
        assert_eq!(saved.trim(), ".husky");

        // After removal, original path should be restored
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

        // Create a file
        std::fs::write(temp.path().join("new.txt"), "content").unwrap();
        git_exec(repo_path, &["add", "."], &[]).await.unwrap();

        // Direct commit (without ELFIEE_TASK_COMMIT) should be blocked
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

        // Create a file
        std::fs::write(temp.path().join("task.txt"), "task content").unwrap();
        git_exec(repo_path, &["add", "."], &[]).await.unwrap();

        // With ELFIEE_TASK_COMMIT=1 → should be allowed
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

        // --no-verify should bypass all hooks
        let result = git_exec(
            repo_path,
            &["commit", "--no-verify", "-m", "bypass commit"],
            &[],
        )
        .await;
        assert!(result.is_ok(), "Should bypass with --no-verify");
    }
}
