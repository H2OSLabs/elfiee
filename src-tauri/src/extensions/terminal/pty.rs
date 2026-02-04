//! PTY (Pseudo-Terminal) pure utility functions
//!
//! This module provides stateless PTY operations as pure functions.
//! All state management is delegated to the caller (extensions/terminal/state.rs).
//!
//! ## Design Philosophy
//!
//! These functions are pure utilities:
//! - No global state
//! - Take handles as parameters
//! - Return results without side effects on module state
//!
//! ## Usage
//!
//! ```rust,ignore
//! use crate::utils::pty::{spawn, write, resize, close, PtyHandle};
//!
//! // Spawn a new PTY
//! let handle = spawn(SpawnConfig { cols: 80, rows: 24, cwd: None, shell: None })?;
//!
//! // Write to PTY
//! write(&handle, "ls -la\n")?;
//!
//! // Resize PTY
//! resize(&handle, 120, 40)?;
//!
//! // Close PTY
//! close(handle)?;
//! ```

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::Write as IoWrite;
use std::path::Path;

// ============================================================================
// Types
// ============================================================================

/// Configuration for spawning a new PTY session.
#[derive(Debug, Clone)]
pub struct SpawnConfig<'a> {
    /// Number of columns
    pub cols: u16,
    /// Number of rows
    pub rows: u16,
    /// Working directory (optional)
    pub cwd: Option<&'a Path>,
    /// Shell override (optional, auto-detected if None)
    pub shell: Option<&'a str>,
    /// Command arguments (optional)
    pub args: Vec<String>,
}

/// Handle to an active PTY session.
///
/// This struct owns the PTY resources and must be stored by the caller.
/// When dropped, the PTY session is terminated.
pub struct PtyHandle {
    /// Writer for sending data to the PTY
    pub writer: Box<dyn IoWrite + Send>,
    /// Master PTY handle for resize operations
    pub master: Box<dyn MasterPty + Send>,
    /// Reader for receiving data from the PTY (for spawning reader thread)
    pub reader: Box<dyn std::io::Read + Send>,
}

// ============================================================================
// Pure Functions
// ============================================================================

/// Spawn a new PTY session with a shell process.
///
/// Creates a PTY, spawns the appropriate shell for the platform,
/// and returns a handle for further operations.
///
/// # Arguments
/// * `config` - Configuration for the PTY session
///
/// # Returns
/// * `Ok(PtyHandle)` - Handle to the new PTY session
/// * `Err(String)` - Error message if spawn fails
///
/// # Platform Behavior
/// - Unix: Spawns bash (or configured shell)
/// - Windows: Spawns PowerShell
pub fn spawn(config: SpawnConfig) -> Result<PtyHandle, String> {
    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows: config.rows,
            cols: config.cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("Failed to open PTY: {}", e))?;

    // Determine shell
    let shell = config.shell.unwrap_or(if cfg!(target_os = "windows") {
        "powershell"
    } else {
        "bash"
    });

    let mut cmd_builder = CommandBuilder::new(shell);

    // Add arguments
    if !config.args.is_empty() {
        cmd_builder.args(config.args);
    }

    // Set TERM environment variable for proper terminal emulation
    cmd_builder.env("TERM", "xterm-256color");

    // Set working directory if provided
    if let Some(cwd) = config.cwd {
        cmd_builder.cwd(cwd);
    }

    let _child = pair
        .slave
        .spawn_command(cmd_builder)
        .map_err(|e| format!("Failed to spawn shell: {}", e))?;

    // Release slave to allow it to close when child exits
    drop(pair.slave);

    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("Failed to clone reader: {}", e))?;

    let writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("Failed to take writer: {}", e))?;

    Ok(PtyHandle {
        writer,
        master: pair.master,
        reader,
    })
}

/// Write data to a PTY.
///
/// Sends data to the PTY's stdin, which will be processed by the shell.
///
/// # Arguments
/// * `writer` - Mutable reference to the PTY writer
/// * `data` - Data to write (typically user input)
///
/// # Returns
/// * `Ok(())` - Data written successfully
/// * `Err(String)` - Error if write fails
pub fn write(writer: &mut (dyn IoWrite + Send), data: &str) -> Result<(), String> {
    write!(writer, "{}", data).map_err(|e| format!("Failed to write to PTY: {}", e))
}

/// Resize a PTY.
///
/// Updates the PTY's window dimensions. This should be called when
/// the terminal viewport size changes.
///
/// # Arguments
/// * `master` - Reference to the PTY master
/// * `cols` - New number of columns
/// * `rows` - New number of rows
///
/// # Returns
/// * `Ok(())` - PTY resized successfully
/// * `Err(String)` - Error if resize fails
pub fn resize(master: &dyn MasterPty, cols: u16, rows: u16) -> Result<(), String> {
    master
        .resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("Failed to resize PTY: {}", e))
}

/// Close a PTY session.
///
/// This is a no-op function that documents the close operation.
/// The actual cleanup happens when the PtyHandle is dropped.
///
/// # Arguments
/// * `handle` - The PTY handle to close (consumed)
///
/// # Note
/// This function takes ownership of the handle, causing it to be dropped
/// and releasing all PTY resources.
pub fn close(handle: PtyHandle) {
    // Dropping the handle releases all resources
    drop(handle);
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_spawn_config_default() {
        let config = SpawnConfig {
            cols: 80,
            rows: 24,
            cwd: None,
            shell: None,
            args: vec![],
        };
        assert_eq!(config.cols, 80);
        assert_eq!(config.rows, 24);
        assert!(config.cwd.is_none());
        assert!(config.shell.is_none());
        assert!(config.args.is_empty());
    }

    #[test]
    fn test_spawn_config_with_cwd() {
        let path = PathBuf::from("/tmp/test");
        let config = SpawnConfig {
            cols: 120,
            rows: 40,
            cwd: Some(&path),
            shell: Some("bash"),
            args: vec!["-c".to_string(), "ls".to_string()],
        };
        assert_eq!(config.cols, 120);
        assert_eq!(config.rows, 40);
        assert!(config.cwd.is_some());
        assert_eq!(config.shell, Some("bash"));
        assert_eq!(config.args.len(), 2);
    }

    // Note: Integration tests for spawn/write/resize/close require
    // a real PTY environment and are tested in extensions/terminal/tests.rs
}
