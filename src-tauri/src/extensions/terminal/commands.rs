//! Terminal PTY Tauri commands
//!
//! This module contains real-time Tauri commands for terminal PTY operations.
//! These are "patch" operations that don't record Events (high-frequency).
//!
//! ## Commands
//!
//! - `init_pty_session` - Initialize and create a PTY session
//! - `write_to_pty` - Write user input to PTY (every keystroke)
//! - `resize_pty` - Resize PTY window (on terminal resize)
//! - `close_pty_session` - Close PTY session
//!
//! ## Architecture Notes
//!
//! Event-producing operations are handled through capabilities:
//! - `terminal.init` - Session started event (called via execute_command)
//! - `terminal.execute` - Command executed event (called via execute_command)
//! - `terminal.save` - Content saved event (called via execute_command)
//! - `terminal.close` - Session closed event (called via execute_command)
//!
//! PTY operations use pure functions from `utils/pty.rs`.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::Serialize;
use specta::specta;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::thread;
use tauri::{AppHandle, Emitter, State};

use super::pty::{resize as pty_resize, spawn as pty_spawn, write as pty_write, SpawnConfig};
use super::{TerminalSession, TerminalState};

/// Payload for PTY output events sent to the frontend.
#[derive(Clone, Serialize)]
struct PtyOutputPayload {
    /// Base64 encoded output data
    data: String,
    /// The terminal block ID
    block_id: String,
}

// ============================================================================
// Shell Initialization
// ============================================================================

/// Generate shell initialization script.
///
/// Overrides the `cd` command to intercept when the user's home directory
/// is passed (after shell expansion of `~`) and redirects to the .elf workspace.
///
/// # Arguments
/// * `work_dir` - The .elf temporary workspace directory
/// * `shell` - The shell type: "bash", "zsh", or "powershell"
///
/// # Returns
/// * `Ok(String)` - The initialization script content
/// * `Err(String)` - Error for unsupported shell types
fn generate_shell_init(work_dir: &Path, shell: &str) -> Result<String, String> {
    let work_dir_str = work_dir
        .to_str()
        .ok_or("Failed to convert work directory path to string")?;

    match shell {
        "bash" | "zsh" => Ok(format!(
            r#"clear
export ELF_WORK_DIR="{}"
cd() {{
    # Check if path equals user's home directory (shell expands ~ to $HOME)
    if [ "$1" = "$HOME" ] || [ "$1" = "~" ]; then
        builtin cd "$ELF_WORK_DIR"
    elif [ -z "$1" ]; then
        builtin cd "$HOME"
    else
        builtin cd "$@"
    fi
}}
"#,
            work_dir_str
        )),
        // PowerShell script to be run via -File
        // Must remove the built-in cd alias first, then define global function
        "powershell" => Ok(format!(
            r#"$env:ELF_WORK_DIR = "{}"
Remove-Item alias:cd -Force -ErrorAction SilentlyContinue
function global:cd {{
    param([string]$Path)
    if ($Path -eq "~" -or $Path -eq $env:USERPROFILE) {{
        Set-Location $env:ELF_WORK_DIR
    }} elseif ([string]::IsNullOrEmpty($Path)) {{
        Set-Location $env:USERPROFILE
    }} else {{
        Set-Location $Path
    }}
}}
Clear-Host
"#,
            work_dir_str
        )),
        _ => Err(format!("Unsupported shell: {}", shell)),
    }
}

/// Detect the current shell type.
fn detect_shell() -> &'static str {
    if cfg!(target_os = "windows") {
        "powershell"
    } else {
        "bash"
    }
}

