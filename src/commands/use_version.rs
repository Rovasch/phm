use anyhow::{bail, Result};
use colored::Colorize;
use crate::composer;
use crate::config;
use crate::discover;
use crate::multishell;
use crate::version::PhpVersion;

pub fn run(version: Option<String>, silent_if_unchanged: bool) -> Result<()> {
    let ms_path = std::env::var("PHM_MULTISHELL_PATH")
        .map_err(|_| anyhow::anyhow!("PHM_MULTISHELL_PATH not set. Run: eval \"$(phm env)\""))?;
    let ms_path = std::path::PathBuf::from(ms_path);

    let current = multishell::read_current(&ms_path);

    // Determine target version
    let target = if let Some(ref ver_str) = version {
        PhpVersion::parse(ver_str)
            .ok_or_else(|| anyhow::anyhow!("invalid version: {}", ver_str))?
    } else {
        // Auto-detect from .php-version or composer.json
        let cwd = std::env::current_dir()?;
        match composer::find_version(&cwd)? {
            Some(ver) => ver,
            None => {
                // Fall back to default
                match config::get_default()? {
                    Some(ver_str) => PhpVersion::parse(&ver_str)
                        .ok_or_else(|| anyhow::anyhow!("invalid default version: {}", ver_str))?,
                    None => {
                        if silent_if_unchanged {
                            return Ok(());
                        }
                        bail!("no PHP version specified and no default set. Run: phm default <version>");
                    }
                }
            }
        }
    };

    let target_str = target.to_string();

    // Fast path: version unchanged
    if current.as_deref() == Some(&target_str) {
        return Ok(());
    }

    // Find the installation
    let installations = discover::discover_versions()?;
    let versions: Vec<PhpVersion> = installations.iter().map(|i| i.version).collect();

    // Try exact match first, then resolve via constraint
    let resolved = if installations.iter().any(|i| i.version == target) {
        Some(target)
    } else {
        PhpVersion::resolve(&target_str, &versions)
    };

    match resolved.and_then(|v| installations.iter().find(|i| i.version == v)) {
        Some(inst) => {
            multishell::link_version(&ms_path, inst)?;
            println!("Using {}", format!("PHP {}", inst.version).green().bold());
        }
        None => {
            if !silent_if_unchanged {
                eprintln!(
                    "{} PHP {} is not installed. Run: {}",
                    "error:".red().bold(),
                    target,
                    format!("phm install {}", target).cyan()
                );
            }
        }
    }

    Ok(())
}
