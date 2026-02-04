//! Claude Code Settings Configuration Merger
//!
//! Utilities for managing `.claude/settings.local.json` to inject/remove
//! MCP tool auto-approve permissions when agents are enabled/disabled.
//!
//! Follows the same idempotent merge/remove pattern as `mcp_config.rs`.

use serde_json::Value;
use std::fs;
use std::path::Path;

/// All Elfiee MCP tool names that should be auto-approved.
///
/// When an agent is enabled, these are injected into `permissions.allow`.
/// When disabled, they are removed. This prevents users from having to
/// confirm every MCP tool call manually.
pub const ELFIEE_MCP_TOOLS: &[&str] = &[
    "mcp__elfiee__elfiee_file_list",
    "mcp__elfiee__elfiee_block_list",
    "mcp__elfiee__elfiee_block_get",
    "mcp__elfiee__elfiee_block_create",
    "mcp__elfiee__elfiee_block_delete",
    "mcp__elfiee__elfiee_block_rename",
    "mcp__elfiee__elfiee_block_change_type",
    "mcp__elfiee__elfiee_block_update_metadata",
    "mcp__elfiee__elfiee_block_link",
    "mcp__elfiee__elfiee_block_unlink",
    "mcp__elfiee__elfiee_markdown_read",
    "mcp__elfiee__elfiee_markdown_write",
    "mcp__elfiee__elfiee_code_read",
    "mcp__elfiee__elfiee_code_write",
    "mcp__elfiee__elfiee_directory_create",
    "mcp__elfiee__elfiee_directory_delete",
    "mcp__elfiee__elfiee_directory_rename",
    "mcp__elfiee__elfiee_directory_write",
    "mcp__elfiee__elfiee_directory_import",
    "mcp__elfiee__elfiee_directory_export",
    "mcp__elfiee__elfiee_terminal_init",
    "mcp__elfiee__elfiee_terminal_execute",
    "mcp__elfiee__elfiee_terminal_save",
    "mcp__elfiee__elfiee_terminal_close",
    "mcp__elfiee__elfiee_task_create",
    "mcp__elfiee__elfiee_task_write",
    "mcp__elfiee__elfiee_task_commit",
    "mcp__elfiee__elfiee_task_link",
    "mcp__elfiee__elfiee_grant",
    "mcp__elfiee__elfiee_revoke",
    "mcp__elfiee__elfiee_editor_create",
    "mcp__elfiee__elfiee_editor_delete",
    "mcp__elfiee__elfiee_exec",
];

/// Merge Elfiee MCP tools into `permissions.allow` in a settings file.
///
/// Idempotent: tools already present are not duplicated.
/// If the file does not exist, it is created.
/// Other entries in `permissions.allow` are preserved.
pub fn merge_allowed_tools(settings_path: &Path) -> Result<(), String> {
    let mut root = if settings_path.exists() {
        let content = fs::read_to_string(settings_path)
            .map_err(|e| format!("Failed to read {}: {}", settings_path.display(), e))?;

        if content.trim().is_empty() {
            serde_json::json!({})
        } else {
            serde_json::from_str::<Value>(&content)
                .map_err(|e| format!("Invalid JSON in {}: {}", settings_path.display(), e))?
        }
    } else {
        serde_json::json!({})
    };

    // Ensure permissions.allow array exists
    let root_obj = root
        .as_object_mut()
        .ok_or_else(|| format!("Expected JSON object in {}", settings_path.display()))?;

    if !root_obj.contains_key("permissions") {
        root_obj.insert("permissions".to_string(), serde_json::json!({}));
    }

    let perms = root_obj
        .get_mut("permissions")
        .and_then(|v| v.as_object_mut())
        .ok_or_else(|| "permissions must be a JSON object".to_string())?;

    if !perms.contains_key("allow") {
        perms.insert("allow".to_string(), serde_json::json!([]));
    }

    let allow_list = perms
        .get_mut("allow")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| "permissions.allow must be a JSON array".to_string())?;

    // Add tools that are not already present
    for tool in ELFIEE_MCP_TOOLS {
        let tool_value = Value::String(tool.to_string());
        if !allow_list.contains(&tool_value) {
            allow_list.push(tool_value);
        }
    }

    // Write back with pretty formatting
    let output = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

    // Ensure parent directory exists
    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
    }

    fs::write(settings_path, output)
        .map_err(|e| format!("Failed to write {}: {}", settings_path.display(), e))?;

    Ok(())
}

