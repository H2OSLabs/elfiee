//! JSONL incremental parser + JSON converter
//!
//! Parses Claude Code session `.jsonl` files incrementally (using byte offset tracking)
//! and extracts key fields from each message.
//!
//! ## Output Format
//!
//! ```json
//! {
//!   "session_id": "abc-123",
//!   "project": "elfiee",
//!   "messages": [
//!     {
//!       "type": "user",
//!       "message": {"role": "user", "content": "..."},
//!       "timestamp": "2026-02-03T06:46:50.519Z",
//!       "version": "2.1.29",
//!       "gitBranch": "dev"
//!     },
//!     {
//!       "type": "assistant",
//!       "message": {"model": "claude-opus-4-5", "role": "assistant", "content": [...]},
//!       "timestamp": "2026-02-03T06:46:55.797Z"
//!     },
//!     {
//!       "type": "progress",
//!       "data": {"type": "agent_progress", ...},
//!       "timestamp": "2026-02-03T06:47:09.133Z"
//!     }
//!   ]
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// A single message in the session, preserving original JSONL fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    /// Message type: "user", "assistant", "progress", etc.
    #[serde(rename = "type")]
    pub msg_type: String,

    /// The message content object (for user/assistant messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<serde_json::Value>,

    /// ISO 8601 timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,

    /// Claude Code version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Git branch name
    #[serde(rename = "gitBranch", skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,

    /// Progress data (for progress messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// The complete session document structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub session_id: String,
    pub project: String,
    pub messages: Vec<SessionMessage>,
}

impl SessionData {
    pub fn new(session_id: String, project: String) -> Self {
        Self {
            session_id,
            project,
            messages: Vec::new(),
        }
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Deserialize from JSON string.
    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }
}

/// Parsed session document with new messages to append.
#[derive(Debug, Clone)]
pub struct SessionDocument {
    /// Session ID (from sessionId field or filename)
    pub session_id: String,
    /// New messages to append
    pub new_messages: Vec<SessionMessage>,
    /// Session metadata (extracted from first message)
    pub metadata: SessionMetadata,
    /// Number of new messages converted in this parse
    pub new_message_count: usize,
}

/// Session metadata extracted from JSONL messages.
#[derive(Debug, Clone, Default)]
pub struct SessionMetadata {
    pub model: Option<String>,
    pub git_branch: Option<String>,
    pub cwd: Option<String>,
    pub version: Option<String>,
    pub started_at: Option<String>,
}

/// Tracks byte offsets per file for incremental parsing.
pub struct OffsetTracker {
    offsets: HashMap<PathBuf, u64>,
}

impl OffsetTracker {
    pub fn new() -> Self {
        Self {
            offsets: HashMap::new(),
        }
    }

    pub fn get(&self, path: &Path) -> u64 {
        self.offsets.get(path).copied().unwrap_or(0)
    }

    pub fn set(&mut self, path: &Path, offset: u64) {
        self.offsets.insert(path.to_path_buf(), offset);
    }

    pub fn reset(&mut self, path: &Path) {
        self.offsets.remove(path);
    }
}

/// JSONL incremental parser that converts to structured JSON.
pub struct SessionParser {
    tracker: OffsetTracker,
}

impl SessionParser {
    pub fn new() -> Self {
        Self {
            tracker: OffsetTracker::new(),
        }
    }

