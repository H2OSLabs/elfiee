/// Block snapshot utilities.
///
/// Writes physical snapshot files to `block-{uuid}/` directories.
/// These snapshots serve as:
/// - Symlink targets (e.g., SKILLS.md → ~/.claude/skills/)
/// - Migration aids (portable content outside event store)
/// - Quick-access cache for external tools
///
/// Event store remains the source of truth. Snapshots are derived data.
use std::fs;
use std::path::Path;

/// Determine the snapshot filename based on block type and name.
///
/// - markdown → `body.md`
/// - code → `body.{ext}` (extension from block name, fallback to `.txt`)
/// - directory → `body.json`
/// - other → `body.txt`
fn snapshot_filename(block_type: &str, block_name: &str) -> String {
    match block_type {
        "markdown" => "body.md".to_string(),
        "task" => "body.md".to_string(),
        "directory" => "body.json".to_string(),
        "code" => {
            if let Some(ext) = Path::new(block_name).extension().and_then(|e| e.to_str()) {
                format!("body.{}", ext)
            } else {
                "body.txt".to_string()
            }
        }
        _ => "body.txt".to_string(),
    }
}

/// Extract text content from block contents based on block type.
///
/// - markdown → `contents.markdown`
/// - task → `contents.markdown` (same model as markdown blocks)
/// - code → `contents.text`
/// - directory → JSON-serialized `contents.entries`
fn extract_content(block_type: &str, contents: &serde_json::Value) -> Option<String> {
    match block_type {
        "markdown" | "task" => contents
            .get("markdown")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        "code" => contents
            .get("text")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        "directory" => contents
            .get("entries")
            .map(|entries| serde_json::to_string_pretty(entries).unwrap_or_default()),
        _ => None,
    }
}

/// Write a snapshot file for a block to its `block-{uuid}/` directory.
///
/// Creates the block directory if it doesn't exist, then writes the
/// content to the appropriate filename (e.g., `body.md`, `body.rs`).
///
/// # Arguments
/// - `temp_dir`: Parent directory containing all block directories
/// - `block_id`: Block UUID
/// - `block_type`: Block type ("markdown", "code", "directory")
/// - `block_name`: Block name (used to infer file extension for code blocks)
/// - `contents`: Block contents JSON value
///
/// # Returns
/// - `Ok(())` on success
/// - `Err(String)` on I/O error or missing content
pub fn write_block_snapshot(
    temp_dir: &Path,
    block_id: &str,
    block_type: &str,
    block_name: &str,
    contents: &serde_json::Value,
) -> Result<(), String> {
    let block_dir = temp_dir.join(format!("block-{}", block_id));

    // Create block directory if needed
    fs::create_dir_all(&block_dir)
        .map_err(|e| format!("Failed to create block directory: {}", e))?;

    let filename = snapshot_filename(block_type, block_name);
    let content = match extract_content(block_type, contents) {
        Some(c) => c,
        None => return Ok(()), // No content to snapshot, skip silently
    };

    let snapshot_path = block_dir.join(&filename);
    fs::write(&snapshot_path, &content)
        .map_err(|e| format!("Failed to write snapshot {}: {}", filename, e))?;

    log::debug!(
        "Snapshot written: block-{}/{} ({} bytes)",
        block_id,
        filename,
        content.len()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_snapshot_filename_markdown() {
        assert_eq!(snapshot_filename("markdown", "README.md"), "body.md");
        assert_eq!(snapshot_filename("markdown", "anything"), "body.md");
    }

    #[test]
    fn test_snapshot_filename_code_with_extension() {
        assert_eq!(snapshot_filename("code", "main.rs"), "body.rs");
        assert_eq!(snapshot_filename("code", "index.ts"), "body.ts");
        assert_eq!(snapshot_filename("code", "config.json"), "body.json");
        assert_eq!(snapshot_filename("code", "style.css"), "body.css");
    }

    #[test]
    fn test_snapshot_filename_code_without_extension() {
        assert_eq!(snapshot_filename("code", "Makefile"), "body.txt");
        assert_eq!(snapshot_filename("code", ""), "body.txt");
    }

    #[test]
    fn test_snapshot_filename_directory() {
        assert_eq!(snapshot_filename("directory", "src"), "body.json");
    }

    #[test]
    fn test_extract_content_markdown() {
        let contents = json!({"markdown": "# Hello"});
        assert_eq!(
            extract_content("markdown", &contents),
            Some("# Hello".to_string())
        );
    }

    #[test]
    fn test_extract_content_code() {
        let contents = json!({"text": "fn main() {}"});
        assert_eq!(
            extract_content("code", &contents),
            Some("fn main() {}".to_string())
        );
    }

    #[test]
    fn test_extract_content_directory() {
        let contents = json!({"entries": {"README.md": {"id": "abc", "type": "file"}}});
        let result = extract_content("directory", &contents);
        assert!(result.is_some());
        assert!(result.unwrap().contains("README.md"));
    }

    #[test]
    fn test_extract_content_missing() {
        let contents = json!({});
        assert_eq!(extract_content("markdown", &contents), None);
        assert_eq!(extract_content("code", &contents), None);
    }

    #[test]
    fn test_write_block_snapshot_markdown() {
        let temp = tempfile::TempDir::new().unwrap();
        let result = write_block_snapshot(
            temp.path(),
            "test-id",
            "markdown",
            "README.md",
            &json!({"markdown": "# Hello World"}),
        );
        assert!(result.is_ok());

        let snapshot = fs::read_to_string(temp.path().join("block-test-id/body.md")).unwrap();
        assert_eq!(snapshot, "# Hello World");
    }

    #[test]
    fn test_write_block_snapshot_code() {
        let temp = tempfile::TempDir::new().unwrap();
        let result = write_block_snapshot(
            temp.path(),
            "test-id",
            "code",
            "main.rs",
            &json!({"text": "fn main() {}"}),
        );
        assert!(result.is_ok());

        let snapshot = fs::read_to_string(temp.path().join("block-test-id/body.rs")).unwrap();
        assert_eq!(snapshot, "fn main() {}");
    }

    #[test]
    fn test_write_block_snapshot_directory() {
        let temp = tempfile::TempDir::new().unwrap();
        let entries = json!({"README.md": {"id": "abc", "type": "file"}});
        let result = write_block_snapshot(
            temp.path(),
            "test-id",
            "directory",
            "src",
            &json!({"entries": entries}),
        );
        assert!(result.is_ok());

        let snapshot = fs::read_to_string(temp.path().join("block-test-id/body.json")).unwrap();
        assert!(snapshot.contains("README.md"));
    }

    #[test]
    fn test_write_block_snapshot_no_content_skips() {
        let temp = tempfile::TempDir::new().unwrap();
        let result = write_block_snapshot(
            temp.path(),
            "test-id",
            "markdown",
            "empty.md",
            &json!({}), // No markdown key
        );
        // Should succeed silently (skip)
        assert!(result.is_ok());
        // No file should be created
        assert!(!temp.path().join("block-test-id/body.md").exists());
    }

    #[test]
    fn test_write_block_snapshot_creates_directory() {
        let temp = tempfile::TempDir::new().unwrap();
        let block_dir = temp.path().join("block-new-id");
        assert!(!block_dir.exists());

        let result = write_block_snapshot(
            temp.path(),
            "new-id",
            "markdown",
            "test.md",
            &json!({"markdown": "content"}),
        );
        assert!(result.is_ok());
        assert!(block_dir.exists());
    }
}
