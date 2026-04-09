use std::path::{Path, PathBuf};
use anyhow::Result;
use crate::version::PhpVersion;

#[derive(Debug, Clone)]
pub struct PhpInstallation {
    pub version: PhpVersion,
    pub bin_dir: PathBuf,
    pub full_version: Option<String>,
}

impl PhpInstallation {
    pub fn php_binary(&self) -> PathBuf {
        self.bin_dir.join("php")
    }
}

/// Discover all installed PHP versions from Homebrew.
pub fn discover_versions() -> Result<Vec<PhpInstallation>> {
    let homebrew_opt = Path::new("/opt/homebrew/opt");
    let mut installations = Vec::new();

    if !homebrew_opt.exists() {
        return Ok(installations);
    }

    let entries = std::fs::read_dir(homebrew_opt)?;

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Match "php@X.Y" directories
        if let Some(version_str) = name_str.strip_prefix("php@") {
            if let Some(version) = PhpVersion::parse(version_str) {
                let bin_dir = entry.path().join("bin");
                if bin_dir.join("php").exists() {
                    installations.push(PhpInstallation {
                        version,
                        bin_dir,
                        full_version: None,
                    });
                }
            }
        }
        // Match bare "php" directory (latest version)
        else if name_str == "php" {
            let bin_dir = entry.path().join("bin");
            if bin_dir.join("php").exists() {
                // Get the actual version by checking if it's a symlink to a Cellar path
                if let Some(version) = detect_bare_php_version(&entry.path()) {
                    // Only add if we don't already have this version via php@X.Y
                    installations.push(PhpInstallation {
                        version,
                        bin_dir,
                        full_version: None,
                    });
                }
            }
        }
    }

    // Deduplicate: if both php@X.Y and bare php resolve to the same version, keep php@X.Y
    deduplicate(&mut installations);

    installations.sort_by(|a, b| a.version.cmp(&b.version));
    Ok(installations)
}

/// Detect the version of the bare "php" formula by reading the Cellar symlink.
fn detect_bare_php_version(php_opt_path: &Path) -> Option<PhpVersion> {
    // /opt/homebrew/opt/php -> ../Cellar/php/8.5.4
    let resolved = std::fs::read_link(php_opt_path).ok()?;
    let resolved_str = resolved.to_string_lossy();

    // Extract version from path like "../Cellar/php/8.5.4"
    let last = resolved_str.rsplit('/').next()?;
    PhpVersion::parse(last)
}

/// Remove duplicates, preferring versioned formula (php@X.Y) over bare (php).
fn deduplicate(installations: &mut Vec<PhpInstallation>) {
    let versioned: std::collections::HashSet<PhpVersion> = installations
        .iter()
        .filter(|i| !i.bin_dir.to_string_lossy().contains("/opt/php/"))
        .map(|i| i.version.clone())
        .collect();

    installations.retain(|i| {
        let is_bare = i.bin_dir.to_string_lossy().contains("/opt/php/");
        if is_bare {
            !versioned.contains(&i.version)
        } else {
            true
        }
    });
}

/// Find a specific installed version.
pub fn find_version(target: &PhpVersion) -> Result<Option<PhpInstallation>> {
    let installations = discover_versions()?;
    Ok(installations.into_iter().find(|i| i.version == *target))
}

/// Get the full version string (e.g., "8.2.30") by running `php -v`.
pub fn get_full_version(installation: &PhpInstallation) -> Option<String> {
    let php_bin = installation.bin_dir.join("php");
    let output = std::process::Command::new(php_bin)
        .arg("-v")
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Parse "PHP 8.2.30 (cli) ..." -> "8.2.30"
    stdout
        .split_whitespace()
        .nth(1)
        .map(|s| s.to_string())
}
