/// Git command execution utilities for Elfiee task.commit workflow.
///
/// Provides functions for:
/// - Executing git commands with environment variable injection
/// - Branch creation and switching
/// - Add + commit workflow with ELFIEE_TASK_COMMIT bypass
/// - Branch name sanitization
use std::path::Path;

/// 执行 git 命令（支持环境变量注入）
///
/// # Arguments
/// - `repo_path`: git repo 根目录
/// - `args`: git 子命令及参数
/// - `env`: 额外注入的环境变量
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

/// 检查路径是否是 git 仓库
pub async fn is_git_repo(repo_path: &str) -> bool {
    Path::new(repo_path).join(".git").exists()
}

/// 创建或切换分支 → add → commit 的完整流程
///
/// 设置 `ELFIEE_TASK_COMMIT=1` 环境变量让 Elfiee 管理的 git hook 放行。
///
/// # Arguments
/// - `repo_path`: git repo 根目录
/// - `branch_name`: 目标分支名（如 `feat/fix-login`）
/// - `message`: commit message
/// - `files`: 要 add 的文件列表（如为空则 `git add -A`）
///
/// # Returns
/// commit hash string
pub async fn git_commit_flow(
    repo_path: &str,
    branch_name: &str,
    message: &str,
    files: &[String],
) -> Result<String, String> {
    // 检查分支是否已存在
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

    // 检查是否有变更需要 commit
    let status = git_exec(repo_path, &["status", "--porcelain"], &[]).await?;
    if status.trim().is_empty() {
        return Err("No changes to commit".to_string());
    }

    // git commit（设置 ELFIEE_TASK_COMMIT=1 让 hook 放行）
    git_exec(
        repo_path,
        &["commit", "-m", message],
        &[("ELFIEE_TASK_COMMIT", "1")],
    )
    .await?;

    // 获取 commit hash
    let hash = git_exec(repo_path, &["rev-parse", "HEAD"], &[]).await?;
    Ok(hash.trim().to_string())
}

/// 清洗分支名（去掉非法字符）
///
/// Git 分支名规则：
/// - 不能包含空格、~、^、:、?、*、[、\
/// - 不能以 . 开头或结尾
/// - 不能包含连续的 ..
/// - 转为小写
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

    // 去掉首尾的 - 和 .，合并连续的 -
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

    result.trim_end_matches('-').to_lowercase()
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
        // 中文字符是 Unicode alphanumeric，保留并转小写
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

    // 异步测试需要 git repo，放在集成测试中
    #[tokio::test]
    async fn test_is_git_repo_nonexistent() {
        assert!(!is_git_repo("/nonexistent/path").await);
    }

    #[tokio::test]
    async fn test_git_commit_flow_with_temp_repo() {
        let temp = tempfile::TempDir::new().unwrap();
        let repo_path = temp.path().to_str().unwrap();

        // 初始化 git repo
        git_exec(repo_path, &["init"], &[]).await.unwrap();
        git_exec(repo_path, &["config", "user.email", "test@test.com"], &[])
            .await
            .unwrap();
        git_exec(repo_path, &["config", "user.name", "Test"], &[])
            .await
            .unwrap();

        // 创建一个文件
        std::fs::write(temp.path().join("test.txt"), "hello").unwrap();
        git_exec(repo_path, &["add", "."], &[]).await.unwrap();
        git_exec(repo_path, &["commit", "-m", "initial"], &[])
            .await
            .unwrap();

        // 创建另一个文件准备 commit
        std::fs::write(temp.path().join("feature.txt"), "new feature").unwrap();

        // 使用 git_commit_flow
        let hash = git_commit_flow(repo_path, "feat/test-feature", "Add test feature", &[])
            .await
            .unwrap();

        assert!(!hash.is_empty(), "Should return a commit hash");
        assert_eq!(hash.len(), 40, "SHA-1 hash should be 40 chars");

        // 验证分支存在
        let branches = git_exec(repo_path, &["branch"], &[]).await.unwrap();
        assert!(branches.contains("feat/test-feature"));

        // 验证 commit message
        let log = git_exec(repo_path, &["log", "--oneline", "-1"], &[])
            .await
            .unwrap();
        assert!(log.contains("Add test feature"));
    }

    #[tokio::test]
    async fn test_git_commit_flow_existing_branch() {
        let temp = tempfile::TempDir::new().unwrap();
        let repo_path = temp.path().to_str().unwrap();

        // 初始化
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

        // 第一次 commit
        std::fs::write(temp.path().join("f1.txt"), "first").unwrap();
        git_commit_flow(repo_path, "feat/test", "First commit", &[])
            .await
            .unwrap();

        // 切回 main/master
        let checkout_result = git_exec(repo_path, &["checkout", "master"], &[]).await;
        if checkout_result.is_err() {
            git_exec(repo_path, &["checkout", "main"], &[])
                .await
                .unwrap();
        }

        // 第二次 commit 到同一分支
        std::fs::write(temp.path().join("f2.txt"), "second").unwrap();
        git_exec(repo_path, &["add", "."], &[]).await.unwrap();
        git_exec(repo_path, &["commit", "-m", "on main"], &[])
            .await
            .unwrap();

        // 切到已有分支再 commit
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

        // 没有新变更，应该报错
        let result = git_commit_flow(repo_path, "feat/empty", "No changes", &[]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No changes to commit"));
    }
}
