/// Infer Block Type from file extension.
///
/// Strategy:
/// 1. Known markdown extensions -> "markdown"
/// 2. Known code/config extensions -> "code"
/// 3. Known binary extensions (images, executables, etc.) -> None (Skip)
/// 4. Unknown extensions -> Some("code") (Treat as plain text fallback)
pub fn infer_block_type(extension: &str) -> Option<String> {
    let ext = extension.to_lowercase();

    // 1. Explicit Binary Blacklist - Do NOT import these into DB as text
    match ext.as_str() {
        // Images
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" |
        "icns" | "bmp" | "tiff" | "tif" | "avif" | "heic" | "heif" |
        "cur" | "psd" | "ai" | "eps" | "raw" | "cr2" | "nef" | "dng" |
        // Video
        "mp4" | "mov" | "avi" | "mkv" | "wmv" | "flv" | "webm" | "m4v" |
        // Audio
        "mp3" | "wav" | "ogg" | "flac" | "aac" | "m4a" | "wma" | "opus" |
        "mid" | "midi" |
        // Archives
        "pdf" | "zip" | "tar" | "gz" | "7z" | "rar" | "bz2" |
        "xz" | "zst" | "tgz" | "cab" | "dmg" | "iso" | "img" |
        // Java/Android archives
        "jar" | "war" | "ear" | "apk" | "aab" |
        // Binary/Compiled
        "exe" | "dll" | "so" | "dylib" | "bin" | "obj" | "o" | "a" |
        "lib" | "pdb" | "msi" | "ipa" |
        // Bytecode
        "pyc" | "class" | "wasm" |
        // Databases
        "db" | "sqlite" | "sqlite3" |
        // Fonts
        "ttf" | "otf" | "woff" | "woff2" | "eot" |
        // Office documents (binary formats)
        "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" |
        "odt" | "ods" | "odp" |
        // Data formats (binary)
        "parquet" | "arrow" | "avro" | "pb" |
        // 3D/Design
        "blend" | "fbx" | "glb" | "swf" |
        // OS artifacts
        "ds_store" => return None,
        _ => {}
    }

    // 2. Specific Type Mapping
    match ext.as_str() {
        // Markdown
        "md" | "markdown" => Some("markdown".to_string()),

        // Code (Handled by 'code' block type)
        "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "c" | "cpp" | "h" | "hpp" | "java" | "go"
        | "rb" | "php" | "swift" | "kt" | "cs" | "scala" | "json" | "toml" | "yaml" | "yml"
        | "xml" | "ini" | "conf" | "sh" | "bash" | "zsh" | "fish" | "html" | "htm" | "css"
        | "scss" | "sass" | "less" | "sql" => Some("code".to_string()),

        // 3. Fallback: Treat everything else as plain text 'code' block
        // This ensures we don't miss .env, .gitignore, license files, etc.
        _ => {
            log::debug!("Unknown extension '{}', defaulting to code block type", ext);
            Some("code".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_extensions() {
        assert_eq!(infer_block_type("md"), Some("markdown".to_string()));
        assert_eq!(infer_block_type("MD"), Some("markdown".to_string()));
    }

    #[test]
    fn test_known_code_extensions() {
        assert_eq!(infer_block_type("rs"), Some("code".to_string()));
        assert_eq!(infer_block_type("json"), Some("code".to_string()));
    }

    #[test]
    fn test_binary_blacklist() {
        assert_eq!(infer_block_type("png"), None);
        assert_eq!(infer_block_type("exe"), None);
        assert_eq!(infer_block_type("wasm"), None);
        assert_eq!(infer_block_type("db"), None);
    }

    #[test]
    fn test_fallback_to_code() {
        // Unknown or missing extensions should be treated as text
        assert_eq!(infer_block_type("env"), Some("code".to_string()));
        assert_eq!(infer_block_type("gitignore"), Some("code".to_string()));
        assert_eq!(infer_block_type("LICENSE"), Some("code".to_string()));
        assert_eq!(infer_block_type(""), Some("code".to_string()));
    }
}
