//! Template Copy Utility
//!
//! Copies embedded template files to `.elf/Agents/elfiee-client/` block directory
//! at runtime. Templates are embedded at compile time via `include_str!()`.
//!
//! ## Usage
//!
//! Called during `.elf/` Dir Block initialization (I10-01 in `elf_meta.rs`)
//! to populate the `elfiee-client` skill directory with SKILL.md, mcp.json,
//! and reference documents.

use std::path::Path;

/// SKILL.md template — Claude Code skill definition.
const SKILL_MD: &str = include_str!("../../templates/elfiee-client/SKILL.md");

/// mcp.json template — MCP server configuration.
const MCP_JSON: &str = include_str!("../../templates/elfiee-client/mcp.json");

/// capabilities.md — Elfiee capabilities reference document.
const CAPABILITIES_MD: &str =
    include_str!("../../templates/elfiee-client/references/capabilities.md");

/// Initialize the `elfiee-client` skill directory inside a `.elf/` block directory.
///
/// Creates the following structure under `{block_dir}/Agents/elfiee-client/`:
///
/// ```text
/// {block_dir}/Agents/elfiee-client/
/// ├── SKILL.md
/// ├── mcp.json
/// ├── scripts/          (empty directory)
/// ├── assets/           (empty directory)
/// └── references/
///     └── capabilities.md
/// ```
///
/// Also creates `{block_dir}/Agents/session/` for session sync storage.
///
/// # Arguments
///
/// * `block_dir` - The `.elf/` Dir Block's physical directory path (`block-{uuid}/`).
/// * `_elf_path` - The `.elf` file's physical path (reserved for future placeholder replacement).
///
/// # Errors
///
/// Returns `Err(String)` if any directory creation or file write fails.
pub fn init_elfiee_client(block_dir: &Path, _elf_path: &str) -> Result<(), String> {
    let base = block_dir.join("Agents").join("elfiee-client");

    // Create all directories (idempotent via create_dir_all)
    let dirs = [
        base.clone(),
        base.join("scripts"),
        base.join("assets"),
        base.join("references"),
        block_dir.join("Agents").join("session"),
    ];

    for dir in &dirs {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("Failed to create directory {}: {}", dir.display(), e))?;
    }

    // Write template files
    write_template_file(&base.join("SKILL.md"), SKILL_MD)?;
    write_template_file(&base.join("mcp.json"), MCP_JSON)?;
    write_template_file(
        &base.join("references").join("capabilities.md"),
        CAPABILITIES_MD,
    )?;

    Ok(())
}

