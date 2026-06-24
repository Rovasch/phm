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

/// Discover all installed PHP versions (platform-appropriate).
#[cfg(not(target_os = "macos"))]
pub fn discover_versions() -> Result<Vec<PhpInstallation>> {
    discover_linux_versions()
}

#[cfg(target_os = "macos")]
pub fn discover_versions() -> Result<Vec<PhpInstallation>> {
    discover_homebrew_versions()
}

/// Discover PHP versions on Linux: phm-managed installs + system PHP.
#[cfg(not(target_os = "macos"))]
fn discover_linux_versions() -> Result<Vec<PhpInstallation>> {
    let mut installations = Vec::new();
    let mut seen: HashSet<PhpVersion> = HashSet::new();

    // 1. phm-managed versions in ~/.local/share/phm/php-versions/{major.minor}/bin/php
    let managed = crate::config::managed_php_dir()?;
    if managed.exists() {
        let mut entries: Vec<_> = std::fs::read_dir(&managed)?.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let name = entry.file_name();
            if let Some(version) = PhpVersion::parse(&name.to_string_lossy()) {
                let bin_dir = entry.path().join("bin");
                if bin_dir.join("php").exists() && seen.insert(version) {
                    installations.push(PhpInstallation { version, bin_dir });
                }
            }
        }
    }

    // 2. System PHP — scan well-known paths for a binary named exactly "php"
    for dir in system_php_dirs() {
        let php = dir.join("php");
        if php.exists() {
            if let Some(version) = detect_php_version(&php) {
                if seen.insert(version) {
                    installations.push(PhpInstallation {
                        version,
                        bin_dir: dir,
                    });
                }
            }
        }
    }

    installations.sort_by(|a, b| a.version.cmp(&b.version));
    Ok(installations)
}

/// Return directories to scan for a system `php` binary.
/// On Termux, $PREFIX points to /data/data/com.termux/files/usr.
#[cfg(not(target_os = "macos"))]
fn system_php_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(prefix) = std::env::var_os("PREFIX") {
        let p = PathBuf::from(prefix).join("bin");
        if p.exists() {
            dirs.push(p);
        }
    }

    for d in ["/usr/bin", "/usr/local/bin"] {
        let p = PathBuf::from(d);
        if p.exists() && !dirs.contains(&p) {
            dirs.push(p);
        }
    }

    dirs
}

/// Run `php --version` on a binary and parse the version from its output.
#[cfg(not(target_os = "macos"))]
fn detect_php_version(binary: &Path) -> Option<PhpVersion> {
    let output = Command::new(binary).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    // "PHP 8.3.19 (cli) ..."
    let first = stdout.lines().next()?;
    let ver_str = first.strip_prefix("PHP ")?.split_whitespace().next()?;
    PhpVersion::parse(ver_str)
}

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
pub fn homebrew_opt_dirs() -> Vec<PathBuf> {
    homebrew_prefixes()
        .into_iter()
        .map(|prefix| prefix.join("opt"))
        .collect()
}

#[cfg(target_os = "macos")]
fn discover_homebrew_versions() -> Result<Vec<PhpInstallation>> {
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

#[cfg(target_os = "macos")]
fn detect_bare_php_version(php_opt_path: &Path) -> Option<PhpVersion> {
    // /opt/homebrew/opt/php -> ../Cellar/php/8.5.4
    let resolved = std::fs::read_link(php_opt_path).ok()?;
    let resolved_str = resolved.to_string_lossy();

    // Extract version from path like "../Cellar/php/8.5.4"
    let last = resolved_str.rsplit('/').next()?;
    PhpVersion::parse(last)
}

#[cfg(target_os = "macos")]
fn is_bare_php(bin_dir: &Path) -> bool {
    // /opt/homebrew/opt/php/bin -> parent is /opt/homebrew/opt/php -> file_name is "php"
    bin_dir
        .parent()
        .and_then(|p| p.file_name())
        .is_some_and(|name| name == "php")
}

#[cfg(target_os = "macos")]
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
