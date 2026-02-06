//! Tests for the session sync module.
//!
//! Unit tests for session_path and parser are co-located in their respective files.
//! This file contains integration-level tests.

use super::parser::*;
use super::session_path::*;
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

// ============================================================================
// Parser Integration Tests
// ============================================================================

#[test]
fn test_incremental_parse_only_new_lines() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test-session.jsonl");

    // Write initial content
    {
        let mut f = std::fs::File::create(&file_path).unwrap();
        writeln!(
            f,
            r#"{{"type":"user","message":{{"role":"user","content":"Hello"}},"timestamp":"2026-02-03T06:00:00Z","sessionId":"test-1"}}"#
        )
        .unwrap();
    }

    let mut parser = SessionParser::new();

    // First parse
    let result = parser.parse_incremental(&file_path).unwrap();
    assert!(result.is_some());
    let doc = result.unwrap();
    assert_eq!(doc.session_id, "test-1");
    assert_eq!(doc.new_message_count, 1);
    assert_eq!(doc.new_messages.len(), 1);
    assert_eq!(doc.new_messages[0].msg_type, "user");
    let msg = doc.new_messages[0].message.as_ref().unwrap();
    assert!(msg["content"].as_str().unwrap().contains("Hello"));

    // Parse again without changes → should return None
    let result = parser.parse_incremental(&file_path).unwrap();
    assert!(result.is_none());

    // Append new content
    {
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&file_path)
            .unwrap();
        writeln!(
            f,
            r#"{{"type":"assistant","message":{{"role":"assistant","content":[{{"type":"text","text":"Hi there!"}}]}},"timestamp":"2026-02-03T06:01:00Z"}}"#
        )
        .unwrap();
    }

    // Second parse — should only get the new line
    let result = parser.parse_incremental(&file_path).unwrap();
    assert!(result.is_some());
    let doc = result.unwrap();
    assert_eq!(doc.new_message_count, 1);
    assert_eq!(doc.new_messages.len(), 1);
    assert_eq!(doc.new_messages[0].msg_type, "assistant");
    let msg = doc.new_messages[0].message.as_ref().unwrap();
    let content = &msg["content"][0]["text"];
    assert!(content.as_str().unwrap().contains("Hi there!"));
}

#[test]
fn test_malformed_line_skipped() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("malformed.jsonl");

    {
        let mut f = std::fs::File::create(&file_path).unwrap();
        writeln!(f, "this is not json").unwrap();
        writeln!(
            f,
            r#"{{"type":"user","message":{{"role":"user","content":"Valid"}},"timestamp":"2026-02-03T06:00:00Z","sessionId":"test-2"}}"#
        )
        .unwrap();
    }

    let mut parser = SessionParser::new();
    let result = parser.parse_incremental(&file_path).unwrap();
    assert!(result.is_some());
    let doc = result.unwrap();
    assert_eq!(doc.new_message_count, 1);
    assert_eq!(doc.new_messages.len(), 1);
    let msg = doc.new_messages[0].message.as_ref().unwrap();
    assert!(msg["content"].as_str().unwrap().contains("Valid"));
}

#[test]
fn test_file_truncation_resets_offset() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("truncated.jsonl");

    // Write long content
    {
        let mut f = std::fs::File::create(&file_path).unwrap();
        for i in 0..10 {
            writeln!(
                f,
                r#"{{"type":"user","message":{{"role":"user","content":"Message {}"}},"timestamp":"2026-02-03T06:0{}:00Z","sessionId":"test-3"}}"#,
                i, i
            )
            .unwrap();
        }
    }

    let mut parser = SessionParser::new();
    let result = parser.parse_incremental(&file_path).unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().new_message_count, 10);

    // Truncate file (shorter than previous offset)
    {
        let mut f = std::fs::File::create(&file_path).unwrap();
        writeln!(
            f,
            r#"{{"type":"user","message":{{"role":"user","content":"Fresh start"}},"timestamp":"2026-02-03T07:00:00Z","sessionId":"test-3"}}"#
        )
        .unwrap();
    }

    // Should detect truncation and re-parse from start
    let result = parser.parse_incremental(&file_path).unwrap();
    assert!(result.is_some());
    let doc = result.unwrap();
    assert_eq!(doc.new_message_count, 1);
    assert_eq!(doc.new_messages.len(), 1);
    let msg = doc.new_messages[0].message.as_ref().unwrap();
    assert!(msg["content"].as_str().unwrap().contains("Fresh start"));
}

