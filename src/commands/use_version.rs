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
    let (target, source_label) = if let Some(ref ver_str) = version {
        // Explicit version passed
        let ver = PhpVersion::parse(ver_str)
            .ok_or_else(|| anyhow::anyhow!("invalid version: {}", ver_str))?;
        (ver, String::new())
    } else {
        // Auto-detect from .php-version or composer.json
        let cwd = std::env::current_dir()?;
        match composer::find_version_file(&cwd)? {
            Some(result) => {
                (result.version, String::new())
            }
            None => {
                // Fall back to default
                match config::get_default()? {
                    Some(ver_str) => {
                        let ver = PhpVersion::parse(&ver_str)
                            .ok_or_else(|| anyhow::anyhow!("invalid default version: {}", ver_str))?;
                        (ver, " (default)".to_string())
                    }
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
    if let Some(ref cur) = current {
        if *cur == target_str {
            return Ok(());
        }
    }

    // Find the installation
    let installations = discover::discover_versions()?;
    let installation = installations.iter().find(|i| i.version == target);

    match installation {
        Some(inst) => {
            multishell::link_version(&ms_path, inst)?;
            println!(
                "Using {}{}",
                format!("PHP {}", target).green().bold(),
                source_label.dimmed()
            );
        }
        None => {
            let resolved = PhpVersion::resolve(&target_str, &installations.iter().map(|i| i.version.clone()).collect::<Vec<_>>());
            if let Some(resolved_ver) = resolved {
                if let Some(inst) = installations.iter().find(|i| i.version == resolved_ver) {
                    multishell::link_version(&ms_path, inst)?;
                    println!(
                        "Using {}{}",
                        format!("PHP {}", resolved_ver).green().bold(),
                        source_label.dimmed()
                    );
                    return Ok(());
                }
            }

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
