//! Tauri commands for Agent operations (Phase 2)
//!
//! These commands handle the I/O layer for agent operations:
//! - `agent_create`: Create Agent Block + auto-enable (symlink + MCP config)
//! - `agent_enable`: Re-enable agent (recreate symlink + MCP config)
//! - `agent_disable`: Disable agent (clean symlink + remove MCP config)

use crate::extensions::agent::{
    AgentContents, AgentCreateResult, AgentCreateV2Payload, AgentDisableResult, AgentEnableResult,
    AgentStatus,
};
use crate::models::Command;
use crate::state::AppState;
use crate::utils::mcp_config;
use std::path::Path;
use tauri::State;

/// Create symlink from source to destination (cross-platform).
///
/// On Unix: creates a symbolic link.
/// On Windows: creates a directory junction (no admin privileges required).
fn create_symlink_dir(src: &Path, dst: &Path) -> Result<(), String> {
    // Remove existing symlink/junction if present
    if dst.exists() || dst.read_link().is_ok() {
        remove_symlink_dir(dst)?;
    }

    // Ensure parent directory exists
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Failed to create parent directory {}: {}",
                parent.display(),
                e
            )
        })?;
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(src, dst).map_err(|e| {
            format!(
                "Failed to create symlink {} -> {}: {}",
                dst.display(),
                src.display(),
                e
            )
        })?;
    }

    #[cfg(windows)]
    {
        // On Windows, use directory junction (no admin privileges required)
        // junction crate is available in the Cargo.toml; if not, use symlink_dir
        // which may require developer mode or admin privileges.
        std::os::windows::fs::symlink_dir(src, dst).map_err(|e| {
            format!(
                "Failed to create symlink {} -> {}: {}. On Windows, enable Developer Mode or run as admin.",
                dst.display(),
                src.display(),
                e
            )
        })?;
    }

    Ok(())
}

/// Remove a symlink or junction directory.
fn remove_symlink_dir(path: &Path) -> Result<(), String> {
    if !path.exists() && path.read_link().is_err() {
        return Ok(()); // Nothing to remove
    }

    #[cfg(unix)]
    {
        std::fs::remove_file(path)
            .or_else(|_| std::fs::remove_dir(path))
            .map_err(|e| format!("Failed to remove symlink {}: {}", path.display(), e))?;
    }

    #[cfg(windows)]
    {
        // On Windows, symlink_dir creates a directory symlink
        std::fs::remove_dir(path).map_err(|e| {
            format!(
                "Failed to remove symlink/junction {}: {}",
                path.display(),
                e
            )
        })?;
    }

    Ok(())
}

/// Get external path from a directory block's metadata.
fn get_external_path(block: &crate::models::Block) -> Result<String, String> {
    block
        .metadata
        .custom
        .get("external_root_path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            format!(
                "Target project has no external path (block_id: {})",
                block.block_id
            )
        })
}

/// Find the .elf/ directory block and return its _block_dir path.
fn get_elf_block_dir(
    blocks: &std::collections::HashMap<String, crate::models::Block>,
) -> Option<String> {
    for block in blocks.values() {
        if block.name == ".elf" && block.block_type == "directory" {
            // _block_dir is injected at runtime by the engine
            if let Some(dir) = block.contents.get("_block_dir").and_then(|v| v.as_str()) {
                return Some(dir.to_string());
            }
        }
    }
    None
}

/// Perform enable I/O: create symlink + merge MCP config.
///
/// Returns a list of warnings for partial failures.
fn perform_enable_io(
    external_path: &str,
    elf_block_dir: &str,
    elf_file_path: &str,
) -> (bool, Vec<String>) {
    let mut warnings = Vec::new();

    // 1. Create symlink: {external_path}/.claude/skills/elfiee-client/ -> {elf_block_dir}/Agents/elfiee-client/
    let symlink_src = Path::new(elf_block_dir)
        .join("Agents")
        .join("elfiee-client");
    let symlink_dst = Path::new(external_path)
        .join(".claude")
        .join("skills")
        .join("elfiee-client");

    if let Err(e) = create_symlink_dir(&symlink_src, &symlink_dst) {
        warnings.push(format!("Failed to create symlink: {}", e));
    }

    // 2. Merge MCP config
    let mcp_config_path = Path::new(external_path).join(".claude").join("mcp.json");
    let server_config = mcp_config::build_elfiee_server_config(elf_file_path);

    if let Err(e) = mcp_config::merge_server(&mcp_config_path, "elfiee", server_config) {
        warnings.push(format!("Failed to write MCP config: {}", e));
    }

    let success = warnings.is_empty();
    (success, warnings)
}

