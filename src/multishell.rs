use crate::discover::PhpInstallation;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub const PHP_BINARIES: &[&str] = &[
    "php",
    "php-cgi",
    "php-config",
    "phpize",
    "phpdbg",
    "phar",
    "phar.phar",
    "pecl",
    "pear",
];

/// Base directory for multishell state.
pub fn multishell_base() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".local/state/phm/multishells"))
}

/// Stable alias directory — not PID-scoped, safe to add to PATH in ~/.zshenv.
pub fn default_alias_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".local/state/phm/aliases/default"))
}

/// Update the stable default alias to point to the given installation.
/// Called by `phm env`, `phm use`, and `phm default` so the alias stays current.
pub fn update_default_alias(installation: &PhpInstallation) -> Result<()> {
    let alias_path = default_alias_path()?;
    link_version(&alias_path, installation)
}

/// Create a new multishell directory for the current shell session.
/// Returns the path to the multishell directory.
pub fn create_multishell(pid: u32) -> Result<PathBuf> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
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

    #[cfg(not(target_os = "macos"))]
    let proot_args = crate::termux::proot_wrap_args();

    for binary in PHP_BINARIES {
        let source = installation.bin_dir.join(binary);
        let target = bin_dir.join(binary);
        if source.exists() {
            #[cfg(not(target_os = "macos"))]
            if let Some((ref proot, ref resolv)) = proot_args {
                install_proot_wrapper(&source, &target, proot, resolv).with_context(|| {
                    format!("failed to create wrapper for {}", target.display())
                })?;
                continue;
            }
            std::os::unix::fs::symlink(&source, &target).with_context(|| {
                format!(
                    "failed to symlink {} -> {}",
                    target.display(),
                    source.display()
                )
            })?;
        }
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

#[cfg(not(target_os = "macos"))]
fn install_proot_wrapper(
    source: &std::path::Path,
    target: &std::path::Path,
    proot: &std::path::Path,
    resolv: &std::path::Path,
) -> anyhow::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    let script = format!(
        "#!/bin/sh\nexec '{}' -b '{}:/etc/resolv.conf' '{}' \"$@\"\n",
        proot.display(),
        resolv.display(),
        source.display(),
    );
    let mut f = std::fs::File::create(target)?;
    f.write_all(script.as_bytes())?;
    std::fs::set_permissions(target, std::fs::Permissions::from_mode(0o755))?;
    Ok(())
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
