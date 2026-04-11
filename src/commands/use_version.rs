use anyhow::{bail, Result};
use colored::Colorize;
use crate::composer;
use crate::config;
use crate::discover;
use crate::multishell;
use crate::version::{PhpVersion, VersionConstraint};

pub fn run(version: Option<String>, silent_if_unchanged: bool) -> Result<()> {
    let ms_path = std::env::var("PHM_MULTISHELL_PATH")
        .map_err(|_| anyhow::anyhow!("PHM_MULTISHELL_PATH not set. Run: eval \"$(phm env)\""))?;
    let ms_path = std::path::PathBuf::from(ms_path);

    let current = multishell::read_current(&ms_path);

    // Determine constraint
    let constraint = if let Some(ref ver_str) = version {
        let v = PhpVersion::parse(ver_str)
            .ok_or_else(|| anyhow::anyhow!("invalid version: {}", ver_str))?;
        VersionConstraint::exact(v)
    } else {
        // Auto-detect from .php-version or composer.json
        let cwd = std::env::current_dir()?;
        match composer::find_version(&cwd)? {
            Some(c) => c,
            None => {
                // Fall back to default
                match config::get_default()? {
                    Some(ver_str) => {
                        let v = PhpVersion::parse(&ver_str)
                            .ok_or_else(|| anyhow::anyhow!("invalid default version: {}", ver_str))?;
                        VersionConstraint::exact(v)
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

    // Fast path: current version already satisfies constraint
    if let Some(ref current_str) = current {
        if let Some(current_ver) = PhpVersion::parse(current_str) {
            if constraint.satisfies(current_ver) {
                return Ok(());
            }
        }
    }

    // Find the best installed version satisfying the constraint
    let installations = discover::discover_versions()?;
    let versions: Vec<PhpVersion> = installations.iter().map(|i| i.version).collect();

    let resolved = constraint.resolve(&versions);

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
                    constraint.target(),
                    format!("phm install {}", constraint.target()).cyan()
                );
            }
        }
    }

    Ok(())
}
