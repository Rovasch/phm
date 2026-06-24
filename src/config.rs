use anyhow::{Context, Result};
use std::path::PathBuf;

/// Get the PHM config directory (~/.phm/).
pub fn config_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".phm"))
}

/// Directory where phm stores downloaded PHP versions on Linux.
/// Follows XDG Base Directory: $XDG_DATA_HOME/phm/php-versions (~/.local/share/phm/php-versions).
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn managed_php_dir() -> Result<PathBuf> {
    let base = dirs::data_local_dir().context("could not determine XDG data dir")?;
    Ok(base.join("phm/php-versions"))
}

/// Ensure the config directory exists.
pub fn ensure_config_dir() -> Result<PathBuf> {
    let dir = config_dir()?;
    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create config dir: {}", dir.display()))?;
    }
    Ok(dir)
}

/// Get the default PHP version.
pub fn get_default() -> Result<Option<String>> {
    let path = config_dir()?.join("default");
    if path.exists() {
        let content =
            std::fs::read_to_string(&path).with_context(|| "failed to read default version")?;
        let trimmed = content.trim().to_string();
        if trimmed.is_empty() {
            Ok(None)
        } else {
            Ok(Some(trimmed))
        }
    } else {
        Ok(None)
    }
}

/// Set the default PHP version.
pub fn set_default(version: &str) -> Result<()> {
    let dir = ensure_config_dir()?;
    std::fs::write(dir.join("default"), format!("{}\n", version))
        .with_context(|| "failed to write default version")?;
    Ok(())
}
