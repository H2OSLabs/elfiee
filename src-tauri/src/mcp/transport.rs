//! MCP Transport Layer
//!
//! Provides independent SSE servers for MCP protocol communication.
//!
//! **Management server** (port 47200): Legacy/fallback mode. Uses GUI active editor.
//! When all SSE clients disconnect, enabled agents are auto-disabled.
//!
//! **Per-agent servers** (ports 47201–47299): Each enabled agent gets its own port.
//! The agent's `editor_id` is used deterministically for all operations.
//! When the agent's last SSE client disconnects, only that agent is auto-disabled.

use super::ElfieeMcpServer;
use crate::extensions::agent::{AgentContents, AgentStatus};
use crate::state::{AgentServerHandle, AppState};
use rmcp::transport::sse_server::{SseServer, SseServerConfig};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// MCP SSE Server default port (management)
pub const MCP_PORT: u16 = 47200;

// ============================================================================
// Management MCP Server (port 47200)
// ============================================================================

/// Start the management MCP SSE Server.
///
/// Called during Tauri setup as a background task.
/// Uses `serve_with_config` for graceful shutdown support.
///
/// In management mode, `agent_block_id = None` so `resolve_agent_editor_id`
/// falls back to the GUI active editor.
pub async fn start_mcp_server(app_state: Arc<AppState>, port: u16) -> Result<(), String> {
    let ct = CancellationToken::new();
    let config = SseServerConfig {
        bind: SocketAddr::from(([127, 0, 0, 1], port)),
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: ct.clone(),
        sse_keep_alive: Some(Duration::from_secs(30)),
    };

    let mut sse_server = SseServer::serve_with_config(config)
        .await
        .map_err(|e| format!("MCP: Failed to bind on port {}: {}", port, e))?;

    println!(
        "MCP Management Server listening on http://127.0.0.1:{}",
        port
    );
    println!("  GET  /sse      - SSE connection");
    println!("  POST /message  - MCP messages");

    tokio::spawn(async move {
        use rmcp::service::ServiceExt;

        while let Some(transport) = sse_server.next_transport().await {
            let app_state = app_state.clone();
            let ct = ct.child_token();

            let count = app_state
                .sse_connection_count
                .fetch_add(1, Ordering::SeqCst)
                + 1;
            println!("MCP: Client connected (active: {})", count);

            tokio::spawn(async move {
                let result = async {
                    let service = ElfieeMcpServer::new(app_state.clone(), None);
                    let server = service
                        .serve_with_ct(transport, ct)
                        .await
                        .map_err(std::io::Error::other)?;
                    server.waiting().await?;
                    tokio::io::Result::Ok(())
                }
                .await;

                if let Err(e) = result {
                    eprintln!("MCP: Connection error: {}", e);
                }

                let remaining = app_state
                    .sse_connection_count
                    .fetch_sub(1, Ordering::SeqCst)
                    - 1;
                println!("MCP: Client disconnected (active: {})", remaining);

                if remaining == 0 {
                    disable_all_agents(&app_state).await;
                }
            });
        }
    });

    Ok(())
}

// ============================================================================
// Per-Agent MCP Server (ports 47201–47299)
// ============================================================================

/// Allocate the next available port for an agent MCP server.
///
/// Ports are allocated sequentially from 47201. Range: 47201–47299.
fn allocate_agent_port(app_state: &AppState) -> Result<u16, String> {
    loop {
        let port = app_state.next_agent_port.fetch_add(1, Ordering::SeqCst);
        if port > 47299 {
            return Err("Agent port range exhausted (47201-47299)".to_string());
        }
        // Skip ports already in use by another agent
        if !app_state
            .agent_servers
            .iter()
            .any(|e| e.value().port == port)
        {
            return Ok(port);
        }
    }
}

/// Start a per-agent MCP server on a dedicated port.
///
/// Each agent gets its own SSE server. The `agent_block_id` is bound to the
/// `ElfieeMcpServer` instance so `resolve_agent_editor_id` returns this
/// agent's editor deterministically.
///
/// Returns the allocated port on success.
pub async fn start_agent_mcp_server(
    app_state: Arc<AppState>,
    agent_block_id: &str,
) -> Result<u16, String> {
    // Check if server already running for this agent
    if app_state.agent_servers.contains_key(agent_block_id) {
        let existing = app_state.agent_servers.get(agent_block_id).unwrap();
        return Ok(existing.port);
    }

    // Allocate port with retry on bind failure
    let mut last_err = String::new();
    for _ in 0..5 {
        let port = allocate_agent_port(&app_state)?;

        match try_start_agent_server(app_state.clone(), agent_block_id.to_string(), port).await {
            Ok(handle) => {
                let allocated_port = handle.port;
                app_state
                    .agent_servers
                    .insert(agent_block_id.to_string(), handle);
                println!(
                    "MCP Agent {}: Server started on port {}",
                    agent_block_id, allocated_port
                );
                return Ok(allocated_port);
            }
            Err(e) if e.contains("bind") || e.contains("address") => {
                last_err = e;
                continue; // Port in use, try next
            }
            Err(e) => return Err(e),
        }
    }
    Err(format!(
        "Failed to allocate port for agent MCP server after 5 attempts: {}",
        last_err
    ))
}