/// Remove Elfiee MCP tools from `permissions.allow` in a settings file.
///
/// Idempotent: if tools are not present, succeeds silently.
/// If the file does not exist, succeeds silently.
/// Other entries in `permissions.allow` are preserved.
pub fn remove_allowed_tools(settings_path: &Path) -> Result<(), String> {
    if !settings_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(settings_path)
        .map_err(|e| format!("Failed to read {}: {}", settings_path.display(), e))?;

    if content.trim().is_empty() {
        return Ok(());
    }

    let mut root: Value = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid JSON in {}: {}", settings_path.display(), e))?;

    if let Some(perms) = root
        .as_object_mut()
        .and_then(|o| o.get_mut("permissions"))
        .and_then(|v| v.as_object_mut())
    {
        if let Some(allow_list) = perms.get_mut("allow").and_then(|v| v.as_array_mut()) {
            allow_list.retain(|v| {
                if let Value::String(s) = v {
                    !ELFIEE_MCP_TOOLS.contains(&s.as_str())
                } else {
                    true
                }
            });
        }
    }

    let output = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

    fs::write(settings_path, output)
        .map_err(|e| format!("Failed to write {}: {}", settings_path.display(), e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_settings_path() -> (TempDir, std::path::PathBuf) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(".claude").join("settings.local.json");
        (dir, path)
    }

    #[test]
    fn test_merge_new_file() {
        let (_dir, path) = temp_settings_path();
        assert!(!path.exists());

        merge_allowed_tools(&path).unwrap();

        assert!(path.exists());
        let content: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let allow = content["permissions"]["allow"].as_array().unwrap();
        assert!(allow.contains(&Value::String("mcp__elfiee__elfiee_file_list".to_string())));
        assert_eq!(allow.len(), ELFIEE_MCP_TOOLS.len());
    }

    #[test]
    fn test_merge_preserves_existing() {
        let (_dir, path) = temp_settings_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            r#"{"permissions": {"allow": ["Bash(cargo test:*)"]}}"#,
        )
        .unwrap();

        merge_allowed_tools(&path).unwrap();

        let content: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let allow = content["permissions"]["allow"].as_array().unwrap();
        assert!(allow.contains(&Value::String("Bash(cargo test:*)".to_string())));
        assert!(allow.contains(&Value::String("mcp__elfiee__elfiee_file_list".to_string())));
    }

    #[test]
    fn test_merge_idempotent() {
        let (_dir, path) = temp_settings_path();

        merge_allowed_tools(&path).unwrap();
        let count1 = serde_json::from_str::<Value>(&fs::read_to_string(&path).unwrap()).unwrap()
            ["permissions"]["allow"]
            .as_array()
            .unwrap()
            .len();

        merge_allowed_tools(&path).unwrap();
        let count2 = serde_json::from_str::<Value>(&fs::read_to_string(&path).unwrap()).unwrap()
            ["permissions"]["allow"]
            .as_array()
            .unwrap()
            .len();

        assert_eq!(count1, count2);
    }

    #[test]
    fn test_remove_cleans_tools() {
        let (_dir, path) = temp_settings_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            r#"{"permissions": {"allow": ["Bash(cargo test:*)", "mcp__elfiee__elfiee_file_list", "mcp__elfiee__elfiee_block_list"]}}"#,
        )
        .unwrap();

        remove_allowed_tools(&path).unwrap();

        let content: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let allow = content["permissions"]["allow"].as_array().unwrap();
        assert_eq!(allow.len(), 1);
        assert!(allow.contains(&Value::String("Bash(cargo test:*)".to_string())));
    }

    #[test]
    fn test_remove_nonexistent_file() {
        let (_dir, path) = temp_settings_path();
        assert!(remove_allowed_tools(&path).is_ok());
    }

    #[test]
    fn test_remove_empty_file() {
        let (_dir, path) = temp_settings_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "").unwrap();
        assert!(remove_allowed_tools(&path).is_ok());
    }
}
