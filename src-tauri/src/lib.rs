pub mod capabilities;
pub mod cli;
pub mod commands;
pub mod config;
pub mod elf_project;
pub mod engine;
pub mod events;
pub mod extensions;
pub mod mcp;
pub mod models;
pub mod services;
pub mod state;
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

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
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
            .events(tauri_specta::collect_events![events::StateChangedEvent,])
            .commands(tauri_specta::collect_commands![
                // File operations
                commands::file::create_file,
                commands::file::open_file,
                commands::file::close_file,
                commands::file::list_open_files,
                commands::file::get_all_events,
                commands::file::get_file_info,
                commands::file::rename_file,
                commands::file::get_system_editor_id_from_config,
                // Event operations (Timeline feature)
                commands::event::get_state_at_event,
                // Block operations (core)
                commands::block::execute_command,
                commands::block::get_block,
                commands::block::get_all_blocks,
                commands::block::rename_block,
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
            ])
            // Core payload types (used by builtin capabilities)
            .typ::<models::CreateBlockPayload>()
            .typ::<models::LinkBlockPayload>()
            .typ::<models::UnlinkBlockPayload>()
            .typ::<models::GrantPayload>()
            .typ::<models::RevokePayload>()
            .typ::<models::WriteBlockPayload>()
            .typ::<models::EditorCreatePayload>()
            .typ::<models::EditorDeletePayload>()
            // Extension payload types
            .typ::<extensions::document::DocumentWritePayload>()
            .typ::<extensions::document::DocumentReadPayload>()
            .typ::<extensions::task::TaskWritePayload>()
            .typ::<extensions::task::TaskReadPayload>()
            .typ::<extensions::task::TaskCommitPayload>()
            .typ::<extensions::session::SessionAppendPayload>()
            // File metadata types
            .typ::<commands::FileMetadata>()
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
        commands::file::close_file,
        commands::file::list_open_files,
        commands::file::get_all_events,
        commands::file::get_file_info,
        commands::file::rename_file,
        commands::file::get_system_editor_id_from_config,
        // Event operations (Timeline feature)
        commands::event::get_state_at_event,
        // Block operations (core)
        commands::block::execute_command,
        commands::block::get_block,
        commands::block::get_all_blocks,
        commands::block::rename_block,
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
    ]);

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
