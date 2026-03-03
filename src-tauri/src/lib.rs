pub mod capabilities;
pub mod commands;
pub mod config;
pub mod elf;
pub mod engine;
pub mod events;
pub mod extensions;
pub mod mcp;
pub mod models;
pub mod state;
pub mod sync;
pub mod utils;

use state::AppState;
use std::sync::Arc;
use tauri::Manager;
use tauri_specta::Event;

#[cfg(debug_assertions)]
use specta_typescript::{BigIntExportBehavior, Typescript};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState::new();
    // TerminalState shares the same sessions Arc as AppState,
    // so both Tauri commands and MCP server access the same terminal sessions.
    let terminal_state = extensions::terminal::TerminalState {
        sessions: app_state.terminal_sessions.clone(),
    };

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        .manage(terminal_state)
        .setup(|app| {
            let app_state: tauri::State<AppState> = app.state();

            // Subscribe to state_changed broadcast and forward to frontend via typed Tauri events
            let app_handle = app.handle().clone();
            let mut state_rx = app_state.state_changed_tx.subscribe();
            tauri::async_runtime::spawn(async move {
                while let Ok(file_id) = state_rx.recv().await {
                    let _ = events::StateChangedEvent { file_id }.emit(&app_handle);
                }
            });

            // Start AgentSyncObserver (manages session sync lifecycle autonomously)
            let sync_observer_state = (*app_state).clone();
            crate::sync::observer::AgentSyncObserver::spawn(sync_observer_state);

            // Start MCP Server (independent port, background task)
            let mcp_state = Arc::new((*app_state).clone());

            tauri::async_runtime::spawn(async move {
                let port = mcp::MCP_PORT;
                if let Err(e) = mcp::start_mcp_server(mcp_state, port).await {
                    eprintln!("MCP Server error: {}", e);
                    // MCP startup failure does not block GUI
                }
            });

            Ok(())
        });

    // Generate TypeScript bindings in debug mode
    #[cfg(debug_assertions)]
    let builder = {
        let specta_builder = tauri_specta::Builder::<tauri::Wry>::new()
            .events(tauri_specta::collect_events![
                events::StateChangedEvent,
                events::PtyOutputEvent,
            ])
            .commands(tauri_specta::collect_commands![
                // File operations
                commands::file::create_file,
                commands::file::open_file,
                commands::file::save_file,
                commands::file::close_file,
                commands::file::list_open_files,
                commands::file::get_all_events,
                commands::file::get_file_info,
                commands::file::rename_file,
                commands::file::duplicate_file,
                commands::file::get_system_editor_id_from_config,
                // Event operations (Timeline feature)
                commands::event::get_state_at_event,
                // Block operations (core)
                commands::block::execute_command,
                commands::block::get_block,
                commands::block::get_all_blocks,
                commands::block::update_block_metadata,
                commands::block::rename_block,
                commands::block::change_block_type,
                commands::block::check_permission,
                // Editor operations
                commands::editor::create_editor,
                commands::editor::delete_editor,
                commands::editor::list_editors,
                commands::editor::get_editor,
                commands::editor::set_active_editor,
                commands::editor::get_active_editor,
                // Grant operations
                commands::editor::list_grants,
                commands::editor::get_block_grants,
                // Workspace/Checkout operations
                commands::checkout::checkout_workspace,
                // Task operations (Split Pattern: I/O side)
                commands::task::commit_task,
                commands::task::inject_hooks_for_repo,
                commands::task::remove_hooks_for_repo,
                commands::task::is_hooks_active,
                // Terminal operations (from extensions/terminal/commands.rs)
                // Note: These are high-frequency "patch" operations that don't record Events.
                // Event-producing operations use capabilities via execute_command.
                extensions::terminal::commands::init_pty_session,
                extensions::terminal::commands::write_to_pty,
                extensions::terminal::commands::resize_pty,
                extensions::terminal::commands::close_pty_session,
                // Agent operations (Phase 2)
                commands::agent::agent_create,
                commands::agent::agent_enable,
                commands::agent::agent_disable,
            ])
            // Explicitly export payload types for frontend type generation
            // These types are used inside Command.payload but not in Tauri command signatures,
            // so specta cannot automatically discover them. We must register them manually.
            // NOTE: When adding a new extension with payload types, register them here.
            // TODO: Consider automating this with a macro if extensions grow beyond ~10
            // Core payload types (used by builtin capabilities)
            .typ::<extensions::task::TaskCommitPayload>()
            .typ::<extensions::task::TaskReadPayload>()
            .typ::<extensions::task::TaskWritePayload>()
            .typ::<commands::task::TaskCommitResult>()
            .typ::<extensions::code::CodeWritePayload>()
            .typ::<extensions::code::CodeReadPayload>()
            .typ::<extensions::directory::DirectoryRenamePayload>()
            .typ::<extensions::directory::DirectoryRenameWithTypeChangePayload>()
            .typ::<extensions::directory::DirectoryDeletePayload>()
            .typ::<extensions::directory::DirectoryCreatePayload>()
            .typ::<extensions::directory::DirectoryExportPayload>()
            .typ::<extensions::directory::DirectoryImportPayload>()
            .typ::<extensions::directory::DirectoryWritePayload>()
            .typ::<models::CreateBlockPayload>()
            .typ::<models::LinkBlockPayload>()
            .typ::<models::UnlinkBlockPayload>()
            .typ::<models::GrantPayload>()
            .typ::<models::RevokePayload>()
            .typ::<models::UpdateMetadataPayload>()
            .typ::<models::EditorCreatePayload>()
            .typ::<models::EditorDeletePayload>()
            // Extension payload types
            .typ::<extensions::markdown::MarkdownWritePayload>()
            .typ::<extensions::terminal::TerminalSavePayload>()
            .typ::<extensions::terminal::TerminalExecutePayload>()
            .typ::<extensions::terminal::TerminalInitPayload>()
            // Agent extension types (Phase 2)
            .typ::<extensions::agent::AgentCreatePayload>()
            .typ::<extensions::agent::AgentEnablePayload>()
            .typ::<extensions::agent::AgentDisablePayload>()
            .typ::<extensions::agent::AgentContents>()
            .typ::<extensions::agent::AgentStatus>()
            .typ::<extensions::agent::AgentCreateResult>()
            .typ::<extensions::agent::AgentEnableResult>()
            .typ::<extensions::agent::AgentDisableResult>()
            // File metadata types
            .typ::<commands::FileMetadata>()
            // Block metadata types
            .typ::<models::BlockMetadata>()
            // Event types
            .typ::<commands::event::StateSnapshot>();

        // Export TypeScript bindings on app startup
        #[cfg(debug_assertions)]
        specta_builder
            .export(
                Typescript::default()
                    .bigint(BigIntExportBehavior::Number)
                    .header("// @ts-nocheck"),
                "../src/bindings.ts",
            )
            .expect("Failed to export TypeScript bindings");

        builder.invoke_handler(specta_builder.invoke_handler())
    };

    // Use standard handler in release mode
    #[cfg(not(debug_assertions))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        // File operations
        commands::file::create_file,
        commands::file::open_file,
        commands::file::save_file,
        commands::file::close_file,
        commands::file::list_open_files,
        commands::file::get_all_events,
        commands::file::get_file_info,
        commands::file::rename_file,
        commands::file::duplicate_file,
        commands::file::get_system_editor_id_from_config,
        // Event operations (Timeline feature)
        commands::event::get_state_at_event,
        // Block operations (core)
        commands::block::execute_command,
        commands::block::get_block,
        commands::block::get_all_blocks,
        commands::block::update_block_metadata,
        commands::block::rename_block,
        commands::block::change_block_type,
        commands::block::check_permission,
        // Editor operations
        commands::editor::create_editor,
        commands::editor::delete_editor,
        commands::editor::list_editors,
        commands::editor::get_editor,
        commands::editor::set_active_editor,
        commands::editor::get_active_editor,
        // Grant operations
        commands::editor::list_grants,
        commands::editor::get_block_grants,
        // Workspace/Checkout operations
        commands::checkout::checkout_workspace,
        // Task operations (Split Pattern: I/O side)
        commands::task::commit_task,
        commands::task::inject_hooks_for_repo,
        commands::task::remove_hooks_for_repo,
        commands::task::is_hooks_active,
        // Terminal operations (from extensions/terminal/commands.rs)
        extensions::terminal::commands::init_pty_session,
        extensions::terminal::commands::write_to_pty,
        extensions::terminal::commands::resize_pty,
        extensions::terminal::commands::close_pty_session,
        // Agent operations (Phase 2)
        commands::agent::agent_create,
        commands::agent::agent_enable,
        commands::agent::agent_disable,
    ]);

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
