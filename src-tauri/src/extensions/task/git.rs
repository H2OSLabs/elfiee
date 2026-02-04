/// Git command execution utilities for Elfiee task.commit workflow.
///
/// Provides functions for:
/// - Executing git commands with environment variable injection
/// - Branch creation and switching
/// - Add + commit workflow with ELFIEE_TASK_COMMIT bypass
/// - Branch name sanitization
use std::path::Path;

/// Execute a git command with optional environment variable injection.
///
/// # Arguments
/// - `repo_path`: Git repository root directory
/// - `args`: Git subcommand and arguments
/// - `env`: Additional environment variables to inject
pub async fn git_exec(
    repo_path: &str,
    args: &[&str],
    env: &[(&str, &str)],
) -> Result<String, String> {
    let mut cmd = tokio::process::Command::new("git");
    cmd.args(args).current_dir(repo_path);
    for (k, v) in env {
        cmd.env(k, v);
    }

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("Failed to execute git: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git {} failed: {}", args[0], stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Check if a path is a git repository.
pub async fn is_git_repo(repo_path: &str) -> bool {
    Path::new(repo_path).join(".git").exists()
}

/// Full workflow: create/switch branch → add → commit.
///
/// Sets `ELFIEE_TASK_COMMIT=1` environment variable to bypass Elfiee-managed git hooks.
///
/// # Arguments
/// - `repo_path`: Git repository root directory
/// - `branch_name`: Target branch name (e.g. `feat/fix-login`)
/// - `message`: Commit message
/// - `files`: Files to add (if empty, uses `git add -A`)
///
/// # Returns
/// commit hash string
pub async fn git_commit_flow(
    repo_path: &str,
    branch_name: &str,
    message: &str,
    files: &[String],
) -> Result<String, String> {
    // Check if branch already exists
    let branch_list = git_exec(repo_path, &["branch", "--list", branch_name], &[]).await?;
    if branch_list.trim().is_empty() {
        git_exec(repo_path, &["checkout", "-b", branch_name], &[]).await?;
    } else {
        git_exec(repo_path, &["checkout", branch_name], &[]).await?;
    }

    // git add
    if files.is_empty() {
        git_exec(repo_path, &["add", "-A"], &[("ELFIEE_TASK_COMMIT", "1")]).await?;
    } else {
        let mut add_args = vec!["add"];
        let file_refs: Vec<&str> = files.iter().map(|s| s.as_str()).collect();
        add_args.extend(file_refs);
        git_exec(repo_path, &add_args, &[("ELFIEE_TASK_COMMIT", "1")]).await?;
    }

    // Check if there are changes to commit
    let status = git_exec(repo_path, &["status", "--porcelain"], &[]).await?;
    if status.trim().is_empty() {
        return Err("No changes to commit".to_string());
    }

    // git commit (ELFIEE_TASK_COMMIT=1 bypasses Elfiee hooks)
    git_exec(
        repo_path,
        &["commit", "-m", message],
        &[("ELFIEE_TASK_COMMIT", "1")],
    )
    .await?;

    // Get commit hash
    let hash = git_exec(repo_path, &["rev-parse", "HEAD"], &[]).await?;
    Ok(hash.trim().to_string())
}

/// Sanitize a branch name by removing illegal characters.
///
/// Git branch name rules:
/// - Cannot contain spaces, ~, ^, :, ?, *, [, \
/// - Cannot start or end with .
/// - Cannot contain consecutive ..
/// - ASCII characters converted to lowercase (non-ASCII preserved as-is)
pub fn sanitize_branch_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '/' {
                c
            } else {
                '-'
            }
        })
        .collect();

    // Remove leading/trailing dashes and collapse consecutive dashes
    let mut result = String::new();
    let mut prev_dash = false;
    for c in sanitized.chars() {
        if c == '-' {
            if !prev_dash && !result.is_empty() {
                result.push(c);
                prev_dash = true;
            }
        } else {
            result.push(c);
            prev_dash = false;
        }
    }

    result.trim_end_matches('-').to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_branch_name_basic() {
        assert_eq!(sanitize_branch_name("fix login bug"), "fix-login-bug");
    }

    #[test]
    fn test_sanitize_branch_name_chinese() {
        // CJK characters are Unicode alphanumeric, preserved as-is (no case conversion)
        assert_eq!(sanitize_branch_name("实现登录功能"), "实现登录功能");
    }

    #[test]
    fn test_sanitize_branch_name_special_chars() {
        assert_eq!(sanitize_branch_name("feat: add OAuth!"), "feat-add-oauth");
    }

    #[test]
    fn test_sanitize_branch_name_already_clean() {
        assert_eq!(sanitize_branch_name("fix-login"), "fix-login");
    }

    #[test]
    fn test_sanitize_branch_name_with_slash() {
        assert_eq!(sanitize_branch_name("feat/login"), "feat/login");
    }

    #[test]
    fn test_sanitize_branch_name_trailing_dash() {
        assert_eq!(sanitize_branch_name("fix-bug-"), "fix-bug");
    }

    #[test]
    fn test_sanitize_branch_name_consecutive_dashes() {
        assert_eq!(sanitize_branch_name("fix--bug"), "fix-bug");
    }

    #[test]
    fn test_sanitize_branch_name_empty() {
        assert_eq!(sanitize_branch_name(""), "");
    }

    #[test]
    fn test_sanitize_branch_name_mixed_ascii_unicode() {
        // ASCII letters are lowercased, non-ASCII Unicode preserved as-is
        assert_eq!(sanitize_branch_name("Fix-登录-Bug"), "fix-登录-bug");
    }

    // Async tests require a git repo, placed in integration tests
    #[tokio::test]
    async fn test_is_git_repo_nonexistent() {
        assert!(!is_git_repo("/nonexistent/path").await);
    }

    #[tokio::test]
    async fn test_git_commit_flow_with_temp_repo() {
        let temp = tempfile::TempDir::new().unwrap();
        let repo_path = temp.path().to_str().unwrap();

        // Initialize git repo
        git_exec(repo_path, &["init"], &[]).await.unwrap();
        git_exec(repo_path, &["config", "user.email", "test@test.com"], &[])
            .await
            .unwrap();
        git_exec(repo_path, &["config", "user.name", "Test"], &[])
            .await
            .unwrap();

        // Create a file
        std::fs::write(temp.path().join("test.txt"), "hello").unwrap();
        git_exec(repo_path, &["add", "."], &[]).await.unwrap();
        git_exec(repo_path, &["commit", "-m", "initial"], &[])
            .await
            .unwrap();

        // Create another file for commit
        std::fs::write(temp.path().join("feature.txt"), "new feature").unwrap();

        // Use git_commit_flow
        let hash = git_commit_flow(repo_path, "feat/test-feature", "Add test feature", &[])
            .await
            .unwrap();

        assert!(!hash.is_empty(), "Should return a commit hash");
        assert_eq!(hash.len(), 40, "SHA-1 hash should be 40 chars");

        // Verify branch exists
        let branches = git_exec(repo_path, &["branch"], &[]).await.unwrap();
        assert!(branches.contains("feat/test-feature"));

        // Verify commit message
        let log = git_exec(repo_path, &["log", "--oneline", "-1"], &[])
            .await
            .unwrap();
        assert!(log.contains("Add test feature"));
    }

    #[tokio::test]
    async fn test_git_commit_flow_existing_branch() {
        let temp = tempfile::TempDir::new().unwrap();
        let repo_path = temp.path().to_str().unwrap();

        // Initialize
        git_exec(repo_path, &["init"], &[]).await.unwrap();
        git_exec(repo_path, &["config", "user.email", "test@test.com"], &[])
            .await
            .unwrap();
        git_exec(repo_path, &["config", "user.name", "Test"], &[])
            .await
            .unwrap();

        std::fs::write(temp.path().join("init.txt"), "init").unwrap();
        git_exec(repo_path, &["add", "."], &[]).await.unwrap();
        git_exec(repo_path, &["commit", "-m", "initial"], &[])
            .await
            .unwrap();

        // First commit
        std::fs::write(temp.path().join("f1.txt"), "first").unwrap();
        git_commit_flow(repo_path, "feat/test", "First commit", &[])
            .await
            .unwrap();

        // Switch back to main/master
        let checkout_result = git_exec(repo_path, &["checkout", "master"], &[]).await;
        if checkout_result.is_err() {
            git_exec(repo_path, &["checkout", "main"], &[])
                .await
                .unwrap();
        }

        // Second commit on main
        std::fs::write(temp.path().join("f2.txt"), "second").unwrap();
        git_exec(repo_path, &["add", "."], &[]).await.unwrap();
        git_exec(repo_path, &["commit", "-m", "on main"], &[])
            .await
            .unwrap();

        // Switch to existing branch and commit again
        std::fs::write(temp.path().join("f3.txt"), "third").unwrap();
        let hash = git_commit_flow(repo_path, "feat/test", "Second commit on feat", &[])
            .await
            .unwrap();

        assert!(!hash.is_empty());
    }

    #[tokio::test]
    async fn test_git_commit_flow_no_changes() {
        let temp = tempfile::TempDir::new().unwrap();
        let repo_path = temp.path().to_str().unwrap();

        git_exec(repo_path, &["init"], &[]).await.unwrap();
        git_exec(repo_path, &["config", "user.email", "test@test.com"], &[])
            .await
            .unwrap();
        git_exec(repo_path, &["config", "user.name", "Test"], &[])
            .await
            .unwrap();

        std::fs::write(temp.path().join("init.txt"), "init").unwrap();
        git_exec(repo_path, &["add", "."], &[]).await.unwrap();
        git_exec(repo_path, &["commit", "-m", "initial"], &[])
            .await
            .unwrap();

        // No new changes, should return error
        let result = git_commit_flow(repo_path, "feat/empty", "No changes", &[]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No changes to commit"));
    }
}
