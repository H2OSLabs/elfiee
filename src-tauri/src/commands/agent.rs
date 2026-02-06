//! Tauri commands for Agent operations
//!
//! These commands handle the I/O layer for agent operations:
//! - `agent_create`: Create Agent Block bound to .claude/ dir + auto-enable
//! - `agent_enable`: Re-enable agent (recreate symlink + MCP config)
//! - `agent_disable`: Disable agent (clean symlink + remove MCP config)
//!
//! Business logic is in `do_*` functions, shared between Tauri commands,
//! MCP server, and auto-disconnect handler (transport.rs).

use crate::capabilities::registry::CapabilityRegistry;
use crate::extensions::agent::{mcp_config, settings_config};
use crate::extensions::agent::{
    AgentContents, AgentCreatePayload, AgentCreateResult, AgentDisableResult, AgentEnableResult,
    AgentStatus,
};
use crate::models::Command;
use crate::state::AppState;
use std::path::Path;
use std::sync::Arc;
use tauri::State;

/// MCP server name used as the key in `.mcp.json` configuration files.
pub const MCP_SERVER_NAME: &str = "elfiee";

/// Create symlink from source to destination (cross-platform).
///
/// On Unix: creates a symbolic link.
/// On Windows: creates a directory junction (no admin privileges required).
fn create_symlink_dir(src: &Path, dst: &Path) -> Result<(), String> {
    // If existing symlink/junction already points to the correct target, skip recreation
    if let Ok(current_target) = dst.read_link() {
        if current_target == src {
            return Ok(());
        }
        // Points to wrong target — remove and recreate
        remove_symlink_dir(dst)?;
    } else if dst.exists() {
        // Exists but is not a symlink — remove
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
        // On Windows, use directory junction (no admin privileges required).
        junction::create(src, dst).map_err(|e| {
            format!(
                "Failed to create junction {} -> {}: {}",
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
        // On Windows, junctions are removed via remove_dir
        junction::delete(path)
            .or_else(|_| std::fs::remove_dir(path))
            .map_err(|e| format!("Failed to remove junction {}: {}", path.display(), e))?;
    }

    Ok(())
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

/// Derive the project root path from a config_dir path.
///
/// config_dir = "/home/user/repo-a/.claude" → project_root = "/home/user/repo-a"
fn get_project_root(config_dir: &str) -> Result<String, String> {
    Path::new(config_dir)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| format!("Cannot derive project root from config_dir: {}", config_dir))
}

/// Perform enable I/O: create symlink + merge MCP config.
///
/// `port` is the per-agent MCP server port used in the SSE URL.
///
/// Returns a list of warnings for partial failures.
fn perform_enable_io(config_dir: &str, elf_block_dir: &str, port: u16) -> (bool, Vec<String>) {
    let mut warnings = Vec::new();

    // 1. Create symlink: {elf_block_dir}/agents/elfiee-client/ -> {config_dir}/skills/elfiee-client/
    let symlink_src = Path::new(elf_block_dir)
        .join("agents")
        .join("elfiee-client");
    let symlink_dst = Path::new(config_dir).join("skills").join("elfiee-client");

    if let Err(e) = create_symlink_dir(&symlink_src, &symlink_dst) {
        warnings.push(format!("Failed to create symlink: {}", e));
    }

    // 2. Merge MCP config to both locations with the per-agent port:
    //    - {project_root}/.mcp.json: Claude Code's project-scope path
    //    - {config_dir}/mcp.json: fallback for compatibility
    let server_config = mcp_config::build_elfiee_server_config(port);

    let project_root = Path::new(config_dir)
        .parent()
        .unwrap_or(Path::new(config_dir));
    let mcp_project_path = project_root.join(".mcp.json");
    if let Err(e) =
        mcp_config::merge_server(&mcp_project_path, MCP_SERVER_NAME, server_config.clone())
    {
        warnings.push(format!("Failed to write .mcp.json: {}", e));
    }

    let mcp_claude_path = Path::new(config_dir).join("mcp.json");
    if let Err(e) = mcp_config::merge_server(&mcp_claude_path, MCP_SERVER_NAME, server_config) {
        warnings.push(format!("Failed to write .claude/mcp.json: {}", e));
    }

    // 3. Inject MCP tool auto-approve permissions into settings.local.json
    let settings_path = Path::new(config_dir).join("settings.local.json");
    if let Err(e) = settings_config::merge_allowed_tools(&settings_path) {
        warnings.push(format!("Failed to inject allowed tools: {}", e));
    }

    let success = warnings.is_empty();
    (success, warnings)
}

/// Perform disable I/O: remove symlink + remove MCP config.
///
/// Returns a list of warnings for partial failures.
pub(crate) fn perform_disable_io(config_dir: &str) -> Vec<String> {
    let mut warnings = Vec::new();

    // 1. Remove symlink
    let symlink_path = Path::new(config_dir).join("skills").join("elfiee-client");

    if let Err(e) = remove_symlink_dir(&symlink_path) {
        warnings.push(format!("Failed to remove symlink: {}", e));
    }

    // 2. Remove MCP config entry from both locations
    let project_root = Path::new(config_dir)
        .parent()
        .unwrap_or(Path::new(config_dir));
    let mcp_project_path = project_root.join(".mcp.json");
    if let Err(e) = mcp_config::remove_server(&mcp_project_path, MCP_SERVER_NAME) {
        warnings.push(format!("Failed to remove .mcp.json: {}", e));
    }

    let mcp_claude_path = Path::new(config_dir).join("mcp.json");
    if let Err(e) = mcp_config::remove_server(&mcp_claude_path, MCP_SERVER_NAME) {
        warnings.push(format!("Failed to remove .claude/mcp.json: {}", e));
    }

    // 3. Remove MCP tool auto-approve permissions from settings.local.json
    let settings_path = Path::new(config_dir).join("settings.local.json");
    if let Err(e) = settings_config::remove_allowed_tools(&settings_path) {
        warnings.push(format!("Failed to remove allowed tools: {}", e));
    }

    warnings
}

// ============================================================================
// Business Functions (shared by Tauri commands, MCP server, transport.rs)
// ============================================================================

/// Create an Agent Block bound to a .claude/ directory and auto-enable it.
///
/// Business logic shared between Tauri command and MCP server.
/// Validates config_dir, checks uniqueness, auto-creates bot editor,
/// creates agent block, performs I/O.
pub async fn do_agent_create(
    app_state: &AppState,
    file_id: &str,
    editor_id: &str,
    mut payload: AgentCreatePayload,
) -> Result<AgentCreateResult, String> {
    // 1. Get engine handle
    let handle = app_state
        .engine_manager
        .get_engine(file_id)
        .ok_or_else(|| format!("File '{}' is not open", file_id))?;

    // 2. Validate config_dir exists
    let config_dir = Path::new(&payload.config_dir);
    if !config_dir.exists() {
        return Err(format!(
            "Claude directory does not exist: {}. Run 'claude' in the project first.",
            payload.config_dir
        ));
    }

    // 3. Uniqueness check: no duplicate agent for the same config_dir
    let all_blocks = handle.get_all_blocks().await;
    for block in all_blocks.values() {
        if block.block_type == "agent" {
            if let Ok(contents) = serde_json::from_value::<AgentContents>(block.contents.clone()) {
                if contents.config_dir == payload.config_dir {
                    return Err(format!(
                        "Agent already exists for config_dir: {} (block_id: {})",
                        payload.config_dir, block.block_id
                    ));
                }
            }
        }
    }

    // 4. Auto-create bot editor if not provided
    if payload.editor_id.is_none()
        || payload
            .editor_id
            .as_ref()
            .is_some_and(|id| id.trim().is_empty())
    {
        let bot_name = payload.name.clone().unwrap_or_else(|| "elfiee".to_string());
        let bot_editor_id = format!("bot-{}", uuid::Uuid::new_v4());

        let create_editor_cmd = Command::new(
            editor_id.to_string(),
            "editor.create".to_string(),
            "".to_string(),
            serde_json::json!({
                "editor_id": bot_editor_id,
                "name": bot_name,
                "editor_type": "Bot"
            }),
        );
        handle.process_command(create_editor_cmd).await?;

        payload.editor_id = Some(bot_editor_id);
    }

    // 5. Create Agent Block via engine
    let cmd = Command::new(
        editor_id.to_string(),
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

    // 6. Auto wildcard grants for the agent's editor
    // Dynamically get all registered capabilities except owner-only ones.
    let agent_editor_id = payload.editor_id.as_ref().unwrap();
    let registry = CapabilityRegistry::new();
    let default_caps = registry.get_grantable_cap_ids(&[
        "core.grant",
        "core.revoke",
        "editor.create",
        "editor.delete",
    ]);

    for cap in &default_caps {
        let grant_cmd = Command::new(
            editor_id.to_string(),
            "core.grant".to_string(),
            "*".to_string(),
            serde_json::json!({
                "target_editor": agent_editor_id,
                "capability": cap,
                "target_block": "*"
            }),
        );
        // Best effort — don't fail agent creation if a grant fails
        if let Err(e) = handle.process_command(grant_cmd).await {
            log::warn!("Failed to grant {} to {}: {}", cap, agent_editor_id, e);
        }
    }

    // 7. Start per-agent MCP server
    let mcp_state = Arc::new(app_state.clone());
    let agent_port = crate::mcp::start_agent_mcp_server(mcp_state, &agent_block_id)
        .await
        .map_err(|e| format!("Agent created but MCP server failed: {}", e))?;

    // 8. Perform enable I/O with per-agent port
    // Re-fetch blocks since state may have changed after create
    let all_blocks = handle.get_all_blocks().await;
    let elf_block_dir = get_elf_block_dir(&all_blocks).ok_or_else(|| {
        ".elf/ directory block not found. Ensure .elf/ is initialized.".to_string()
    })?;

    let (io_success, warnings) = perform_enable_io(&payload.config_dir, &elf_block_dir, agent_port);

    let project_root = get_project_root(&payload.config_dir).unwrap_or_default();
    let message = if io_success {
        format!(
            "Agent '{}' created and enabled on port {} for project at {}. Please restart Claude Code to activate MCP.",
            payload.name.as_deref().unwrap_or("elfiee"),
            agent_port,
            project_root
        )
    } else {
        format!(
            "Agent created (port {}) but some I/O operations failed. Run agent.enable to retry. Warnings: {}",
            agent_port,
            warnings.join("; ")
        )
    };

    // 9. Notify sync observer of new enabled agent
    let _ =
        app_state
            .agent_sync_tx
            .send(crate::sync::observer::AgentSyncEvent::AgentStateChanged {
                file_id: file_id.to_string(),
                agent_block_id: agent_block_id.clone(),
                new_status: AgentStatus::Enabled,
                config_dir: payload.config_dir.clone(),
            });

    Ok(AgentCreateResult {
        agent_block_id,
        status: AgentStatus::Enabled,
        needs_restart: io_success,
        message,
    })
}

/// Enable an Agent Block: recreate symlink and inject MCP config.
///
/// Business logic shared between Tauri command and MCP server.
/// Idempotent: can be called on an already-enabled agent to refresh configuration.
pub async fn do_agent_enable(
    app_state: &AppState,
    file_id: &str,
    editor_id: &str,
    agent_block_id: &str,
) -> Result<AgentEnableResult, String> {
    let handle = app_state
        .engine_manager
        .get_engine(file_id)
        .ok_or_else(|| format!("File '{}' is not open", file_id))?;

    // Get Agent Block
    let agent_block = handle
        .get_block(agent_block_id.to_string())
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

    // Process enable command via engine
    let cmd = Command::new(
        editor_id.to_string(),
        "agent.enable".to_string(),
        agent_block_id.to_string(),
        serde_json::json!({}),
    );

    handle.process_command(cmd).await?;

    // Start per-agent MCP server
    let mcp_state = Arc::new(app_state.clone());
    let agent_port = crate::mcp::start_agent_mcp_server(mcp_state, agent_block_id)
        .await
        .map_err(|e| format!("Agent enabled but MCP server failed: {}", e))?;

    // Perform I/O with per-agent port
    let all_blocks = handle.get_all_blocks().await;
    let elf_block_dir = get_elf_block_dir(&all_blocks).ok_or_else(|| {
        ".elf/ directory block not found. Ensure .elf/ is initialized.".to_string()
    })?;

    let (io_success, warnings) =
        perform_enable_io(&contents.config_dir, &elf_block_dir, agent_port);

    let project_root = get_project_root(&contents.config_dir).unwrap_or_default();
    let message = if io_success {
        format!(
            "Agent enabled on port {} for project at {}. Please restart Claude Code to activate MCP.",
            agent_port, project_root
        )
    } else {
        format!(
            "Agent enabled (port {}) but some I/O operations failed: {}",
            agent_port,
            warnings.join("; ")
        )
    };

    // Notify sync observer of agent enabled
    let _ =
        app_state
            .agent_sync_tx
            .send(crate::sync::observer::AgentSyncEvent::AgentStateChanged {
                file_id: file_id.to_string(),
                agent_block_id: agent_block_id.to_string(),
                new_status: AgentStatus::Enabled,
                config_dir: contents.config_dir.clone(),
            });

    Ok(AgentEnableResult {
        agent_block_id: agent_block_id.to_string(),
        status: AgentStatus::Enabled,
        needs_restart: io_success,
        message,
        warnings,
    })
}

/// Disable an Agent Block: remove symlink and MCP config.
///
/// Business logic shared between Tauri command, MCP server, and
/// auto-disconnect handler (transport.rs).
/// Idempotent: can be called on an already-disabled agent.
pub async fn do_agent_disable(
    app_state: &AppState,
    file_id: &str,
    editor_id: &str,
    agent_block_id: &str,
) -> Result<AgentDisableResult, String> {
    let handle = app_state
        .engine_manager
        .get_engine(file_id)
        .ok_or_else(|| format!("File '{}' is not open", file_id))?;

    // Get Agent Block
    let agent_block = handle
        .get_block(agent_block_id.to_string())
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

    // Process disable command via engine
    let cmd = Command::new(
        editor_id.to_string(),
        "agent.disable".to_string(),
        agent_block_id.to_string(),
        serde_json::json!({}),
    );

    handle.process_command(cmd).await?;

    // Stop per-agent MCP server
    crate::mcp::stop_agent_mcp_server(app_state, agent_block_id)
        .await
        .unwrap_or_else(|e| eprintln!("Warning: Failed to stop agent MCP server: {}", e));

    // Notify sync observer of agent disabled
    let _ =
        app_state
            .agent_sync_tx
            .send(crate::sync::observer::AgentSyncEvent::AgentStateChanged {
                file_id: file_id.to_string(),
                agent_block_id: agent_block_id.to_string(),
                new_status: AgentStatus::Disabled,
                config_dir: contents.config_dir.clone(),
            });

    // Perform I/O: clean up symlink and MCP config
    let warnings = perform_disable_io(&contents.config_dir);

    let project_root = get_project_root(&contents.config_dir).unwrap_or_default();
    let message = if warnings.is_empty() {
        format!("Agent disabled for project at {}.", project_root)
    } else {
        format!(
            "Agent disabled but some cleanup failed: {}",
            warnings.join("; ")
        )
    };

    Ok(AgentDisableResult {
        agent_block_id: agent_block_id.to_string(),
        status: AgentStatus::Disabled,
        message,
        warnings,
    })
}

// ============================================================================
// Recovery: Restore MCP servers for enabled agents
// ============================================================================

/// Recover per-agent MCP servers for all enabled agents in a file.
///
/// Called when a file is opened. For each agent block with status=Enabled:
/// 1. Allocate a new port (ports don't persist across restarts)
/// 2. Start per-agent MCP server
/// 3. Update .mcp.json with the new port
/// 4. Refresh symlink (idempotent)
///
/// Session sync is handled separately by the AgentSyncObserver via FileOpened event.
///
/// Returns a list of (agent_name, error_message) for agents that failed to recover.
/// Successful recoveries are logged to stdout.
pub async fn recover_agent_servers(app_state: &AppState, file_id: &str) -> Vec<(String, String)> {
    let mut failures: Vec<(String, String)> = Vec::new();

    let handle = match app_state.engine_manager.get_engine(file_id) {
        Some(h) => h,
        None => return failures,
    };

    let blocks = handle.get_all_blocks().await;
    let elf_block_dir = get_elf_block_dir(&blocks);

    for block in blocks.values() {
        if block.block_type != "agent" {
            continue;
        }

        let contents: AgentContents = match serde_json::from_value(block.contents.clone()) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if contents.status != AgentStatus::Enabled {
            continue;
        }

        // Start per-agent MCP server
        let mcp_state = Arc::new(app_state.clone());
        match crate::mcp::start_agent_mcp_server(mcp_state, &block.block_id).await {
            Ok(port) => {
                // Update .mcp.json with new port
                if let Some(ref elf_dir) = elf_block_dir {
                    let (_, warnings) = perform_enable_io(&contents.config_dir, elf_dir, port);
                    if !warnings.is_empty() {
                        let warning_msg = warnings.join("; ");
                        eprintln!(
                            "Agent recovery '{}': I/O warnings: {}",
                            block.name, warning_msg
                        );
                        failures
                            .push((block.name.clone(), format!("I/O warnings: {}", warning_msg)));
                    }
                }
                println!("Agent recovery: Restored '{}' on port {}", block.name, port);
            }
            Err(e) => {
                eprintln!(
                    "Agent recovery: Failed to start MCP server for '{}': {}",
                    block.name, e
                );
                failures.push((block.name.clone(), e));
            }
        }
    }

    failures
}

/// Stop all per-agent MCP servers for agents in a specific file.
///
/// Called when a file is closed. Finds all agent servers belonging to
/// this file and shuts them down.
/// Session sync shutdown is handled by the AgentSyncObserver via FileClosing event.
pub async fn shutdown_agent_servers(app_state: &AppState, file_id: &str) {
    let handle = match app_state.engine_manager.get_engine(file_id) {
        Some(h) => h,
        None => return,
    };

    let blocks = handle.get_all_blocks().await;
    for block in blocks.values() {
        if block.block_type != "agent" {
            continue;
        }
        if let Err(e) = crate::mcp::stop_agent_mcp_server(app_state, &block.block_id).await {
            eprintln!(
                "Failed to stop agent MCP server for '{}': {}",
                block.name, e
            );
        }
    }
}

// ============================================================================
// Tauri Commands (thin wrappers around business functions)
// ============================================================================

/// Create an Agent Block bound to a .claude/ directory and auto-enable it.
#[tauri::command]
#[specta::specta]
pub async fn agent_create(
    state: State<'_, AppState>,
    file_id: String,
    payload: AgentCreatePayload,
) -> Result<AgentCreateResult, String> {
    let editor_id = state
        .get_active_editor(&file_id)
        .ok_or_else(|| "No active editor set for this file".to_string())?;
    do_agent_create(&state, &file_id, &editor_id, payload).await
}

/// Enable an Agent Block: recreate symlink and inject MCP config.
#[tauri::command]
#[specta::specta]
pub async fn agent_enable(
    state: State<'_, AppState>,
    file_id: String,
    agent_block_id: String,
) -> Result<AgentEnableResult, String> {
    let editor_id = state
        .get_active_editor(&file_id)
        .ok_or_else(|| "No active editor set for this file".to_string())?;
    do_agent_enable(&state, &file_id, &editor_id, &agent_block_id).await
}

/// Disable an Agent Block: remove symlink and MCP config.
#[tauri::command]
#[specta::specta]
pub async fn agent_disable(
    state: State<'_, AppState>,
    file_id: String,
    agent_block_id: String,
) -> Result<AgentDisableResult, String> {
    let editor_id = state
        .get_active_editor(&file_id)
        .ok_or_else(|| "No active editor set for this file".to_string())?;
    do_agent_disable(&state, &file_id, &editor_id, &agent_block_id).await
}