#[test]
fn test_empty_file_returns_none() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("empty.jsonl");
    std::fs::File::create(&file_path).unwrap();

    let mut parser = SessionParser::new();
    let result = parser.parse_incremental(&file_path).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_session_id_from_filename_fallback() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("abc-123-def.jsonl");

    {
        let mut f = std::fs::File::create(&file_path).unwrap();
        // Write a message without sessionId
        writeln!(
            f,
            r#"{{"type":"user","message":{{"role":"user","content":"Test"}},"timestamp":"2026-02-03T06:00:00Z"}}"#
        )
        .unwrap();
    }

    let mut parser = SessionParser::new();
    let result = parser.parse_incremental(&file_path).unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().session_id, "abc-123-def");
}

// ============================================================================
// Path Encoding Tests (additional)
// ============================================================================

#[test]
fn test_encode_temp_path() {
    // Temp paths like "C:\Users\Lenovo\AppData\Local\Temp\tmp6mKgVe"
    let encoded = encode_project_path(Path::new(
        "C:\\Users\\Lenovo\\AppData\\Local\\Temp\\tmp6mKgVe",
    ));
    assert_eq!(encoded, "C--Users-Lenovo-AppData-Local-Temp-tmp6mKgVe");
}

#[test]
#[cfg(windows)]
fn test_encode_nested_project_path() {
    let encoded = encode_project_path(Path::new(
        "D:\\workspace\\zhidaoyuan\\elfiee-peoject\\elfiee",
    ));
    assert_eq!(encoded, "D--workspace-zhidaoyuan-elfiee-peoject-elfiee");
}

#[test]
#[cfg(not(windows))]
fn test_encode_nested_project_path_unix() {
    let encoded = encode_project_path(Path::new(
        "/home/yaosh/projects/elfiee-project/elfiee",
    ));
    assert_eq!(encoded, "-home-yaosh-projects-elfiee-project-elfiee");
}

#[test]
#[cfg(windows)]
fn test_extract_project_name_from_complex_path() {
    assert_eq!(
        extract_project_name_from_config(
            "D:\\workspace\\zhidaoyuan\\frontend-component-library\\.cursor"
        ),
        "frontend-component-library"
    );
}

#[test]
#[cfg(not(windows))]
fn test_extract_project_name_from_complex_path_unix() {
    assert_eq!(
        extract_project_name_from_config(
            "/home/yaosh/projects/frontend-component-library/.cursor"
        ),
        "frontend-component-library"
    );
}

// ============================================================================
// Additional Tests per Requirements Document
// ============================================================================

#[test]
fn test_metadata_extraction() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("metadata-test.jsonl");

    {
        let mut f = std::fs::File::create(&file_path).unwrap();
        // User message with metadata
        writeln!(
            f,
            r#"{{"type":"user","message":{{"role":"user","content":"Hello"}},"timestamp":"2026-02-03T06:00:00Z","sessionId":"meta-test","version":"2.1.29","gitBranch":"dev","cwd":"D:\\workspace\\elfiee"}}"#
        ).unwrap();
        // Assistant message with model info
        writeln!(
            f,
            r#"{{"type":"assistant","message":{{"model":"claude-opus-4-5-20251101","role":"assistant","content":[{{"type":"text","text":"Hi!"}}]}},"timestamp":"2026-02-03T06:01:00Z"}}"#
        ).unwrap();
    }

    let mut parser = SessionParser::new();
    let result = parser.parse_incremental(&file_path).unwrap();
    assert!(result.is_some());
    let doc = result.unwrap();

    // Check metadata was extracted
    assert_eq!(doc.metadata.version, Some("2.1.29".to_string()));
    assert_eq!(doc.metadata.git_branch, Some("dev".to_string()));
    assert_eq!(doc.metadata.cwd, Some("D:\\workspace\\elfiee".to_string()));
    assert_eq!(
        doc.metadata.model,
        Some("claude-opus-4-5-20251101".to_string())
    );
    assert_eq!(
        doc.metadata.started_at,
        Some("2026-02-03T06:00:00Z".to_string())
    );
}

