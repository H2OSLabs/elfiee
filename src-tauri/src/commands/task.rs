/// Tauri commands for task operations.
///
/// Implements the I/O side of the Split Pattern for task.commit:
/// - Capability handler (extensions/task/task_commit.rs): validation + audit event
/// - This module: auto-discover repos + file export + git operations
/// - Git hooks injection/removal for linked repositories
use crate::models::Command;
use crate::state::AppState;
use crate::utils::git::{git_commit_flow, is_git_repo, sanitize_branch_name};
use crate::utils::git_hooks::{inject_git_hooks, is_hooks_injected, remove_git_hooks};
use crate::utils::path_validator::validate_virtual_path;
use serde::{Deserialize, Serialize};
use specta::specta;
use specta::Type;
use std::collections::HashMap;
use std::path::Path;
use tauri::State;

/// Result of a task.commit operation.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TaskCommitResult {
    /// Git commit hash
    pub commit_hash: String,
    /// Git branch name
    pub branch_name: String,
    /// List of exported file paths
    pub exported_files: Vec<String>,
}

/// Auto-discover the repo path and entry key for a given block.
///
/// Traverses all directory blocks with `source=linked` and checks their entries
/// for the target block_id. Returns `(external_root_path, entry_key)` if found,
/// where entry_key is the relative file path (e.g. `src/main.rs`).
fn find_block_repo_path(
    all_blocks: &HashMap<String, crate::models::Block>,
    target_block_id: &str,
) -> Option<(String, String)> {
    for block in all_blocks.values() {
        if block.block_type != "directory" {
            continue;
        }

        let source = block
            .contents
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if source != "linked" {
            continue;
        }

        if let Some(entries) = block.contents.get("entries").and_then(|v| v.as_object()) {
            for (entry_key, entry) in entries {
                if let Some(id) = entry.get("id").and_then(|v| v.as_str()) {
                    if id == target_block_id {
                        let repo_path = block
                            .metadata
                            .custom
                            .get("external_root_path")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())?;
                        return Some((repo_path, entry_key.clone()));
                    }
                }
            }
        }
    }
    None
}