    /// Parse new lines from a JSONL file incrementally.
    ///
    /// Returns `Ok(Some(doc))` if new content was found, `Ok(None)` if no
    /// new meaningful content, `Err` on I/O failure.
    pub fn parse_incremental(
        &mut self,
        file_path: &Path,
    ) -> Result<Option<SessionDocument>, String> {
        let file = File::open(file_path)
            .map_err(|e| format!("Cannot open {}: {}", file_path.display(), e))?;

        let file_len = file
            .metadata()
            .map_err(|e| format!("Cannot stat {}: {}", file_path.display(), e))?
            .len();

        let current_offset = self.tracker.get(file_path);

        // File was truncated — reset and re-parse from start
        if file_len < current_offset {
            self.tracker.reset(file_path);
            return self.parse_incremental(file_path);
        }

        // No new data
        if file_len == current_offset {
            return Ok(None);
        }

        let mut reader = BufReader::new(file);
        reader
            .seek(SeekFrom::Start(current_offset))
            .map_err(|e| format!("Seek error: {}", e))?;

        let mut new_messages: Vec<SessionMessage> = Vec::new();
        let mut metadata = SessionMetadata::default();
        let mut session_id = String::new();
        let mut new_offset = current_offset;

        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    new_offset += n as u64;
                }
                Err(e) => {
                    log::warn!("Read error at offset {}: {}", new_offset, e);
                    break;
                }
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let json: serde_json::Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(e) => {
                    log::warn!(
                        "Malformed JSONL line in {}: {} (skipping)",
                        file_path.display(),
                        e
                    );
                    continue;
                }
            };

            // Extract session_id from first message
            if session_id.is_empty() {
                if let Some(sid) = json.get("sessionId").and_then(|v| v.as_str()) {
                    session_id = sid.to_string();
                }
            }

            // Extract metadata from early messages
            update_metadata(&json, &mut metadata);

            // Convert to SessionMessage
            if let Some(msg) = jsonl_to_message(&json) {
                new_messages.push(msg);
            }
        }

        self.tracker.set(file_path, new_offset);

        if new_messages.is_empty() {
            return Ok(None);
        }

        // Fallback: derive session_id from filename if not found in content
        if session_id.is_empty() {
            session_id = file_path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());
        }

        let message_count = new_messages.len();

        Ok(Some(SessionDocument {
            session_id,
            new_messages,
            metadata,
            new_message_count: message_count,
        }))
    }

    /// Get current byte offset for a file.
    pub fn get_offset(&self, file_path: &Path) -> u64 {
        self.tracker.get(file_path)
    }

    /// Set byte offset for a file (used when restoring from persisted metadata).
    pub fn set_offset(&mut self, file_path: &Path, offset: u64) {
        self.tracker.set(file_path, offset);
    }

    /// Reset offset for a file (forces full re-parse on next call).
    pub fn reset_offset(&mut self, file_path: &Path) {
        self.tracker.reset(file_path);
    }
}

/// Create initial SessionData.
pub fn create_session_data(
    _metadata: &SessionMetadata,
    session_id: &str,
    project_name: &str,
) -> SessionData {
    SessionData {
        session_id: session_id.to_string(),
        project: project_name.to_string(),
        messages: Vec::new(),
    }
}

/// Update metadata from a JSONL object (extracts model, branch, cwd, etc.).
fn update_metadata(json: &serde_json::Value, meta: &mut SessionMetadata) {
    if meta.model.is_none() {
        if let Some(model) = json
            .get("message")
            .and_then(|m| m.get("model"))
            .and_then(|v| v.as_str())
        {
            meta.model = Some(model.to_string());
        }
    }
    if meta.git_branch.is_none() {
        if let Some(branch) = json.get("gitBranch").and_then(|v| v.as_str()) {
            meta.git_branch = Some(branch.to_string());
        }
    }
    if meta.cwd.is_none() {
        if let Some(cwd) = json.get("cwd").and_then(|v| v.as_str()) {
            meta.cwd = Some(cwd.to_string());
        }
    }
    if meta.version.is_none() {
        if let Some(ver) = json.get("version").and_then(|v| v.as_str()) {
            meta.version = Some(ver.to_string());
        }
    }
    if meta.started_at.is_none() {
        if let Some(ts) = json.get("timestamp").and_then(|v| v.as_str()) {
            meta.started_at = Some(ts.to_string());
        }
    }
}

