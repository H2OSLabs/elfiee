use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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
    /// List of directory names to ignore
    pub ignore_patterns: Vec<String>,
    /// Maximum file size in bytes to include
    pub max_file_size: u64,
    /// Maximum number of files to scan
    pub max_files: usize,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            max_depth: 100,
            follow_symlinks: false,
            ignore_hidden: true,
            ignore_patterns: vec![
                "node_modules".to_string(),
                ".git".to_string(),
                "target".to_string(),
                "dist".to_string(),
                "build".to_string(),
                ".DS_Store".to_string(),
            ],
            max_file_size: 10 * 1024 * 1024, // 10 MB
            max_files: 10_000,
        }
    }
}

/// Scan a directory and return a list of files.
///
/// Uses `filter_entry()` to prevent descent into hidden and ignored directories.
/// Without `filter_entry()`, `continue` only skips the entry from results but
/// WalkDir still descends into hidden/ignored directories (bug fixed here).
pub fn scan_directory(root: &Path, options: &ScanOptions) -> Result<Vec<FileInfo>, String> {
    let mut files = Vec::new();
    let mut count = 0;

    let ignore_hidden = options.ignore_hidden;
    let ignore_patterns = options.ignore_patterns.clone();

    let walker = WalkDir::new(root)
        .max_depth(options.max_depth)
        .follow_links(options.follow_symlinks);

    // filter_entry prevents descent into filtered directories.
    // Depth 0 is the root entry itself — always allow it through since the
    // caller explicitly chose to scan that path (it may be a hidden dir).
    for entry in walker.into_iter().filter_entry(move |e| {
        if e.depth() == 0 {
            return true;
        }

        let name = e.file_name().to_string_lossy();

        // Prevent descent into hidden directories
        if ignore_hidden && name.starts_with('.') {
            return false;
        }

        // Prevent descent into ignored directories
        if ignore_patterns.iter().any(|p| name == *p) {
            return false;
        }

        true
    }) {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        // Check file count limit
        count += 1;
        if count > options.max_files {
            return Err(format!("Too many files (limit: {})", options.max_files));
        }

        // Skip symlinks (they pass filter_entry but shouldn't be included in results)
        if entry.path_is_symlink() {
            continue;
        }

        let metadata = entry
            .metadata()
            .map_err(|e| format!("Failed to read metadata: {}", e))?;

        // Skip large files
        if metadata.is_file() && metadata.len() > options.max_file_size {
            log::warn!("Skipping large file: {:?} ({} bytes)", path, metadata.len());
            continue;
        }

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
    fn test_scan_respects_ignore_patterns() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir(temp_dir.path().join("node_modules")).unwrap();
        fs::write(temp_dir.path().join("node_modules/package.json"), "{}").unwrap();
        fs::write(temp_dir.path().join("main.js"), "code").unwrap();

        let options = ScanOptions::default();
        let files = scan_directory(temp_dir.path(), &options).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_name, "main.js");
    }
}
