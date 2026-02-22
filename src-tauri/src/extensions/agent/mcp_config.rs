//! MCP Configuration Merger
//!
//! Utilities for managing `.claude/mcp.json` configuration files.
//! Supports idempotent merge and remove operations for MCP server entries.

use serde_json::Value;
use std::fs;
use std::path::Path;

/// Merge an MCP server configuration into a `.claude/mcp.json` file.
///
/// Idempotent: if `server_name` already exists, it is overwritten.
/// If the file does not exist, it is created.
/// Other server entries are preserved.
///
/// # Arguments
/// * `config_path` - Path to the `.claude/mcp.json` file
/// * `server_name` - Server name key (e.g., "elfiee")
/// * `server_config` - Server configuration value (e.g., `{"command": "elfiee", "args": [...]}`)
///
/// # Errors
/// Returns error if the existing file contains invalid JSON or I/O fails.
pub fn merge_server(
    config_path: &Path,
    server_name: &str,
    server_config: Value,
) -> Result<(), String> {
    let mut root = if config_path.exists() {
        let content = fs::read_to_string(config_path)
            .map_err(|e| format!("Failed to read {}: {}", config_path.display(), e))?;

        if content.trim().is_empty() {
            serde_json::json!({})
        } else {
            serde_json::from_str::<Value>(&content)
                .map_err(|e| format!("Invalid JSON in {}: {}", config_path.display(), e))?
        }
    } else {
        serde_json::json!({})
    };

    // Ensure mcpServers object exists
    let root_obj = root
        .as_object_mut()
        .ok_or_else(|| format!("Expected JSON object in {}", config_path.display()))?;

    if !root_obj.contains_key("mcpServers") {
        root_obj.insert("mcpServers".to_string(), serde_json::json!({}));
    }

    let mcp_servers = root_obj
        .get_mut("mcpServers")
        .and_then(|v| v.as_object_mut())
        .ok_or_else(|| "mcpServers must be a JSON object".to_string())?;

    // Insert or overwrite the server entry
    mcp_servers.insert(server_name.to_string(), server_config);

    // Write back with pretty formatting
    let output = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

    // Ensure parent directory exists
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
    }

    fs::write(config_path, output)
        .map_err(|e| format!("Failed to write {}: {}", config_path.display(), e))?;

    Ok(())
}

/// Remove an MCP server configuration from a `.claude/mcp.json` file.
///
/// Idempotent: if `server_name` does not exist, succeeds silently.
/// If the file does not exist, succeeds silently.
/// After removal, if `mcpServers` is empty, it remains as `{"mcpServers": {}}`.
///
/// # Arguments
/// * `config_path` - Path to the `.claude/mcp.json` file
/// * `server_name` - Server name key to remove (e.g., "elfiee")
///
/// # Errors
/// Returns error if the existing file contains invalid JSON or I/O fails.
pub fn remove_server(config_path: &Path, server_name: &str) -> Result<(), String> {
    if !config_path.exists() {
        return Ok(()); // File doesn't exist, nothing to remove
    }

    let content = fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read {}: {}", config_path.display(), e))?;

    if content.trim().is_empty() {
        return Ok(()); // Empty file, nothing to remove
    }

    let mut root: Value = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid JSON in {}: {}", config_path.display(), e))?;

    let root_obj = root
        .as_object_mut()
        .ok_or_else(|| format!("Expected JSON object in {}", config_path.display()))?;

    if let Some(mcp_servers) = root_obj
        .get_mut("mcpServers")
        .and_then(|v| v.as_object_mut())
    {
        mcp_servers.remove(server_name);
    }
    // If mcpServers doesn't exist, that's fine — nothing to remove

    // Write back
    let output = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

    fs::write(config_path, output)
        .map_err(|e| format!("Failed to write {}: {}", config_path.display(), e))?;

    Ok(())
}