/// Convert a single JSONL object to a SessionMessage.
///
/// Extracts fields: type, message, timestamp, version, gitBranch, data
fn jsonl_to_message(json: &serde_json::Value) -> Option<SessionMessage> {
    // type is required
    let msg_type = json.get("type")?.as_str()?.to_string();

    // Skip internal types that are not useful
    if matches!(
        msg_type.as_str(),
        "file-history-snapshot" | "summary" | "system" | "result"
    ) {
        return None;
    }

    Some(SessionMessage {
        msg_type,
        message: json.get("message").cloned(),
        timestamp: json
            .get("timestamp")
            .and_then(|v| v.as_str())
            .map(String::from),
        version: json
            .get("version")
            .and_then(|v| v.as_str())
            .map(String::from),
        git_branch: json
            .get("gitBranch")
            .and_then(|v| v.as_str())
            .map(String::from),
        data: json.get("data").cloned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_user_message() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "type": "user",
                "message": {"role": "user", "content": "Hello, help me check the code"},
                "timestamp": "2026-02-03T06:46:50.519Z",
                "version": "2.1.29",
                "gitBranch": "dev",
                "sessionId": "abc123"
            }"#,
        )
        .unwrap();

        let msg = jsonl_to_message(&json).unwrap();
        assert_eq!(msg.msg_type, "user");
        assert_eq!(msg.timestamp, Some("2026-02-03T06:46:50.519Z".to_string()));
        assert_eq!(msg.version, Some("2.1.29".to_string()));
        assert_eq!(msg.git_branch, Some("dev".to_string()));
        assert!(msg.message.is_some());
        let message = msg.message.unwrap();
        assert_eq!(
            message["content"].as_str().unwrap(),
            "Hello, help me check the code"
        );
    }

    #[test]
    fn test_parse_assistant_message() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "type": "assistant",
                "message": {
                    "model": "claude-opus-4-5-20251101",
                    "role": "assistant",
                    "content": [
                        {"type": "text", "text": "Let me analyze the code."}
                    ]
                },
                "timestamp": "2026-02-03T06:46:55.797Z"
            }"#,
        )
        .unwrap();

        let msg = jsonl_to_message(&json).unwrap();
        assert_eq!(msg.msg_type, "assistant");
        assert_eq!(msg.timestamp, Some("2026-02-03T06:46:55.797Z".to_string()));
        assert!(msg.message.is_some());
        let message = msg.message.unwrap();
        assert_eq!(
            message["model"].as_str().unwrap(),
            "claude-opus-4-5-20251101"
        );
    }

    #[test]
    fn test_assistant_with_tool_use_preserved() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "type": "assistant",
                "message": {
                    "role": "assistant",
                    "content": [
                        {"type": "tool_use", "id": "toolu_xxx", "name": "Bash", "input": {"command": "cargo test"}}
                    ]
                },
                "timestamp": "2026-02-03T06:47:00Z"
            }"#,
        )
        .unwrap();

        // Now we preserve assistant messages with tool_use
        let msg = jsonl_to_message(&json).unwrap();
        assert_eq!(msg.msg_type, "assistant");
        assert!(msg.message.is_some());
    }

    #[test]
    fn test_progress_message_preserved() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "type": "progress",
                "data": {"type": "agent_progress", "agentId": "abc123"},
                "timestamp": "2026-02-03T06:47:09Z"
            }"#,
        )
        .unwrap();

        let msg = jsonl_to_message(&json).unwrap();
        assert_eq!(msg.msg_type, "progress");
        assert!(msg.data.is_some());
        let data = msg.data.unwrap();
        assert_eq!(data["type"].as_str().unwrap(), "agent_progress");
        assert_eq!(data["agentId"].as_str().unwrap(), "abc123");
    }

    #[test]
    fn test_file_history_snapshot_skipped() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "type": "file-history-snapshot",
                "files": [],
                "timestamp": "2026-02-03T06:46:00Z"
            }"#,
        )
        .unwrap();

        assert!(jsonl_to_message(&json).is_none());
    }

    #[test]
    fn test_summary_skipped() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "type": "summary",
                "summary": "Session summary text",
                "timestamp": "2026-02-03T06:50:00Z"
            }"#,
        )
        .unwrap();

        assert!(jsonl_to_message(&json).is_none());
    }

    #[test]
    fn test_session_data_serialization() {
        let mut data = SessionData::new("abc-123".to_string(), "elfiee".to_string());
        data.messages.push(SessionMessage {
            msg_type: "user".to_string(),
            message: Some(serde_json::json!({"role": "user", "content": "Hello"})),
            timestamp: Some("2026-02-03T06:46:50.519Z".to_string()),
            version: Some("2.1.29".to_string()),
            git_branch: Some("dev".to_string()),
            data: None,
        });
        data.messages.push(SessionMessage {
            msg_type: "assistant".to_string(),
            message: Some(serde_json::json!({"role": "assistant", "content": [{"type": "text", "text": "Hi there!"}]})),
            timestamp: Some("2026-02-03T06:46:55.797Z".to_string()),
            version: None,
            git_branch: None,
            data: None,
        });

        let json = data.to_json();
        assert!(json.contains("\"session_id\": \"abc-123\""));
        assert!(json.contains("\"project\": \"elfiee\""));
        assert!(json.contains("\"messages\""));
        assert!(json.contains("\"type\": \"user\""));
        assert!(json.contains("\"gitBranch\": \"dev\""));

        // Round-trip
        let parsed = SessionData::from_json(&json).unwrap();
        assert_eq!(parsed.session_id, "abc-123");
        assert_eq!(parsed.messages.len(), 2);
        assert_eq!(parsed.messages[0].msg_type, "user");
        assert_eq!(parsed.messages[1].msg_type, "assistant");
    }

    #[test]
    fn test_missing_type_returns_none() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "message": {"role": "user", "content": "Hello"},
                "timestamp": "2026-02-03T06:46:50Z"
            }"#,
        )
        .unwrap();

        assert!(jsonl_to_message(&json).is_none());
    }

    #[test]
    fn test_optional_fields_omitted() {
        let msg = SessionMessage {
            msg_type: "user".to_string(),
            message: Some(serde_json::json!({"content": "test"})),
            timestamp: Some("2026-02-03T06:46:50Z".to_string()),
            version: None,
            git_branch: None,
            data: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(!json.contains("version"));
        assert!(!json.contains("gitBranch"));
        assert!(!json.contains("data"));
    }
}
