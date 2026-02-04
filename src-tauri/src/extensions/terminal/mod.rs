//! Terminal Extension
//!
//! PTY-based terminal implementation using portable-pty.
//!
//! ## Architecture
//!
//! The terminal extension follows Elfiee's split architecture:
//!
//! ### State Management (state.rs)
//! - `TerminalState` - Global state for managing all terminal sessions
//! - `TerminalSession` - Individual PTY session with writer and master handles
//!
//! ### Pure PTY Utilities (utils/pty.rs)
//! - `spawn()` - Create PTY and spawn shell
//! - `write()` - Write data to PTY
//! - `resize()` - Resize PTY window
//! - `close()` - Close PTY session
//!
//! ### Capabilities (produce Events)
//! - `terminal_init.rs` - terminal.init capability (session started event)
//! - `terminal_execute.rs` - terminal.execute capability (command executed event)
//! - `terminal_save.rs` - terminal.save capability (content saved event)
//! - `terminal_close.rs` - terminal.close capability (session closed event)
//!
//! ### Tauri Commands (commands.rs)
//! Real-time PTY operations (patches, don't record Events):
//! - `write_to_pty` - Write user input to PTY
//! - `resize_pty` - Resize PTY window
//! - `close_pty_session` - Close PTY session
//!
//! ## Payload Types
//!
//! - `TerminalInitPayload` - Parameters for PTY initialization
//! - `TerminalExecutePayload` - Parameters for command execution
//! - `TerminalSavePayload` - Parameters for content save

use serde::{Deserialize, Serialize};
use specta::Type;

// ============================================================================
// Module Exports
// ============================================================================

pub mod commands;
pub mod pty;
pub mod state;
pub mod terminal_close;
pub mod terminal_execute;
pub mod terminal_init;
pub mod terminal_save;

// Re-export state management
pub use state::{TerminalSession, TerminalState};

// Re-export capabilities
pub use terminal_close::TerminalCloseCapability;
pub use terminal_execute::TerminalExecuteCapability;
pub use terminal_init::TerminalInitCapability;
pub use terminal_save::TerminalSaveCapability;

// Re-export Tauri commands
pub use commands::{close_pty_session, init_pty_session, resize_pty, write_to_pty};

// ============================================================================
// Payload Definitions
// ============================================================================

/// Payload for terminal.init capability
///
/// This payload is used to initialize a PTY session for a terminal block.
#[derive(Debug, Clone, Deserialize, Serialize, Type)]
pub struct TerminalInitPayload {
    /// Number of columns in the terminal
    pub cols: u16,
    /// Number of rows in the terminal
    pub rows: u16,
    /// The terminal block ID
    pub block_id: String,
    /// The editor initiating the session
    pub editor_id: String,
    /// The file containing the terminal block (required for permission checking)
    pub file_id: String,
    /// Optional initial working directory
    pub cwd: Option<String>,
}

/// Payload for terminal.execute capability
///
/// This payload records a command execution in the terminal for audit/history.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TerminalExecutePayload {
    /// The command string that was executed
    pub command: String,
    /// Exit code of the command (if available)
    pub exit_code: Option<i32>,
}

/// Payload for terminal.save capability
///
/// This payload is used to save terminal content (buffer snapshot) to a terminal block.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TerminalSavePayload {
    /// The terminal content to save (typically from xterm.js buffer)
    pub saved_content: String,
    /// Timestamp when the content was saved (ISO 8601 format)
    pub saved_at: String,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests;