/// Replace placeholders in an MCP config template with actual values.
///
/// Supported placeholders:
/// - `{elf_path}` — replaced with the .elf file's physical path
///
/// Recursively processes all string values in the JSON structure.
pub fn resolve_template(template: &Value, elf_path: &str) -> Value {
    match template {
        Value::String(s) => Value::String(s.replace("{elf_path}", elf_path)),
        Value::Array(arr) => {
            Value::Array(arr.iter().map(|v| resolve_template(v, elf_path)).collect())
        }
        Value::Object(obj) => {
            let mut new_obj = serde_json::Map::new();
            for (k, v) in obj {
                new_obj.insert(k.clone(), resolve_template(v, elf_path));
            }
            Value::Object(new_obj)
        }
        other => other.clone(),
    }
}

/// Read the existing port for a named server from a `.mcp.json` config file.
///
/// Parses the SSE URL (`http://127.0.0.1:{port}/sse`) to extract the port number.
/// Returns `None` if the file doesn't exist, is invalid, or the server entry is missing.
///
/// The URL format is hardcoded to match what `build_elfiee_server_config` writes.
/// If external tools modify `.mcp.json` with a different URL format (e.g., `localhost`,
/// `::1`), parsing fails gracefully and the agent gets a new port on recovery.
pub fn read_existing_port(config_path: &Path, server_name: &str) -> Option<u16> {
    let content = fs::read_to_string(config_path).ok()?;
    let root: Value = serde_json::from_str(&content).ok()?;
    let url = root
        .get("mcpServers")?
        .get(server_name)?
        .get("url")?
        .as_str()?;
    let port = url
        .strip_prefix("http://127.0.0.1:")
        .and_then(|s| s.strip_suffix("/sse"))
        .and_then(|s| s.parse::<u16>().ok());
    if port.is_none() {
        log::debug!(
            "Could not parse port from URL '{}' in {} (expected http://127.0.0.1:PORT/sse)",
            url,
            config_path.display()
        );
    }
    port
}

