//! Typed Tauri events for frontend push notifications.
//!
//! All events emitted from the Rust backend to the frontend are defined here.
//! These use tauri-specta's typed event system for auto-generated TypeScript bindings.
//!
//! Registered in `lib.rs` via `collect_events![]` and consumed in the frontend
//! via the `events` export from `@/bindings`.
//!
//! ## Events
//!
//! - [`StateChangedEvent`] — Backend state changed (blocks/grants modified via MCP or Tauri commands)
//! - [`PtyOutputEvent`] — Terminal PTY output (high-frequency, from reader thread)

use serde::{Deserialize, Serialize};

/// Emitted when backend state changes (e.g., blocks modified via MCP or Tauri commands).
///
/// The frontend auto-refreshes blocks, grants, and events for the affected file.
/// Sent through the `state_changed_tx` broadcast channel by both Tauri commands
/// and the MCP server after successful command processing.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct StateChangedEvent {
    pub file_id: String,
}

/// Emitted when PTY produces output (high-frequency, from reader thread).
///
/// The frontend terminal (xterm.js) decodes the base64 data and writes it to the screen.
/// Only emitted by GUI-initiated PTY sessions (not MCP-initiated sessions which
/// only write to the output buffer).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct PtyOutputEvent {
    /// Base64 encoded output data
    pub data: String,
    /// The terminal block ID
    pub block_id: String,
}
