use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Default content for `.elftypes`, bundled at compile time.
const DEFAULT_ELFTYPES: &str = include_str!("../../.elftypes");

/// Cached type map: extension → block_type.
static TYPE_MAP: OnceLock<HashMap<String, String>> = OnceLock::new();

/// Return the path to the user's `.elftypes` file (`~/.elf/.elftypes`).
///
/// Respects `ELF_TEST_ELFTYPES_PATH` for testing.
fn elftypes_path() -> Option<PathBuf> {
    if let Ok(test_path) = std::env::var("ELF_TEST_ELFTYPES_PATH") {
        return Some(PathBuf::from(test_path));
    }
    let home = dirs::home_dir()?;
    Some(home.join(".elf").join(".elftypes"))
}

/// Load and parse `.elftypes`, auto-creating the file from the bundled
/// default if it does not exist yet.
fn load_elftypes() -> HashMap<String, String> {
    let path = match elftypes_path() {
        Some(p) => p,
        None => return parse_elftypes(DEFAULT_ELFTYPES),
    };

    // Auto-create from bundled default if missing
    if !path.exists() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, DEFAULT_ELFTYPES);
    }

    match fs::read_to_string(&path) {
        Ok(content) => parse_elftypes(&content),
        Err(_) => parse_elftypes(DEFAULT_ELFTYPES),
    }
}

/// Parse `.elftypes` content into a `HashMap<extension, block_type>`.
///
/// Format:
/// ```text
/// [markdown]
/// .md
/// .markdown
///
/// [code]
/// .rs
/// .py
/// ```
///
/// Leading dots are stripped so that keys match the output of
/// `Path::extension()` (which returns extensions without the dot).
fn parse_elftypes(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut current_type: Option<String> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            current_type = Some(line[1..line.len() - 1].trim().to_string());
        } else if let Some(ref block_type) = current_type {
            let ext = line.strip_prefix('.').unwrap_or(line);
            map.insert(ext.to_lowercase(), block_type.clone());
        }
    }
    map
}

/// Infer Block Type from file extension.
///
/// Looks up the extension in the type map loaded from `~/.elf/.elftypes`.
/// Unknown extensions fall back to `"code"` block type.
///
/// Binary files (images, executables, archives, etc.) are filtered out at scan
/// time via `.elfignore` patterns and never reach this function.
pub fn infer_block_type(extension: &str) -> Option<String> {
    let ext = extension.to_lowercase();
    let map = TYPE_MAP.get_or_init(load_elftypes);

    if let Some(block_type) = map.get(&ext) {
        Some(block_type.clone())
    } else {
        log::debug!("Unknown extension '{}', defaulting to code block type", ext);
        Some("code".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_elftypes() {
        let content = r#"
# comment
[markdown]
.md
.markdown

[code]
.rs
.py
# inline comment
.js
"#;
        let map = parse_elftypes(content);
        assert_eq!(map.get("md").unwrap(), "markdown");
        assert_eq!(map.get("markdown").unwrap(), "markdown");
        assert_eq!(map.get("rs").unwrap(), "code");
        assert_eq!(map.get("py").unwrap(), "code");
        assert_eq!(map.get("js").unwrap(), "code");
        assert_eq!(map.get("unknown"), None);
    }

    #[test]
    fn test_parse_elftypes_case_insensitive() {
        let content = "[markdown]\n.MD\n";
        let map = parse_elftypes(content);
        assert_eq!(map.get("md").unwrap(), "markdown");
    }

    #[test]
    fn test_parse_elftypes_empty() {
        let map = parse_elftypes("");
        assert!(map.is_empty());
    }

    #[test]
    fn test_bundled_default_contains_expected_entries() {
        let map = parse_elftypes(DEFAULT_ELFTYPES);
        assert_eq!(map.get("md").unwrap(), "markdown");
        assert_eq!(map.get("rs").unwrap(), "code");
        assert_eq!(map.get("json").unwrap(), "code");
        assert_eq!(map.get("html").unwrap(), "code");
    }

    #[test]
    fn test_load_elftypes_auto_creates_default() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join(".elftypes");

        assert!(!path.exists());

        std::env::set_var("ELF_TEST_ELFTYPES_PATH", &path);
        let map = load_elftypes();
        std::env::remove_var("ELF_TEST_ELFTYPES_PATH");

        assert!(path.exists());
        assert_eq!(map.get("md").unwrap(), "markdown");
        assert_eq!(map.get("rs").unwrap(), "code");
    }

    #[test]
    fn test_load_elftypes_custom_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join(".elftypes");
        fs::write(&path, "[diagram]\n.mermaid\n.dot\n").unwrap();

        std::env::set_var("ELF_TEST_ELFTYPES_PATH", &path);
        let map = load_elftypes();
        std::env::remove_var("ELF_TEST_ELFTYPES_PATH");

        assert_eq!(map.get("mermaid").unwrap(), "diagram");
        assert_eq!(map.get("dot").unwrap(), "diagram");
    }
}