#[test]
fn test_session_data_round_trip() {
    // Create session data with messages
    let mut data = SessionData::new("test-session-123".to_string(), "my-project".to_string());
    data.messages.push(SessionMessage {
        msg_type: "user".to_string(),
        message: Some(serde_json::json!({"role": "user", "content": "Hello world"})),
        timestamp: Some("2026-02-03T06:00:00Z".to_string()),
        version: Some("2.1.29".to_string()),
        git_branch: Some("main".to_string()),
        data: None,
    });
    data.messages.push(SessionMessage {
        msg_type: "progress".to_string(),
        message: None,
        timestamp: Some("2026-02-03T06:01:00Z".to_string()),
        version: None,
        git_branch: None,
        data: Some(serde_json::json!({"type": "agent_progress", "agentId": "xyz"})),
    });

    // Serialize to JSON
    let json = data.to_json();

    // Deserialize back
    let parsed = SessionData::from_json(&json).unwrap();

    // Verify round-trip
    assert_eq!(parsed.session_id, "test-session-123");
    assert_eq!(parsed.project, "my-project");
    assert_eq!(parsed.messages.len(), 2);
    assert_eq!(parsed.messages[0].msg_type, "user");
    assert_eq!(parsed.messages[0].git_branch, Some("main".to_string()));
    assert_eq!(parsed.messages[1].msg_type, "progress");
    assert!(parsed.messages[1].data.is_some());
}

#[test]
fn test_offset_persistence_and_restore() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("offset-test.jsonl");

    {
        let mut f = std::fs::File::create(&file_path).unwrap();
        writeln!(
            f,
            r#"{{"type":"user","message":{{"role":"user","content":"Line 1"}},"timestamp":"2026-02-03T06:00:00Z","sessionId":"offset-test"}}"#
        ).unwrap();
    }

    let mut parser = SessionParser::new();

    // Parse first line
    let _ = parser.parse_incremental(&file_path).unwrap();

    // Get offset
    let offset = parser.get_offset(&file_path);
    assert!(offset > 0);

    // Simulate restart: new parser with restored offset
    let mut parser2 = SessionParser::new();
    parser2.set_offset(&file_path, offset);

    // Should return None since no new content
    let result = parser2.parse_incremental(&file_path).unwrap();
    assert!(result.is_none());

    // Append new content
    {
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&file_path)
            .unwrap();
        writeln!(
            f,
            r#"{{"type":"user","message":{{"role":"user","content":"Line 2"}},"timestamp":"2026-02-03T06:01:00Z"}}"#
        ).unwrap();
    }

    // Now should get the new line
    let result = parser2.parse_incremental(&file_path).unwrap();
    assert!(result.is_some());
    let doc = result.unwrap();
    assert_eq!(doc.new_message_count, 1);
    let msg = doc.new_messages[0].message.as_ref().unwrap();
    assert!(msg["content"].as_str().unwrap().contains("Line 2"));
}

#[test]
fn test_system_message_skipped() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("system-skip.jsonl");

    {
        let mut f = std::fs::File::create(&file_path).unwrap();
        writeln!(
            f,
            r#"{{"type":"system","content":"System initialized","timestamp":"2026-02-03T06:00:00Z","sessionId":"sys-test"}}"#
        ).unwrap();
        writeln!(
            f,
            r#"{{"type":"user","message":{{"role":"user","content":"Hello"}},"timestamp":"2026-02-03T06:01:00Z"}}"#
        ).unwrap();
    }

    let mut parser = SessionParser::new();
    let result = parser.parse_incremental(&file_path).unwrap();
    assert!(result.is_some());
    let doc = result.unwrap();
    // Only the user message should be parsed
    assert_eq!(doc.new_message_count, 1);
    assert_eq!(doc.new_messages[0].msg_type, "user");
}

#[test]
fn test_result_message_skipped() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("result-skip.jsonl");

    {
        let mut f = std::fs::File::create(&file_path).unwrap();
        writeln!(
            f,
            r#"{{"type":"result","result":{{"success":true}},"timestamp":"2026-02-03T06:00:00Z","sessionId":"result-test"}}"#
        ).unwrap();
        writeln!(
            f,
            r#"{{"type":"assistant","message":{{"role":"assistant","content":[{{"type":"text","text":"Done"}}]}},"timestamp":"2026-02-03T06:01:00Z"}}"#
        ).unwrap();
    }

    let mut parser = SessionParser::new();
    let result = parser.parse_incremental(&file_path).unwrap();
    assert!(result.is_some());
    let doc = result.unwrap();
    // Only the assistant message should be parsed
    assert_eq!(doc.new_message_count, 1);
    assert_eq!(doc.new_messages[0].msg_type, "assistant");
}