/// Execute a task commit: validate → auto-discover repo → export snapshots → git commit.
///
/// This command follows the Split Pattern:
/// 1. Calls task.commit capability handler (authorization + audit event)
/// 2. Auto-discovers linked repo from downstream blocks
/// 3. Verifies discovered path has .git
/// 4. Copies downstream block snapshots to repo path
/// 5. Executes git branch + add + commit flow
///
/// # Arguments
/// * `file_id` - Elf file containing the task block
/// * `task_block_id` - The task block to commit
/// * `editor_id` - Optional editor ID (defaults to active editor)
#[tauri::command]
#[specta]
pub async fn commit_task(
    file_id: String,
    task_block_id: String,
    editor_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<TaskCommitResult, String> {
    // Get engine handle
    let handle = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or_else(|| format!("File '{}' is not open", file_id))?;

    // Determine effective editor_id
    let effective_editor_id = if let Some(id) = editor_id {
        id
    } else {
        state
            .active_editors
            .get(&file_id)
            .map(|e| e.value().clone())
            .ok_or("No active editor set for this file")?
    };

    // Step 1: Execute task.commit capability (authorization + audit event)
    let cmd = Command::new(
        effective_editor_id.clone(),
        "task.commit".to_string(),
        task_block_id.clone(),
        serde_json::json!({}),
    );
    let events = handle.process_command(cmd).await?;

    // Step 2: Extract downstream_block_ids from the commit event
    let commit_event = events.first().ok_or("No commit event generated")?;
    let downstream_ids: Vec<String> = serde_json::from_value(
        commit_event
            .value
            .get("downstream_block_ids")
            .cloned()
            .ok_or("Missing downstream_block_ids in commit event")?,
    )
    .map_err(|e| format!("Failed to parse downstream_block_ids: {}", e))?;

    // Step 3: Get all blocks and auto-discover repo paths
    let all_blocks = handle.get_all_blocks().await;

    // Group downstream blocks by repo_path, recording entry_key (original relative path)
    let mut repo_blocks: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for block_id in &downstream_ids {
        if let Some((repo_path, entry_key)) = find_block_repo_path(&all_blocks, block_id) {
            repo_blocks
                .entry(repo_path)
                .or_default()
                .push((block_id.clone(), entry_key));
        }
    }

    if repo_blocks.is_empty() {
        return Err(
            "No linked git repositories found for downstream blocks. Import a directory first."
                .to_string(),
        );
    }

    // Step 4: Get task block info
    let task_block = handle
        .get_block(task_block_id.clone())
        .await
        .ok_or("Task block not found after commit")?;
    let task_name = &task_block.name;
    let task_description = task_block.metadata.description.as_deref().unwrap_or("");

    // Step 5: Export files + git commit per repository
    let mut all_exported_files = Vec::new();
    let mut last_commit_hash = String::new();
    let mut last_branch_name = String::new();

    // Get elf hooks dir (stored in elf temp dir, auto-cleaned on crash/close)
    let elf_hooks_dir = get_elf_hooks_dir(&file_id, &state)?;

    for (repo_path, block_entries) in &repo_blocks {
        // Verify repo_path is a git repository
        if !is_git_repo(repo_path).await {
            return Err(format!(
                "Project '{}' does not have a .git directory. Target must be a git repository.",
                repo_path
            ));
        }

        // Auto-inject hooks if not already present
        if !is_hooks_injected(repo_path, &elf_hooks_dir).await {
            tokio::fs::create_dir_all(&elf_hooks_dir)
                .await
                .map_err(|e| format!("Failed to create hooks directory: {}", e))?;
            if let Err(e) = inject_git_hooks(repo_path, &elf_hooks_dir).await {
                log::warn!("Failed to inject git hooks for {}: {}", repo_path, e);
            }
        }

        // Export downstream block contents to their original file paths
        let mut exported_files = Vec::new();
        for (block_id, entry_key) in block_entries {
            // Security: validate entry_key before joining with repo_path
            validate_virtual_path(entry_key)?;

            let block = all_blocks
                .get(block_id)
                .ok_or_else(|| format!("Downstream block {} not found", block_id))?;

            let content = block
                .contents
                .get("text")
                .or_else(|| block.contents.get("markdown"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let file_path = Path::new(repo_path).join(entry_key);
            if let Some(parent) = file_path.parent() {
                tokio::fs::create_dir_all(parent).await.map_err(|e| {
                    format!(
                        "Failed to create parent directory for '{}': {}",
                        entry_key, e
                    )
                })?;
            }

            tokio::fs::write(&file_path, content).await.map_err(|e| {
                format!(
                    "Failed to export block {} to '{}': {}",
                    block_id, entry_key, e
                )
            })?;
            exported_files.push(entry_key.clone());
            all_exported_files.push(entry_key.clone());
        }

        // Git operations (only add the specifically exported files)
        let branch_name = format!("feat/{}", sanitize_branch_name(task_name));
        let commit_hash =
            git_commit_flow(repo_path, &branch_name, task_description, &exported_files).await?;

        last_commit_hash = commit_hash;
        last_branch_name = branch_name;
    }

    Ok(TaskCommitResult {
        commit_hash: last_commit_hash,
        branch_name: last_branch_name,
        exported_files: all_exported_files,
    })
}

/// Inject git hooks into a linked repository (commit protect ON).
///
/// Hooks are stored in the .elf temp dir, so they disappear on crash/close.
/// Sets `core.hooksPath` to block direct commits and require task.commit workflow.
///
/// # Arguments
/// * `file_id` - Elf file ID (used to locate temp dir)
/// * `repo_path` - External project git repo root
#[tauri::command]
#[specta]
pub async fn inject_hooks_for_repo(
    file_id: String,
    repo_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if !is_git_repo(&repo_path).await {
        return Err(format!("'{}' is not a git repository", repo_path));
    }

    let elf_hooks_dir = get_elf_hooks_dir(&file_id, &state)?;

    if is_hooks_injected(&repo_path, &elf_hooks_dir).await {
        return Ok(());
    }

    tokio::fs::create_dir_all(&elf_hooks_dir)
        .await
        .map_err(|e| format!("Failed to create hooks directory: {}", e))?;
    inject_git_hooks(&repo_path, &elf_hooks_dir).await
}

/// Remove git hooks from a linked repository (commit protect OFF).
///
/// Restores original `core.hooksPath` and cleans up Elfiee hook files.
#[tauri::command]
#[specta]
pub async fn remove_hooks_for_repo(
    file_id: String,
    repo_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let elf_hooks_dir = get_elf_hooks_dir(&file_id, &state)?;
    remove_git_hooks(&repo_path, &elf_hooks_dir).await
}

/// Check if git hooks are currently injected for a repo.
#[tauri::command]
#[specta]
pub async fn is_hooks_active(
    file_id: String,
    repo_path: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let elf_hooks_dir = get_elf_hooks_dir(&file_id, &state)?;
    Ok(is_hooks_injected(&repo_path, &elf_hooks_dir).await)
}

/// Compute the elf hooks directory path from file_id's temp dir.
fn get_elf_hooks_dir(file_id: &str, state: &AppState) -> Result<String, String> {
    let temp_dir = state
        .files
        .get(file_id)
        .map(|f| f.archive.temp_path().to_path_buf())
        .ok_or_else(|| format!("File '{}' not found in state", file_id))?;
    Ok(temp_dir
        .join(".elf/git/hooks")
        .to_string_lossy()
        .to_string())
}