/// Perform disable I/O: remove symlink + remove MCP config.
///
/// Returns a list of warnings for partial failures.
fn perform_disable_io(external_path: &str) -> Vec<String> {
    let mut warnings = Vec::new();

    // 1. Remove symlink
    let symlink_path = Path::new(external_path)
        .join(".claude")
        .join("skills")
        .join("elfiee-client");

    if let Err(e) = remove_symlink_dir(&symlink_path) {
        warnings.push(format!("Failed to remove symlink: {}", e));
    }

    // 2. Remove MCP config entry
    let mcp_config_path = Path::new(external_path).join(".claude").join("mcp.json");

    if let Err(e) = mcp_config::remove_server(&mcp_config_path, "elfiee") {
        warnings.push(format!("Failed to remove MCP config: {}", e));
    }

    warnings
}

/// Create an Agent Block for an external project and auto-enable it.
///
/// This command:
/// 1. Validates the target project (Dir Block exists, has external_path, has .claude/)
/// 2. Checks uniqueness (no existing agent for same project)
/// 3. Creates the Agent Block via engine
/// 4. Performs I/O: creates symlink + merges MCP config
#[tauri::command]
#[specta::specta]
pub async fn agent_create(
    state: State<'_, AppState>,
    file_id: String,
    payload: AgentCreateV2Payload,
) -> Result<AgentCreateResult, String> {
    // 1. Get engine handle
    let handle = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or_else(|| format!("File '{}' is not open", file_id))?;

    // 2. Get active editor
    let editor_id = state
        .get_active_editor(&file_id)
        .ok_or_else(|| "No active editor set for this file".to_string())?;

    // 3. Validate target project
    let target_block = handle
        .get_block(payload.target_project_id.clone())
        .await
        .ok_or_else(|| {
            format!(
                "Target project block not found: {}",
                payload.target_project_id
            )
        })?;

    if target_block.block_type != "directory" {
        return Err(format!(
            "Target block is not a directory (type: {})",
            target_block.block_type
        ));
    }

    let external_path = get_external_path(&target_block)?;

    // 4. Check .claude/ exists
    let claude_dir = Path::new(&external_path).join(".claude");
    if !claude_dir.exists() {
        return Err(format!(
            "Claude not initialized in target project: {}. Run 'claude' first.",
            external_path
        ));
    }

    // 5. Uniqueness check: no existing agent for this project
    let all_blocks = handle.get_all_blocks().await;
    for block in all_blocks.values() {
        if block.block_type == "agent" {
            if let Ok(contents) = serde_json::from_value::<AgentContents>(block.contents.clone()) {
                if contents.target_project_id == payload.target_project_id {
                    return Err(format!(
                        "Agent already exists for project: {} (block_id: {})",
                        block.name, block.block_id
                    ));
                }
            }
        }
    }

    // 6. Create Agent Block via engine
    let cmd = Command::new(
        editor_id.clone(),
        "agent.create".to_string(),
        "".to_string(),
        serde_json::json!(payload),
    );

    let events = handle.process_command(cmd).await?;

    let agent_block_id = events
        .first()
        .ok_or("No event returned from agent.create")?
        .entity
        .clone();

    // 7. Perform enable I/O
    // Get .elf file path from AppState
    let elf_file_path = state
        .files
        .get(&file_id)
        .map(|f| f.path.to_string_lossy().to_string())
        .ok_or_else(|| format!("File info not found for '{}'", file_id))?;

    // Get .elf/ block dir from all_blocks (need to re-fetch since state may have changed)
    let all_blocks = handle.get_all_blocks().await;
    let elf_block_dir = get_elf_block_dir(&all_blocks).ok_or_else(|| {
        ".elf/ directory block not found. Ensure .elf/ is initialized.".to_string()
    })?;

    let (io_success, warnings) = perform_enable_io(&external_path, &elf_block_dir, &elf_file_path);

    let message = if io_success {
        format!(
            "Agent '{}' created and enabled for project at {}. Please restart Claude Code to activate MCP.",
            payload.name.as_deref().unwrap_or("elfiee"),
            external_path
        )
    } else {
        format!(
            "Agent created but some I/O operations failed. Run agent.enable to retry. Warnings: {}",
            warnings.join("; ")
        )
    };

    Ok(AgentCreateResult {
        agent_block_id,
        status: AgentStatus::Enabled,
        needs_restart: io_success,
        message,
    })
}