/// Initialize and create a new PTY session.
///
/// This command:
/// 1. Creates a PTY using pure functions from utils/pty.rs
/// 2. Spawns a shell process
/// 3. Starts a reader thread to forward PTY output to the frontend
/// 4. Stores the session in TerminalState
///
/// Note: The terminal.init capability should be called separately via
/// execute_command to record the session_started event.
///
/// # Arguments
/// * `app` - Tauri app handle for emitting events
/// * `state` - Terminal state containing active sessions
/// * `block_id` - The terminal block ID
/// * `cols` - Number of columns
/// * `rows` - Number of rows
/// * `cwd` - Optional working directory
///
/// # Returns
/// * `Ok(())` - Session created successfully
/// * `Err(String)` - Error if creation fails
#[tauri::command]
#[specta]
pub async fn init_pty_session(
    app: AppHandle,
    state: State<'_, TerminalState>,
    block_id: String,
    cols: u16,
    rows: u16,
    cwd: Option<String>,
) -> Result<(), String> {
    // Check if session already exists
    {
        let sessions = state.sessions.lock().unwrap();
        if sessions.contains_key(&block_id) {
            return Err(format!(
                "Terminal session already exists for block: {}",
                block_id
            ));
        }
    }

    // Convert cwd to PathBuf if provided
    let cwd_path = cwd.map(PathBuf::from);

    // Detect shell type
    let shell = detect_shell();
    let mut args = Vec::new();
    let mut script_content_for_pty = String::new();

    // Prepare init script
    if let Some(ref work_dir) = cwd_path {
        if let Ok(content) = generate_shell_init(work_dir, shell) {
            if shell == "powershell" {
                // For PowerShell, write to temp file and pass as argument
                // This ensures clean execution and no visual noise
                let temp_dir = std::env::temp_dir();
                let profile_path =
                    temp_dir.join(format!("elfiee_init_{}.ps1", uuid::Uuid::new_v4()));

                std::fs::write(&profile_path, content)
                    .map_err(|e| format!("Failed to write init script: {}", e))?;

                args.push("-NoExit".to_string());
                args.push("-ExecutionPolicy".to_string());
                args.push("Bypass".to_string());
                args.push("-File".to_string());
                args.push(profile_path.to_string_lossy().to_string());
            } else {
                // For Bash/Zsh, we will inject via PTY
                script_content_for_pty = content;
            }
        }
    }

    // Spawn PTY using pure function
    let config = SpawnConfig {
        cols,
        rows,
        cwd: cwd_path.as_deref(),
        shell: Some(shell),
        args,
    };

    let handle = pty_spawn(config)?;

    // Create the session (take writer and master, reader goes to thread)
    // When the session is dropped, PTY resources are released and reader gets EOF
    let session = TerminalSession {
        writer: handle.writer,
        master: handle.master,
    };

    // Store session in state
    {
        let mut sessions = state.sessions.lock().unwrap();
        sessions.insert(block_id.clone(), session);
    }

    // Send shell initialization script if we have a working directory
    // This overrides `cd ~` to redirect to the workspace directory
    // Only for Bash/Zsh (PowerShell handled via args)
    if !script_content_for_pty.is_empty() {
        // Small delay to ensure shell is ready
        thread::sleep(std::time::Duration::from_millis(100));

        // Send init script to PTY
        let mut sessions = state.sessions.lock().unwrap();
        if let Some(session) = sessions.get_mut(&block_id) {
            let _ = pty_write(&mut *session.writer, &script_content_for_pty);
        }
    }

    // Spawn reader thread to forward PTY output to frontend
    // Thread exits naturally when PTY is closed (reader returns EOF)
    let block_id_clone = block_id.clone();
    let mut reader = handle.reader;

    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => {
                    // EOF - PTY closed (session dropped)
                    break;
                }
                Ok(n) => {
                    // Encode output as base64 (frontend expects this format)
                    let data = BASE64.encode(&buf[..n]);
                    let payload = PtyOutputPayload {
                        data,
                        block_id: block_id_clone.clone(),
                    };
                    // Emit to "pty-out" event (frontend listens on this)
                    let _ = app.emit("pty-out", payload);
                }
                Err(e) => {
                    // Log error but continue (may be temporary)
                    log::warn!("PTY read error for {}: {}", block_id_clone, e);
                    // Small delay to prevent busy loop on persistent errors
                    thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        }
    });

    Ok(())
}

/// Write data to the PTY.
///
/// This command forwards user input from the frontend terminal (xterm.js)
/// to the backend PTY process.
///
/// **Note**: This is a high-frequency operation (called on every keystroke).
/// No capability check is performed for performance reasons.
/// Authorization was already checked during session initialization.
///
/// # Arguments
/// * `state` - Terminal state containing active sessions
/// * `block_id` - The terminal block ID
/// * `data` - The data to write (user input)
///
/// # Returns
/// * `Ok(())` - Data written successfully
/// * `Err(String)` - Error if session not found or write fails
#[tauri::command]
#[specta]
pub async fn write_to_pty(
    state: State<'_, TerminalState>,
    block_id: String,
    data: String,
) -> Result<(), String> {
    let mut sessions = state.sessions.lock().unwrap();
    if let Some(session) = sessions.get_mut(&block_id) {
        pty_write(&mut *session.writer, &data)?;
    } else {
        return Err("Terminal session not found".to_string());
    }

    Ok(())
}

/// Resize the PTY.
///
/// This command updates the PTY window dimensions when the frontend
/// terminal viewport changes size.
///
/// **Note**: This is a high-frequency operation (called on window resize).
/// No capability check is performed for performance reasons.
/// Authorization was already checked during session initialization.
///
/// # Arguments
/// * `state` - Terminal state containing active sessions
/// * `block_id` - The terminal block ID
/// * `cols` - New number of columns
/// * `rows` - New number of rows
///
/// # Returns
/// * `Ok(())` - PTY resized successfully
/// * `Err(String)` - Error if session not found or resize fails
#[tauri::command]
#[specta]
pub async fn resize_pty(
    state: State<'_, TerminalState>,
    block_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let mut sessions = state.sessions.lock().unwrap();
    if let Some(session) = sessions.get_mut(&block_id) {
        pty_resize(&*session.master, cols, rows)?;
    } else {
        return Err("Terminal session not found".to_string());
    }

    Ok(())
}