/// Write a single template file to the target path.
///
/// Overwrites existing files (idempotent — ensures templates are always up to date).
fn write_template_file(target_path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
    }

    std::fs::write(target_path, content)
        .map_err(|e| format!("Failed to write {}: {}", target_path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_block_dir() -> TempDir {
        TempDir::new().expect("Failed to create temp dir")
    }

    #[test]
    fn test_init_creates_directory_structure() {
        let tmp = setup_block_dir();
        let block_dir = tmp.path();

        init_elfiee_client(block_dir, "/test/project.elf").unwrap();

        let base = block_dir.join("Agents").join("elfiee-client");
        assert!(base.exists(), "elfiee-client/ should exist");
        assert!(base.join("scripts").exists(), "scripts/ should exist");
        assert!(base.join("assets").exists(), "assets/ should exist");
        assert!(base.join("references").exists(), "references/ should exist");
        assert!(
            block_dir.join("Agents").join("session").exists(),
            "session/ should exist"
        );
    }

    #[test]
    fn test_init_writes_skill_md() {
        let tmp = setup_block_dir();
        let block_dir = tmp.path();

        init_elfiee_client(block_dir, "/test/project.elf").unwrap();

        let skill_path = block_dir
            .join("Agents")
            .join("elfiee-client")
            .join("SKILL.md");
        assert!(skill_path.exists(), "SKILL.md should exist");

        let content = std::fs::read_to_string(&skill_path).unwrap();
        assert!(!content.is_empty(), "SKILL.md should not be empty");
        assert!(
            content.contains("name: elfiee-client"),
            "SKILL.md should contain frontmatter name"
        );
        assert!(
            content.contains("NEVER use filesystem commands"),
            "SKILL.md should contain critical rule"
        );
    }

    #[test]
    fn test_init_writes_mcp_json() {
        let tmp = setup_block_dir();
        let block_dir = tmp.path();

        init_elfiee_client(block_dir, "/test/project.elf").unwrap();

        let mcp_path = block_dir
            .join("Agents")
            .join("elfiee-client")
            .join("mcp.json");
        assert!(mcp_path.exists(), "mcp.json should exist");

        let content = std::fs::read_to_string(&mcp_path).unwrap();
        let json: serde_json::Value =
            serde_json::from_str(&content).expect("mcp.json should be valid JSON");
        assert!(
            json.get("mcpServers").is_some(),
            "mcp.json should contain mcpServers"
        );
        assert!(
            json["mcpServers"].get("elfiee").is_some(),
            "mcp.json should contain elfiee server"
        );
    }

    #[test]
    fn test_init_writes_capabilities_md() {
        let tmp = setup_block_dir();
        let block_dir = tmp.path();

        init_elfiee_client(block_dir, "/test/project.elf").unwrap();

        let cap_path = block_dir
            .join("Agents")
            .join("elfiee-client")
            .join("references")
            .join("capabilities.md");
        assert!(cap_path.exists(), "capabilities.md should exist");

        let content = std::fs::read_to_string(&cap_path).unwrap();
        assert!(!content.is_empty(), "capabilities.md should not be empty");
        assert!(
            content.contains("core.create"),
            "capabilities.md should reference core.create"
        );
        assert!(
            content.contains("markdown.write"),
            "capabilities.md should reference markdown.write"
        );
    }

    #[test]
    fn test_init_creates_empty_dirs() {
        let tmp = setup_block_dir();
        let block_dir = tmp.path();

        init_elfiee_client(block_dir, "/test/project.elf").unwrap();

        let scripts = block_dir
            .join("Agents")
            .join("elfiee-client")
            .join("scripts");
        let assets = block_dir
            .join("Agents")
            .join("elfiee-client")
            .join("assets");

        assert!(scripts.is_dir(), "scripts/ should be a directory");
        assert!(assets.is_dir(), "assets/ should be a directory");

        // Empty directories
        assert_eq!(
            std::fs::read_dir(&scripts).unwrap().count(),
            0,
            "scripts/ should be empty"
        );
        assert_eq!(
            std::fs::read_dir(&assets).unwrap().count(),
            0,
            "assets/ should be empty"
        );
    }

    #[test]
    fn test_init_creates_session_dir() {
        let tmp = setup_block_dir();
        let block_dir = tmp.path();

        init_elfiee_client(block_dir, "/test/project.elf").unwrap();

        let session = block_dir.join("Agents").join("session");
        assert!(session.is_dir(), "session/ should be a directory");
    }

    #[test]
    fn test_init_idempotent() {
        let tmp = setup_block_dir();
        let block_dir = tmp.path();

        // Call twice — should not error
        init_elfiee_client(block_dir, "/test/project.elf").unwrap();
        init_elfiee_client(block_dir, "/test/project.elf").unwrap();

        // Files should still be valid
        let skill_path = block_dir
            .join("Agents")
            .join("elfiee-client")
            .join("SKILL.md");
        let content = std::fs::read_to_string(&skill_path).unwrap();
        assert!(content.contains("name: elfiee-client"));
    }

    #[test]
    fn test_skill_md_has_frontmatter() {
        let content = SKILL_MD;
        assert!(
            content.starts_with("---"),
            "SKILL.md should start with YAML frontmatter"
        );
        // Find second ---
        let after_first = &content[3..];
        assert!(
            after_first.contains("---"),
            "SKILL.md should have closing frontmatter delimiter"
        );
    }

    #[test]
    fn test_skill_md_mentions_all_tools() {
        let content = SKILL_MD;
        let tools = [
            "elfiee_file_list",
            "elfiee_block_list",
            "elfiee_block_get",
            "elfiee_block_create",
            "elfiee_block_delete",
            "elfiee_block_rename",
            "elfiee_block_change_type",
            "elfiee_block_update_metadata",
            "elfiee_block_link",
            "elfiee_block_unlink",
            "elfiee_markdown_read",
            "elfiee_markdown_write",
            "elfiee_code_read",
            "elfiee_code_write",
            "elfiee_directory_create",
            "elfiee_directory_delete",
            "elfiee_directory_rename",
            "elfiee_directory_write",
            "elfiee_directory_import",
            "elfiee_directory_export",
            "elfiee_terminal_init",
            "elfiee_terminal_execute",
            "elfiee_terminal_save",
            "elfiee_terminal_close",
            "elfiee_grant",
            "elfiee_revoke",
            "elfiee_editor_create",
            "elfiee_editor_delete",
            "elfiee_exec",
        ];

        for tool in &tools {
            assert!(
                content.contains(tool),
                "SKILL.md should mention tool: {}",
                tool
            );
        }
    }

    #[test]
    fn test_mcp_json_valid() {
        let json: serde_json::Value =
            serde_json::from_str(MCP_JSON).expect("MCP_JSON template should be valid JSON");
        assert!(json.is_object());
    }

    #[test]
    fn test_mcp_json_has_elfiee_server() {
        let json: serde_json::Value = serde_json::from_str(MCP_JSON).unwrap();
        let servers = json.get("mcpServers").expect("should have mcpServers");
        let elfiee = servers.get("elfiee").expect("should have elfiee server");
        assert!(
            elfiee.get("type").is_some() || elfiee.get("command").is_some(),
            "elfiee server should have type or command field"
        );
    }
}
