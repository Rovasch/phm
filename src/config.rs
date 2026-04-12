use anyhow::{Context, Result};
use std::path::PathBuf;

/// Get the PHM config directory (~/.phm/).
pub fn config_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".phm"))
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