/// Build the MCP server config for Elfiee with a specific port.
///
/// The Elfiee MCP server runs as an embedded SSE server inside the GUI process.
/// Each enabled agent gets its own port (47201–47299).
/// The management port (47200) is used for legacy/fallback mode.
pub fn build_elfiee_server_config(port: u16) -> Value {
    let url = format!("http://127.0.0.1:{}/sse", port);
    serde_json::json!({
        "type": "sse",
        "url": url
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_config_path() -> (TempDir, std::path::PathBuf) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(".claude").join("mcp.json");
        (dir, path)
    }

    // --- merge_server tests ---

    #[test]
    fn test_merge_new_file() {
        let (_dir, path) = temp_config_path();
        assert!(!path.exists());

        let config = serde_json::json!({"command": "elfiee", "args": ["mcp-server"]});
        merge_server(&path, "elfiee", config.clone()).unwrap();

        assert!(path.exists());
        let content: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"]["elfiee"], config);
    }

    #[test]
    fn test_merge_empty_file() {
        let (_dir, path) = temp_config_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "").unwrap();

        let config = serde_json::json!({"command": "elfiee"});
        merge_server(&path, "elfiee", config.clone()).unwrap();

        let content: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"]["elfiee"], config);
    }

    #[test]
    fn test_merge_existing_no_server() {
        let (_dir, path) = temp_config_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, r#"{"mcpServers": {}}"#).unwrap();

        let config = serde_json::json!({"command": "elfiee"});
        merge_server(&path, "elfiee", config.clone()).unwrap();

        let content: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"]["elfiee"], config);
    }

    #[test]
    fn test_merge_existing_with_server_overwrites() {
        let (_dir, path) = temp_config_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, r#"{"mcpServers": {"elfiee": {"command": "old"}}}"#).unwrap();

        let new_config = serde_json::json!({"command": "new"});
        merge_server(&path, "elfiee", new_config.clone()).unwrap();

        let content: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"]["elfiee"]["command"], "new");
    }

    #[test]
    fn test_merge_preserves_other_servers() {
        let (_dir, path) = temp_config_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            r#"{"mcpServers": {"other-tool": {"command": "other"}}}"#,
        )
        .unwrap();

        let config = serde_json::json!({"command": "elfiee"});
        merge_server(&path, "elfiee", config).unwrap();

        let content: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"]["other-tool"]["command"], "other");
        assert_eq!(content["mcpServers"]["elfiee"]["command"], "elfiee");
    }

    #[test]
    fn test_merge_invalid_json_fails() {
        let (_dir, path) = temp_config_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "not valid json {{{").unwrap();

        let config = serde_json::json!({"command": "elfiee"});
        let result = merge_server(&path, "elfiee", config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid JSON"));
    }

    #[test]
    fn test_merge_no_mcp_servers_field() {
        let (_dir, path) = temp_config_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, r#"{"other": "data"}"#).unwrap();

        let config = serde_json::json!({"command": "elfiee"});
        merge_server(&path, "elfiee", config.clone()).unwrap();

        let content: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"]["elfiee"], config);
        assert_eq!(content["other"], "data"); // preserved
    }

    // --- remove_server tests ---

    #[test]
    fn test_remove_existing() {
        let (_dir, path) = temp_config_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            r#"{"mcpServers": {"elfiee": {"command": "elfiee"}}}"#,
        )
        .unwrap();

        remove_server(&path, "elfiee").unwrap();

        let content: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert!(content["mcpServers"]
            .as_object()
            .unwrap()
            .get("elfiee")
            .is_none());
    }

    #[test]
    fn test_remove_nonexistent_server() {
        let (_dir, path) = temp_config_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, r#"{"mcpServers": {}}"#).unwrap();

        let result = remove_server(&path, "elfiee");
        assert!(result.is_ok()); // Silent success
    }

    #[test]
    fn test_remove_file_not_found() {
        let (_dir, path) = temp_config_path();
        assert!(!path.exists());

        let result = remove_server(&path, "elfiee");
        assert!(result.is_ok()); // Silent success
    }

    #[test]
    fn test_remove_preserves_other_servers() {
        let (_dir, path) = temp_config_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            r#"{"mcpServers": {"elfiee": {"command": "elfiee"}, "other": {"command": "other"}}}"#,
        )
        .unwrap();

        remove_server(&path, "elfiee").unwrap();

        let content: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert!(content["mcpServers"]
            .as_object()
            .unwrap()
            .get("elfiee")
            .is_none());
        assert_eq!(content["mcpServers"]["other"]["command"], "other");
    }

    #[test]
    fn test_remove_leaves_empty_object() {
        let (_dir, path) = temp_config_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            r#"{"mcpServers": {"elfiee": {"command": "elfiee"}}}"#,
        )
        .unwrap();

        remove_server(&path, "elfiee").unwrap();

        let content: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let servers = content["mcpServers"].as_object().unwrap();
        assert!(servers.is_empty());
    }

    #[test]
    fn test_remove_invalid_json_fails() {
        let (_dir, path) = temp_config_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "not json!").unwrap();

        let result = remove_server(&path, "elfiee");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid JSON"));
    }

    #[test]
    fn test_remove_empty_file_silent_success() {
        let (_dir, path) = temp_config_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "").unwrap();

        let result = remove_server(&path, "elfiee");
        assert!(result.is_ok());
    }

    // --- resolve_template tests ---

    #[test]
    fn test_resolve_template_elf_path() {
        let template = serde_json::json!({
            "command": "elfiee",
            "args": ["mcp-server", "--elf", "{elf_path}"]
        });

        let resolved = resolve_template(&template, "/home/user/project.elf");

        assert_eq!(resolved["command"], "elfiee");
        let args = resolved["args"].as_array().unwrap();
        assert_eq!(args[2], "/home/user/project.elf");
    }

    #[test]
    fn test_resolve_template_nested() {
        let template = serde_json::json!({
            "mcpServers": {
                "elfiee": {
                    "command": "elfiee",
                    "args": ["mcp-server", "--elf", "{elf_path}"]
                }
            }
        });

        let resolved = resolve_template(&template, "/path/to/file.elf");

        assert_eq!(
            resolved["mcpServers"]["elfiee"]["args"][2],
            "/path/to/file.elf"
        );
    }

    #[test]
    fn test_resolve_template_no_placeholders() {
        let template = serde_json::json!({"command": "other", "args": ["--flag"]});
        let resolved = resolve_template(&template, "/some/path.elf");
        assert_eq!(resolved, template);
    }

    #[test]
    fn test_resolve_template_preserves_non_strings() {
        let template = serde_json::json!({
            "name": "{elf_path}",
            "count": 42,
            "enabled": true,
            "data": null
        });

        let resolved = resolve_template(&template, "/path.elf");
        assert_eq!(resolved["name"], "/path.elf");
        assert_eq!(resolved["count"], 42);
        assert_eq!(resolved["enabled"], true);
        assert!(resolved["data"].is_null());
    }

    // --- build_elfiee_server_config tests ---

    #[test]
    fn test_build_elfiee_server_config() {
        let config = build_elfiee_server_config(47201);
        assert_eq!(config["type"], "sse");
        assert_eq!(config["url"], "http://127.0.0.1:47201/sse");
    }

    #[test]
    fn test_build_elfiee_server_config_management_port() {
        let config = build_elfiee_server_config(crate::mcp::MCP_PORT);
        assert_eq!(config["type"], "sse");
        assert_eq!(
            config["url"],
            format!("http://127.0.0.1:{}/sse", crate::mcp::MCP_PORT)
        );
    }

    // --- read_existing_port tests ---

    #[test]
    fn test_read_existing_port_success() {
        let (_dir, path) = temp_config_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            r#"{"mcpServers": {"elfiee": {"type": "sse", "url": "http://127.0.0.1:47205/sse"}}}"#,
        )
        .unwrap();

        assert_eq!(read_existing_port(&path, "elfiee"), Some(47205));
    }

    #[test]
    fn test_read_existing_port_file_not_found() {
        let (_dir, path) = temp_config_path();
        assert_eq!(read_existing_port(&path, "elfiee"), None);
    }

    #[test]
    fn test_read_existing_port_server_not_present() {
        let (_dir, path) = temp_config_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            r#"{"mcpServers": {"other": {"url": "http://127.0.0.1:9999/sse"}}}"#,
        )
        .unwrap();

        assert_eq!(read_existing_port(&path, "elfiee"), None);
    }

    #[test]
    fn test_read_existing_port_invalid_json() {
        let (_dir, path) = temp_config_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "not json!").unwrap();

        assert_eq!(read_existing_port(&path, "elfiee"), None);
    }

    #[test]
    fn test_read_existing_port_malformed_url() {
        let (_dir, path) = temp_config_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            r#"{"mcpServers": {"elfiee": {"url": "http://localhost:47205/api"}}}"#,
        )
        .unwrap();

        assert_eq!(read_existing_port(&path, "elfiee"), None);
    }

    #[test]
    fn test_read_existing_port_localhost_url_returns_none() {
        // Verifies that URLs using "localhost" instead of "127.0.0.1" fail gracefully
        let (_dir, path) = temp_config_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            r#"{"mcpServers": {"elfiee": {"url": "http://localhost:47205/sse"}}}"#,
        )
        .unwrap();

        assert_eq!(read_existing_port(&path, "elfiee"), None);
    }

    #[test]
    fn test_read_existing_port_ipv6_url_returns_none() {
        // Verifies that IPv6 URLs fail gracefully
        let (_dir, path) = temp_config_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            r#"{"mcpServers": {"elfiee": {"url": "http://[::1]:47205/sse"}}}"#,
        )
        .unwrap();

        assert_eq!(read_existing_port(&path, "elfiee"), None);
    }
}