/// Try to start an agent MCP server on a specific port.
async fn try_start_agent_server(
    app_state: Arc<AppState>,
    agent_block_id: String,
    port: u16,
) -> Result<AgentServerHandle, String> {
    let ct = CancellationToken::new();
    let config = SseServerConfig {
        bind: SocketAddr::from(([127, 0, 0, 1], port)),
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: ct.clone(),
        sse_keep_alive: Some(Duration::from_secs(30)),
    };

    let mut sse_server = SseServer::serve_with_config(config)
        .await
        .map_err(|e| format!("Failed to bind agent MCP on port {}: {}", port, e))?;

    let sse_count = Arc::new(AtomicUsize::new(0));
    let sse_count_clone = sse_count.clone();
    let ct_clone = ct.clone();
    let agent_id_clone = agent_block_id.clone();

    tokio::spawn(async move {
        use rmcp::service::ServiceExt;

        while let Some(transport) = sse_server.next_transport().await {
            let app_state = app_state.clone();
            let agent_id = agent_id_clone.clone();
            let sse_count = sse_count_clone.clone();
            let child_ct = ct_clone.child_token();

            let count = sse_count.fetch_add(1, Ordering::SeqCst) + 1;
            println!(
                "MCP Agent {}: Client connected (active: {})",
                agent_id, count
            );

            tokio::spawn(async move {
                let result = async {
                    let service = ElfieeMcpServer::new(app_state.clone(), Some(agent_id.clone()));
                    let server = service
                        .serve_with_ct(transport, child_ct)
                        .await
                        .map_err(std::io::Error::other)?;
                    server.waiting().await?;
                    tokio::io::Result::Ok(())
                }
                .await;

                if let Err(e) = result {
                    eprintln!("MCP Agent {}: Connection error: {}", agent_id, e);
                }

                let remaining = sse_count.fetch_sub(1, Ordering::SeqCst) - 1;
                println!(
                    "MCP Agent {}: Client disconnected (active: {})",
                    agent_id, remaining
                );

                // When last client disconnects, auto-disable only this agent
                if remaining == 0 {
                    disable_single_agent(&app_state, &agent_id).await;
                }
            });
        }
    });

    Ok(AgentServerHandle {
        port,
        agent_block_id,
        cancel_token: ct,
        sse_count,
    })
}

/// Stop a per-agent MCP server and clean up.
///
/// Cancels the server's token, waits for connections to drain (max 5s),
/// then removes from the agent_servers map.
pub async fn stop_agent_mcp_server(
    app_state: &AppState,
    agent_block_id: &str,
) -> Result<(), String> {
    let handle = match app_state.agent_servers.remove(agent_block_id) {
        Some((_, handle)) => handle,
        None => return Ok(()), // Not running, nothing to stop
    };

    // Trigger cancellation
    handle.cancel_token.cancel();

    // Wait for connection cleanup (max 5 seconds)
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while handle.sse_count.load(Ordering::SeqCst) > 0 {
        if tokio::time::Instant::now() > deadline {
            println!(
                "MCP Agent {}: Force shutdown (connections remaining: {})",
                agent_block_id,
                handle.sse_count.load(Ordering::SeqCst)
            );
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    println!(
        "MCP Agent {}: Server stopped on port {}",
        agent_block_id, handle.port
    );
    Ok(())
}

// ============================================================================
// Auto-Disable Handlers
// ============================================================================

/// Disable a single agent when its per-agent MCP server loses all SSE clients.
async fn disable_single_agent(app_state: &AppState, agent_block_id: &str) {
    println!(
        "MCP Agent {}: All clients disconnected — auto-disabling...",
        agent_block_id
    );

    // Find which file this agent belongs to
    for (file_id, _path) in app_state.list_open_files() {
        let handle = match app_state.engine_manager.get_engine(&file_id) {
            Some(h) => h,
            None => continue,
        };

        if let Some(block) = handle.get_block(agent_block_id.to_string()).await {
            if block.block_type != "agent" {
                continue;
            }

            let editor_id = match app_state.get_active_editor(&file_id) {
                Some(e) => e,
                None => continue,
            };

            match crate::commands::agent::do_agent_disable(
                app_state,
                &file_id,
                &editor_id,
                agent_block_id,
            )
            .await
            {
                Ok(_) => println!("MCP: Auto-disabled agent '{}'", block.name),
                Err(e) => eprintln!("MCP: Failed to auto-disable agent '{}': {}", block.name, e),
            }
            return;
        }
    }

    eprintln!(
        "MCP: Could not find agent block {} in any open file for auto-disable",
        agent_block_id
    );
}

/// Disable all enabled agent blocks across all open files.
///
/// Called when the management port's last SSE client disconnects.
async fn disable_all_agents(app_state: &AppState) {
    println!("MCP: All management clients disconnected — disabling enabled agents...");

    for (file_id, _path) in app_state.list_open_files() {
        let handle = match app_state.engine_manager.get_engine(&file_id) {
            Some(h) => h,
            None => continue,
        };
        let editor_id = match app_state.get_active_editor(&file_id) {
            Some(e) => e,
            None => continue,
        };

        let blocks = handle.get_all_blocks().await;
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

            match crate::commands::agent::do_agent_disable(
                app_state,
                &file_id,
                &editor_id,
                &block.block_id,
            )
            .await
            {
                Ok(_) => println!("MCP: Auto-disabled agent '{}'", block.name),
                Err(e) => eprintln!("MCP: Failed to auto-disable agent '{}': {}", block.name, e),
            }
        }
    }
}
