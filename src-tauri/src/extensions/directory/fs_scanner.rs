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
    /// Maximum file size in bytes to include
    pub max_file_size: u64,
    /// Maximum number of files to scan
    pub max_files: usize,
    /// Whether to respect .gitignore files when scanning
    pub use_gitignore: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            max_depth: 100,
            follow_symlinks: false,
            ignore_hidden: true,
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
