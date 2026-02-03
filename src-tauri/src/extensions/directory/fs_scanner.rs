use ignore::overrides::OverrideBuilder;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Information about a scanned file.
///
/// This struct holds metadata and path information for files found during
/// directory scanning. It is used by import and refresh operations to
/// process external files into blocks.
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// Absolute path to the file
    pub absolute_path: PathBuf,
    /// Relative path from the scan root
    pub relative_path: String,
    /// File name including extension
    pub file_name: String,
    /// File extension (without dot)
    pub extension: String,
    /// File size in bytes
    pub size: u64,
    /// Whether this entry is a directory
    pub is_directory: bool,
}

/// Options for directory scanning
#[derive(Debug, Clone)]
pub struct ScanOptions {
    /// Maximum recursion depth
    pub max_depth: usize,
    /// Whether to follow symbolic links
    pub follow_symlinks: bool,
    /// Whether to ignore hidden files (starting with dot)
    pub ignore_hidden: bool,
    /// Directory/file patterns to always exclude (applied as highest-priority
    /// overrides, effective even when no .gitignore exists). Loaded from the
    /// bundled `.elfignore` at compile time.
    pub ignore_patterns: Vec<String>,
    /// Maximum file size in bytes to include
    pub max_file_size: u64,
    /// Maximum number of files to scan
    pub max_files: usize,
    /// Whether to respect .gitignore files when scanning
    pub use_gitignore: bool,
}

/// Content of `.elfignore`, bundled at compile time.
const DEFAULT_ELFIGNORE: &str = include_str!("../../.elfignore");

/// Parse `.elfignore` content into a list of patterns.
/// Strips comments, empty lines, and trailing slashes.
fn parse_elfignore(content: &str) -> Vec<String> {
    content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.trim_end_matches('/').to_string())
        .collect()
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            max_depth: 100,
            follow_symlinks: false,
            ignore_hidden: true,
            ignore_patterns: parse_elfignore(DEFAULT_ELFIGNORE),
            max_file_size: 10 * 1024 * 1024, // 10 MB
            max_files: 10_000,
            use_gitignore: true,
        }
    }
}