/// Enable an Agent Block: recreate symlink and inject MCP config.
///
/// Idempotent: can be called on an already-enabled agent to refresh configuration.
#[tauri::command]
#[specta::specta]
pub async fn agent_enable(
    state: State<'_, AppState>,
    file_id: String,
    agent_block_id: String,
) -> Result<AgentEnableResult, String> {
    let handle = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or_else(|| format!("File '{}' is not open", file_id))?;

    let editor_id = state
        .get_active_editor(&file_id)
        .ok_or_else(|| "No active editor set for this file".to_string())?;

    // Get Agent Block
    let agent_block = handle
        .get_block(agent_block_id.clone())
        .await
        .ok_or_else(|| format!("Agent block not found: {}", agent_block_id))?;

    if agent_block.block_type != "agent" {
        return Err(format!(
            "Block is not an agent (type: {})",
            agent_block.block_type
        ));
    }

    let contents: AgentContents = serde_json::from_value(agent_block.contents.clone())
        .map_err(|e| format!("Invalid AgentContents: {}", e))?;

    // Get target project external path
    let target_block = handle
        .get_block(contents.target_project_id.clone())
        .await
        .ok_or_else(|| {
            format!(
                "Target project block not found: {}",
                contents.target_project_id
            )
        })?;

    let external_path = get_external_path(&target_block)?;

    // Process enable command via engine
    let cmd = Command::new(
        editor_id.clone(),
        "agent.enable".to_string(),
        agent_block_id.clone(),
        serde_json::json!({}),
    );

    handle.process_command(cmd).await?;

    // Perform I/O
    let elf_file_path = state
        .files
        .get(&file_id)
        .map(|f| f.path.to_string_lossy().to_string())
        .ok_or_else(|| format!("File info not found for '{}'", file_id))?;

    let all_blocks = handle.get_all_blocks().await;
    let elf_block_dir = get_elf_block_dir(&all_blocks).ok_or_else(|| {
        ".elf/ directory block not found. Ensure .elf/ is initialized.".to_string()
    })?;

    let (io_success, warnings) = perform_enable_io(&external_path, &elf_block_dir, &elf_file_path);

    let message = if io_success {
        format!(
            "Agent enabled for project at {}. Please restart Claude Code to activate MCP.",
            external_path
        )
    } else {
        format!(
            "Agent enabled but some I/O operations failed: {}",
            warnings.join("; ")
        )
    };

    Ok(AgentEnableResult {
        agent_block_id,
        status: AgentStatus::Enabled,
        needs_restart: io_success,
        message,
        warnings,
    })
}

/// Disable an Agent Block: remove symlink and MCP config.
///
/// Idempotent: can be called on an already-disabled agent.
#[tauri::command]
#[specta::specta]
pub async fn agent_disable(
    state: State<'_, AppState>,
    file_id: String,
    agent_block_id: String,
) -> Result<AgentDisableResult, String> {
    let handle = state
        .engine_manager
        .get_engine(&file_id)
        .ok_or_else(|| format!("File '{}' is not open", file_id))?;

    let editor_id = state
        .get_active_editor(&file_id)
        .ok_or_else(|| "No active editor set for this file".to_string())?;

    // Get Agent Block
    let agent_block = handle
        .get_block(agent_block_id.clone())
        .await
        .ok_or_else(|| format!("Agent block not found: {}", agent_block_id))?;

    if agent_block.block_type != "agent" {
        return Err(format!(
            "Block is not an agent (type: {})",
            agent_block.block_type
        ));
    }

    let contents: AgentContents = serde_json::from_value(agent_block.contents.clone())
        .map_err(|e| format!("Invalid AgentContents: {}", e))?;

    // Get target project external path
    let target_block = handle
        .get_block(contents.target_project_id.clone())
        .await
        .ok_or_else(|| {
            format!(
                "Target project block not found: {}",
                contents.target_project_id
            )
        })?;

    let external_path = get_external_path(&target_block)?;

    // Process disable command via engine
    let cmd = Command::new(
        editor_id.clone(),
        "agent.disable".to_string(),
        agent_block_id.clone(),
        serde_json::json!({}),
    );

    handle.process_command(cmd).await?;

    // Perform I/O: clean up symlink and MCP config
    let warnings = perform_disable_io(&external_path);

    let message = if warnings.is_empty() {
        format!("Agent disabled for project at {}.", external_path)
    } else {
        format!(
            "Agent disabled but some cleanup failed: {}",
            warnings.join("; ")
        )
    };

    Ok(AgentDisableResult {
        agent_block_id,
        status: AgentStatus::Disabled,
        message,
        warnings,
    })
}