/// Close a PTY session.
///
/// This command removes the session from state and drops the PTY resources.
/// When the session is dropped, the reader thread receives EOF and exits naturally.
///
/// Note: The terminal.close capability should be called separately via
/// execute_command to record the session_closed event.
///
/// # Arguments
/// * `state` - Terminal state containing active sessions
/// * `block_id` - The terminal block ID
///
/// # Returns
/// * `Ok(())` - Session closed successfully (or was already closed)
/// * `Err(String)` - Error if operation fails
#[tauri::command]
#[specta]
pub async fn close_pty_session(
    state: State<'_, TerminalState>,
    block_id: String,
) -> Result<(), String> {
    let mut sessions = state.sessions.lock().unwrap();
    // Removing and dropping the session releases PTY resources,
    // causing the reader thread to receive EOF and exit
    sessions.remove(&block_id);
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_bash_basic() {
        let work_dir = PathBuf::from("/tmp/test-elf-workspace");
        let script = generate_shell_init(&work_dir, "bash");
        assert!(script.is_ok(), "Should generate bash script");

        let script = script.unwrap();
        assert!(
            script.contains("export ELF_WORK_DIR="),
            "Script should export ELF_WORK_DIR"
        );
        assert!(
            script.contains("/tmp/test-elf-workspace"),
            "Script should contain the work directory path"
        );
        assert!(
            script.contains("cd()"),
            "Script should define cd function override"
        );
        assert!(
            script.contains("builtin cd"),
            "Script should use builtin cd"
        );
        assert!(script.contains("clear"), "Script should clear the screen");
    }

    #[test]
    fn test_shell_zsh_basic() {
        let work_dir = PathBuf::from("/home/user/.elf-workspace");
        let script = generate_shell_init(&work_dir, "zsh");
        assert!(script.is_ok(), "Should generate zsh script");

        let script = script.unwrap();
        assert!(script.contains("clear"));
        assert!(script.contains("export ELF_WORK_DIR="));
        assert!(script.contains("/home/user/.elf-workspace"));
        assert!(script.contains("cd()"));
    }

    #[test]
    fn test_shell_powershell_basic() {
        let work_dir = PathBuf::from("C:\\Users\\test\\.elf-workspace");
        let script = generate_shell_init(&work_dir, "powershell");
        assert!(script.is_ok(), "Should handle PowerShell");

        let script = script.unwrap();
        assert!(
            script.contains("$env:ELF_WORK_DIR ="),
            "PowerShell script should set environment variable"
        );
        assert!(
            script.contains("Remove-Item alias:cd"),
            "PowerShell script should remove built-in cd alias"
        );
        assert!(
            script.contains("function global:cd"),
            "PowerShell script should define global:cd function"
        );
        assert!(
            script.contains("Set-Location $env:ELF_WORK_DIR"),
            "PowerShell script should support cd ~"
        );
    }

    #[test]
    fn test_shell_unsupported_error() {
        let work_dir = PathBuf::from("/tmp/test");
        let result = generate_shell_init(&work_dir, "fish");
        assert!(result.is_err(), "Should reject unsupported shell");
        assert!(result.unwrap_err().contains("Unsupported shell"));
    }

    #[test]
    fn test_shell_special_path_edge_case() {
        let work_dir = PathBuf::from("/tmp/elfiee workspace/test-123");
        let script = generate_shell_init(&work_dir, "bash");
        assert!(script.is_ok(), "Should handle paths with spaces");
        assert!(script.unwrap().contains("/tmp/elfiee workspace/test-123"));
    }

    #[test]
    fn test_shell_cd_override_basic() {
        let work_dir = PathBuf::from("/tmp/elfiee-workspace");
        let script = generate_shell_init(&work_dir, "bash").unwrap();

        assert!(
            script.contains(r#"if [ "$1" = "$HOME" ] || [ "$1" = "~" ]"#),
            "Script should check for both $HOME and ~"
        );
        assert!(
            script.contains(r#"builtin cd "$ELF_WORK_DIR""#),
            "Script should redirect to ELF_WORK_DIR"
        );
    }

    #[test]
    fn test_detect_shell() {
        let shell = detect_shell();
        if cfg!(target_os = "windows") {
            assert_eq!(shell, "powershell");
        } else {
            assert_eq!(shell, "bash");
        }
    }
}
