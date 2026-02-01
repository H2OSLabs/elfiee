//! MCP Transport Layer
//!
//! Provides an independent SSE server for MCP protocol communication.
//! The MCP server runs on its own port (47200), separate from the Tauri app.
//!
//! When all SSE clients disconnect, enabled agent blocks are automatically
//! disabled to keep agent status in sync with actual connectivity.

use super::ElfieeMcpServer;
use crate::commands::agent::{get_external_path, perform_disable_io};
use crate::extensions::agent::{AgentContents, AgentStatus};
use crate::models::Command;
use crate::state::AppState;
use rmcp::transport::sse_server::{SseServer, SseServerConfig};
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// MCP SSE Server default port
pub const MCP_PORT: u16 = 47200;

/// Start an independent MCP SSE Server.
///
/// Called during Tauri setup as a background task.
/// The MCP server shares AppState with the GUI (same process).
///
/// Each SSE connection is tracked. When all clients disconnect,
/// enabled agent blocks are automatically disabled.
pub async fn start_mcp_server(app_state: Arc<AppState>, port: u16) -> Result<(), String> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let config = SseServerConfig {
        bind: addr,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: CancellationToken::new(),
        sse_keep_alive: Some(Duration::from_secs(30)),
    };

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("MCP: Failed to bind to port {}: {}", port, e))?;

    println!("MCP Server listening on http://{}", addr);
    println!("  GET  /sse      - SSE connection");
    println!("  POST /message  - MCP messages");

    let (mut sse_server, router) = SseServer::new(config);

    // Spawn the connection acceptor with lifecycle tracking.
    // Instead of using sse_server.with_service() (fire-and-forget),
    // we manually loop over incoming transports so we can hook into
    // connection close events and run agent cleanup.
    let ct = sse_server.config.ct.clone();
    tokio::spawn(async move {
        use rmcp::service::ServiceExt;

        while let Some(transport) = sse_server.next_transport().await {
            let app_state = app_state.clone();
            let ct = ct.child_token();

            // Track new connection
            let count = app_state
                .sse_connection_count
                .fetch_add(1, Ordering::SeqCst)
                + 1;
            println!("MCP: Client connected (active: {})", count);

            tokio::spawn(async move {
                // Serve the MCP connection
                let result = async {
                    let service = ElfieeMcpServer::new(app_state.clone());
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

                // Connection closed — decrement counter
                let remaining = app_state
                    .sse_connection_count
                    .fetch_sub(1, Ordering::SeqCst)
                    - 1;
                println!("MCP: Client disconnected (active: {})", remaining);

                // When last connection closes, auto-disable all enabled agents
                if remaining == 0 {
                    disable_all_agents(&app_state).await;
                }
            });
        }
    });

    axum::serve(listener, router)
        .await
        .map_err(|e| format!("MCP Server error: {}", e))?;

    Ok(())
}

/// Disable all enabled agent blocks across all open files.
///
/// Called when the last SSE client disconnects. For each enabled agent:
/// 1. Sends `agent.disable` command to the engine (updates block state)
/// 2. Performs I/O cleanup (removes symlink + MCP config)
async fn disable_all_agents(app_state: &AppState) {
    println!("MCP: All clients disconnected — disabling enabled agents...");

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

            // 1. Send disable command to engine
            let cmd = Command::new(
                editor_id.clone(),
                "agent.disable".to_string(),
                block.block_id.clone(),
                serde_json::json!({}),
            );

            if let Err(e) = handle.process_command(cmd).await {
                eprintln!("MCP: Failed to auto-disable agent '{}': {}", block.name, e);
                continue;
            }

            // 2. Perform I/O cleanup (remove symlink + MCP config)
            if let Some(target_block) = handle.get_block(contents.target_project_id.clone()).await {
                if let Ok(external_path) = get_external_path(&target_block) {
                    let warnings = perform_disable_io(&external_path);
                    for w in &warnings {
                        eprintln!("MCP: Auto-disable I/O warning for '{}': {}", block.name, w);
                    }
                }
            }

            println!("MCP: Auto-disabled agent '{}'", block.name);
        }
    }
}
