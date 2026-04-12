use crate::version::PhpVersion;
use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct PhpInstallation {
    pub version: PhpVersion,
    pub bin_dir: PathBuf,
}

pub fn homebrew_prefixes() -> Vec<PathBuf> {
    let mut prefixes = Vec::new();
    let mut seen = HashSet::new();

    if let Some(prefix) = std::env::var_os("HOMEBREW_PREFIX") {
        let path = PathBuf::from(prefix);
        if seen.insert(path.clone()) {
            prefixes.push(path);
        }
    }

    if let Ok(output) = Command::new("brew").arg("--prefix").output()
        && output.status.success()
    {
        let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !prefix.is_empty() {
            let path = PathBuf::from(prefix);
            if seen.insert(path.clone()) {
                prefixes.push(path);
            }
        }
    }

    for prefix in ["/opt/homebrew", "/usr/local"] {
        let path = PathBuf::from(prefix);
        if seen.insert(path.clone()) {
            prefixes.push(path);
        }
    }

    prefixes
}

pub fn homebrew_opt_dirs() -> Vec<PathBuf> {
    homebrew_prefixes()
        .into_iter()
        .map(|prefix| prefix.join("opt"))
        .collect()
}

/// Discover all installed PHP versions from Homebrew.
pub fn discover_versions() -> Result<Vec<PhpInstallation>> {
    let mut installations = Vec::new();

    for homebrew_opt in homebrew_opt_dirs() {
        if !homebrew_opt.exists() {
            continue;
        }

        let entries = std::fs::read_dir(&homebrew_opt)?;

        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Match "php@X.Y" directories
            if let Some(version_str) = name_str.strip_prefix("php@") {
                if let Some(version) = PhpVersion::parse(version_str) {
                    let bin_dir = entry.path().join("bin");
                    if bin_dir.join("php").exists() {
                        installations.push(PhpInstallation { version, bin_dir });
                    }
                }
            }
            // Match bare "php" directory (latest version)
            else if name_str == "php" {
                let bin_dir = entry.path().join("bin");
                if bin_dir.join("php").exists()
                    && let Some(version) = detect_bare_php_version(&entry.path())
                {
                    installations.push(PhpInstallation { version, bin_dir });
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

/// Check if a path is the bare "php" formula (not "php@X.Y").
fn is_bare_php(bin_dir: &Path) -> bool {
    // /opt/homebrew/opt/php/bin -> parent is /opt/homebrew/opt/php -> file_name is "php"
    bin_dir
        .parent()
        .and_then(|p| p.file_name())
        .is_some_and(|name| name == "php")
}

/// Remove duplicates, preferring versioned formula (php@X.Y) over bare (php).
fn deduplicate(installations: &mut Vec<PhpInstallation>) {
    let versioned: HashSet<PhpVersion> = installations
        .iter()
        .filter(|i| !is_bare_php(&i.bin_dir))
        .map(|i| i.version)
        .collect();

    installations.retain(|i| {
        if is_bare_php(&i.bin_dir) {
            !versioned.contains(&i.version)
        } else {
            true
        }
    });
}