/// Scan a directory and return a list of files
///
/// Supports `.gitignore` parsing: when `use_gitignore` is enabled (default),
/// the scanner reads `.gitignore` files (root + nested) and filters out
/// matching entries automatically.
pub fn scan_directory(root: &Path, options: &ScanOptions) -> Result<Vec<FileInfo>, String> {
    let mut files = Vec::new();
    let mut count = 0;

    let mut builder = WalkBuilder::new(root);
    builder.max_depth(Some(options.max_depth));
    builder.follow_links(options.follow_symlinks);
    builder.hidden(options.ignore_hidden);
    builder.max_filesize(Some(options.max_file_size));

    // .gitignore support
    builder.git_ignore(options.use_gitignore);
    builder.git_global(options.use_gitignore);
    builder.git_exclude(options.use_gitignore);
    builder.ignore(options.use_gitignore);

    // Also read .gitignore in non-git directories (no .git folder required)
    if options.use_gitignore {
        builder.add_custom_ignore_filename(".gitignore");
    }

    // Apply ignore_patterns as highest-priority overrides (effective even without .gitignore)
    if !options.ignore_patterns.is_empty() {
        let mut overrides = OverrideBuilder::new(root);
        for dir in &options.ignore_patterns {
            overrides
                .add(&format!("!**/{}", dir))
                .map_err(|e| format!("Invalid ignore dir '{}': {}", dir, e))?;
        }
        let built = overrides
            .build()
            .map_err(|e| format!("Failed to build overrides: {}", e))?;
        builder.overrides(built);
    }

    for result in builder.build() {
        let entry = result.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        // Check file count limit
        count += 1;
        if count > options.max_files {
            return Err(format!("Too many files (limit: {})", options.max_files));
        }

        let metadata = entry
            .metadata()
            .map_err(|e| format!("Failed to read metadata: {}", e))?;

        let relative_path = path
            .strip_prefix(root)
            .map_err(|e| format!("Failed to strip prefix: {}", e))?
            .to_string_lossy()
            .to_string();

        // Skip root directory itself
        if relative_path.is_empty() {
            continue;
        }

        let file_info = FileInfo {
            absolute_path: path.to_path_buf(),
            relative_path: relative_path.clone(),
            file_name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            extension: path
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            size: metadata.len(),
            is_directory: metadata.is_dir(),
        };

        files.push(file_info);
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scan_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let options = ScanOptions::default();

        let files = scan_directory(temp_dir.path(), &options).unwrap();
        assert_eq!(files.len(), 0);
    }

    #[test]
    fn test_scan_with_files() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file1.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("file2.md"), "content").unwrap();

        let options = ScanOptions::default();
        let files = scan_directory(temp_dir.path(), &options).unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.file_name == "file1.txt"));
        assert!(files.iter().any(|f| f.file_name == "file2.md"));
    }

    #[test]
    fn test_scan_ignores_hidden_files() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join(".hidden"), "content").unwrap();
        fs::write(temp_dir.path().join("visible.txt"), "content").unwrap();

        let options = ScanOptions::default();
        let files = scan_directory(temp_dir.path(), &options).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_name, "visible.txt");
    }

    #[test]
    fn test_scan_skips_ignore_patterns() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir(temp_dir.path().join("node_modules")).unwrap();
        fs::write(temp_dir.path().join("node_modules/pkg.json"), "{}").unwrap();
        fs::create_dir(temp_dir.path().join("__pycache__")).unwrap();
        fs::write(temp_dir.path().join("__pycache__/mod.pyc"), "").unwrap();
        fs::write(temp_dir.path().join("main.rs"), "code").unwrap();

        let options = ScanOptions {
            ignore_patterns: vec!["node_modules".to_string(), "__pycache__".to_string()],
            ..Default::default()
        };
        let files = scan_directory(temp_dir.path(), &options).unwrap();

        let names: Vec<&str> = files.iter().map(|f| f.file_name.as_str()).collect();
        assert!(names.contains(&"main.rs"));
        assert!(!names.contains(&"pkg.json"));
        assert!(!names.contains(&"mod.pyc"));
    }

    #[test]
    fn test_parse_elfignore() {
        let content = r#"
# This is a comment
node_modules/
__pycache__/

  target
# Another comment
  .venv/
"#;
        let patterns = parse_elfignore(content);
        assert_eq!(
            patterns,
            vec!["node_modules", "__pycache__", "target", ".venv"]
        );
    }

    #[test]
    fn test_default_elfignore_contains_expected_patterns() {
        let patterns = parse_elfignore(DEFAULT_ELFIGNORE);
        assert!(patterns.contains(&"node_modules".to_string()));
        assert!(patterns.contains(&"__pycache__".to_string()));
        assert!(patterns.contains(&"target".to_string()));
        assert!(patterns.contains(&"*.png".to_string()));
        assert!(patterns.contains(&"*.exe".to_string()));
        assert!(patterns.contains(&".DS_Store".to_string()));
    }

    #[test]
    fn test_scan_respects_gitignore() {
        let temp_dir = TempDir::new().unwrap();
        // Create a .gitignore that ignores *.log files
        fs::write(temp_dir.path().join(".gitignore"), "*.log\n").unwrap();
        fs::write(temp_dir.path().join("app.rs"), "fn main() {}").unwrap();
        fs::write(temp_dir.path().join("debug.log"), "log content").unwrap();

        let options = ScanOptions::default();
        let files = scan_directory(temp_dir.path(), &options).unwrap();

        // .gitignore is hidden (starts with dot) → filtered by ignore_hidden
        // debug.log is filtered by .gitignore rule
        // Only app.rs should remain
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_name, "app.rs");
    }

    #[test]
    fn test_scan_respects_nested_gitignore() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir(temp_dir.path().join("subdir")).unwrap();
        // Root .gitignore ignores *.tmp
        fs::write(temp_dir.path().join(".gitignore"), "*.tmp\n").unwrap();
        // Nested .gitignore ignores *.bak
        fs::write(temp_dir.path().join("subdir/.gitignore"), "*.bak\n").unwrap();
        fs::write(temp_dir.path().join("main.rs"), "code").unwrap();
        fs::write(temp_dir.path().join("temp.tmp"), "temp").unwrap();
        fs::write(temp_dir.path().join("subdir/keep.rs"), "code").unwrap();
        fs::write(temp_dir.path().join("subdir/remove.bak"), "backup").unwrap();

        let options = ScanOptions::default();
        let files = scan_directory(temp_dir.path(), &options).unwrap();

        let file_names: Vec<&str> = files
            .iter()
            .filter(|f| !f.is_directory)
            .map(|f| f.file_name.as_str())
            .collect();

        assert!(file_names.contains(&"main.rs"));
        assert!(file_names.contains(&"keep.rs"));
        assert!(!file_names.contains(&"temp.tmp"));
        assert!(!file_names.contains(&"remove.bak"));
    }

    #[test]
    fn test_scan_skips_glob_patterns() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("app.rs"), "code").unwrap();
        fs::write(temp_dir.path().join("logo.png"), &[0x89, 0x50, 0x4E, 0x47]).unwrap();
        fs::write(temp_dir.path().join("photo.jpg"), &[0xFF, 0xD8]).unwrap();
        fs::write(temp_dir.path().join("data.db"), &[0x00]).unwrap();

        let options = ScanOptions {
            ignore_patterns: vec!["*.png".to_string(), "*.jpg".to_string(), "*.db".to_string()],
            ..Default::default()
        };
        let files = scan_directory(temp_dir.path(), &options).unwrap();

        let names: Vec<&str> = files.iter().map(|f| f.file_name.as_str()).collect();
        assert!(names.contains(&"app.rs"));
        assert!(!names.contains(&"logo.png"));
        assert!(!names.contains(&"photo.jpg"));
        assert!(!names.contains(&"data.db"));
    }

    #[test]
    fn test_scan_gitignore_disabled() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join(".gitignore"), "*.log\n").unwrap();
        fs::write(temp_dir.path().join("app.rs"), "code").unwrap();
        fs::write(temp_dir.path().join("debug.log"), "log").unwrap();

        let mut options = ScanOptions::default();
        options.use_gitignore = false;
        options.ignore_hidden = false; // so .gitignore file itself is visible
        let files = scan_directory(temp_dir.path(), &options).unwrap();

        let file_names: Vec<&str> = files
            .iter()
            .filter(|f| !f.is_directory)
            .map(|f| f.file_name.as_str())
            .collect();

        // With gitignore disabled, debug.log should be present
        assert!(file_names.contains(&"app.rs"));
        assert!(file_names.contains(&"debug.log"));
        assert!(file_names.contains(&".gitignore"));
    }
}
