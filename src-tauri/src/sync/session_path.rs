//! Session directory path calculator
//!
//! Computes the Claude Code session directory from an Agent Block's `config_dir`.
//!
//! ## Path Encoding Rules (verified on actual machines)
//!
//! - Windows: `D:\workspace\zhidaoyuan\elfiee` → `D--workspace-zhidaoyuan-elfiee`
//!   - `\` replaced with `-`
//!   - `:` replaced with `-` (combined with `\`→`-` produces `D:` → `D-` + `-workspace` = `D--workspace`)
//!   - Drive letter may be lowercase
//! - Unix: `/home/yaosh/projects/elfiee` → `-home-yaosh-projects-elfiee`
//!   - `/` replaced with `-`
//!   - Leading `/` produces leading `-`

use std::path::{Path, PathBuf};

/// Compute session directory from an Agent's config_dir.
///
/// # Flow
/// 1. Extract parent from config_dir to get the project path
///    e.g. `D:\workspace\zhidaoyuan\elfiee\.claude` → `D:\workspace\zhidaoyuan\elfiee`
/// 2. Encode the project path using Claude Code's encoding rules
/// 3. Combine as `~/.claude/projects/{encoded}/`
///
/// # Returns
/// - `Ok(PathBuf)` — session directory path
/// - `Err(String)` — config_dir has no parent or home dir unavailable
pub fn compute_session_dir_from_config(config_dir: &str) -> Result<PathBuf, String> {
    let config_path = Path::new(config_dir);
    let project_path = config_path
        .parent()
        .ok_or_else(|| format!("config_dir has no parent: {}", config_dir))?;

    compute_session_dir(project_path)
}

/// Compute session directory from a project path.
///
/// Encodes the path and combines with `~/.claude/projects/`.
pub fn compute_session_dir(project_path: &Path) -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let encoded = encode_project_path(project_path);

    Ok(home.join(".claude").join("projects").join(encoded))
}

/// Encode a project path using Claude Code's encoding rules.
///
/// Verified results:
/// - `D:\workspace\zhidaoyuan\elfiee` → `D--workspace-zhidaoyuan-elfiee`
/// - `/home/yaosh/projects/elfiee` → `-home-yaosh-projects-elfiee`
///
/// Rules (order matters):
/// 1. Replace `\` with `-`
/// 2. Replace `/` with `-`
/// 3. Replace `:` with `-` (NOT removed — produces `D:` → `D-`, combined with `\` → `-` gives `D--`)
pub fn encode_project_path(path: &Path) -> String {
    let path_str = path.to_string_lossy().to_string();

    path_str
        .replace('\\', "-")
        .replace('/', "-")
        .replace(':', "-")
}

/// Extract project name from config_dir.
///
/// Takes the parent of config_dir, then returns the last component.
/// e.g. `D:\workspace\zhidaoyuan\elfiee\.claude` → `elfiee`
pub fn extract_project_name_from_config(config_dir: &str) -> String {
    let config_path = Path::new(config_dir);
    if let Some(project_path) = config_path.parent() {
        project_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    } else {
        "unknown".to_string()
    }
}

/// Extract project name from an encoded path string.
///
/// Takes the last `-`-separated segment.
/// e.g. `d--workspace-zhidaoyuan-elfiee` → `elfiee`
pub fn extract_project_name(encoded_path: &str) -> String {
    encoded_path
        .rsplit('-')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

/// Find session directory, trying case variants on Windows.
///
/// Claude Code may use either uppercase or lowercase drive letter.
/// Returns the first existing directory path, or None.
pub fn find_session_dir(project_path: &Path) -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let encoded = encode_project_path(project_path);
    let projects_dir = home.join(".claude").join("projects");

    // Try exact encoding first
    let exact = projects_dir.join(&encoded);
    if exact.is_dir() {
        return Some(exact);
    }

    // On Windows, try toggling the drive letter case
    #[cfg(windows)]
    {
        if let Some(first_char) = encoded.chars().next() {
            if first_char.is_ascii_alphabetic() {
                let toggled = if first_char.is_ascii_uppercase() {
                    let mut s = encoded.clone();
                    s.replace_range(0..1, &first_char.to_ascii_lowercase().to_string());
                    s
                } else {
                    let mut s = encoded.clone();
                    s.replace_range(0..1, &first_char.to_ascii_uppercase().to_string());
                    s
                };

                let alt = projects_dir.join(&toggled);
                if alt.is_dir() {
                    return Some(alt);
                }
            }
        }
    }

    None
}

/// Find session directory from config_dir with case-insensitive fallback.
///
/// Convenience wrapper combining `compute_session_dir_from_config` with
/// `find_session_dir` for case-insensitive lookup.
pub fn find_session_dir_from_config(config_dir: &str) -> Option<PathBuf> {
    let config_path = Path::new(config_dir);
    let project_path = config_path.parent()?;
    find_session_dir(project_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unix_path_encoding() {
        let path = Path::new("/home/yaosh/projects/elfiee");
        assert_eq!(encode_project_path(path), "-home-yaosh-projects-elfiee");
    }

    #[test]
    fn test_windows_path_encoding() {
        let path = Path::new("D:\\workspace\\zhidaoyuan\\elfiee");
        assert_eq!(encode_project_path(path), "D--workspace-zhidaoyuan-elfiee");
    }

    #[test]
    fn test_windows_colon_replaced_with_dash() {
        // `:` is replaced with `-`, combined with `\` also becoming `-`:
        // D:\workspace → D + - (from :) + - (from \) + workspace = D--workspace
        let path = Path::new("D:\\workspace");
        assert_eq!(encode_project_path(path), "D--workspace");
        // Should NOT produce triple --- (would happen if `:` produced two dashes)
        let full = encode_project_path(Path::new("D:\\workspace\\elfiee"));
        assert!(!full.contains("---"));
    }

    #[test]
    fn test_extract_project_name() {
        assert_eq!(
            extract_project_name("d--workspace-zhidaoyuan-elfiee"),
            "elfiee"
        );
    }

    #[test]
    fn test_extract_project_name_unix() {
        assert_eq!(
            extract_project_name("-home-yaosh-projects-elfiee"),
            "elfiee"
        );
    }

    #[test]
    fn test_extract_project_name_from_config_windows() {
        assert_eq!(
            extract_project_name_from_config("D:\\workspace\\zhidaoyuan\\elfiee\\.claude"),
            "elfiee"
        );
    }

    #[test]
    fn test_extract_project_name_from_config_unix() {
        assert_eq!(
            extract_project_name_from_config("/home/yaosh/projects/elfiee/.claude"),
            "elfiee"
        );
    }

    #[test]
    fn test_config_dir_to_session_dir() {
        let result = compute_session_dir_from_config("D:\\workspace\\zhidaoyuan\\elfiee\\.claude");
        assert!(result.is_ok());
        let path = result.unwrap();
        let path_str = path.to_string_lossy().to_string();
        assert!(path_str.contains(".claude"));
        assert!(path_str.contains("projects"));
        assert!(path_str.contains("D--workspace-zhidaoyuan-elfiee"));
    }
}
