use crate::discover::PhpInstallation;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Base directory for multishell state.
pub fn multishell_base() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".local/state/phm/multishells"))
}

/// Create a new multishell directory for the current shell session.
/// Returns the path to the multishell directory.
pub fn create_multishell(pid: u32) -> Result<PathBuf> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let id = format!("{}_{}", pid, ts);
    let dir = multishell_base()?.join(&id);
    let bin_dir = dir.join("bin");

    std::fs::create_dir_all(&bin_dir)
        .with_context(|| format!("failed to create multishell dir: {}", bin_dir.display()))?;

    Ok(dir)
}

/// Populate the multishell bin directory with symlinks to the given PHP installation.
pub fn link_version(multishell_path: &Path, installation: &PhpInstallation) -> Result<()> {
    let bin_dir = multishell_path.join("bin");

    // Remove existing symlinks
    if bin_dir.exists() {
        for entry in std::fs::read_dir(&bin_dir)? {
            let entry = entry?;
            let _ = std::fs::remove_file(entry.path());
        }
    } else {
        std::fs::create_dir_all(&bin_dir)?;
    }

    // Create symlinks for all binaries in the PHP installation's bin dir
    let binaries = [
        "php",
        "php-cgi",
        "php-config",
        "phpize",
        "phpdbg",
        "phar",
        "pecl",
        "pear",
    ];

    for binary in &binaries {
        let source = installation.bin_dir.join(binary);
        let target = bin_dir.join(binary);
        if source.exists() {
            std::os::unix::fs::symlink(&source, &target).with_context(|| {
                format!(
                    "failed to symlink {} -> {}",
                    target.display(),
                    source.display()
                )
            })?;
        }
    }

    // Also handle phar.phar -> phar symlink if it exists
    let phar_phar = installation.bin_dir.join("phar.phar");
    if phar_phar.exists() {
        let target = bin_dir.join("phar.phar");
        std::os::unix::fs::symlink(&phar_phar, &target)?;
    }

    // Write current version
    std::fs::write(
        multishell_path.join("current"),
        format!("{}\n", installation.version),
    )?;

    Ok(())
}

/// Read the current version from a multishell directory.
pub fn read_current(multishell_path: &Path) -> Option<String> {
    let current_file = multishell_path.join("current");
    std::fs::read_to_string(current_file)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn is_process_alive(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

/// Clean up stale multishell directories from dead PIDs.
pub fn cleanup_stale() {
    let base = match multishell_base() {
        Ok(base) => base,
        Err(_) => return,
    };
    if !base.exists() {
        return;
    }

    let entries = match std::fs::read_dir(&base) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if let Some(pid_str) = name_str.split('_').next()
            && let Ok(pid) = pid_str.parse::<i32>()
            && !is_process_alive(pid)
        {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}
