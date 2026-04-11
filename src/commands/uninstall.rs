use anyhow::{Context, Result};
use colored_text::Colorize;
use crate::config;
use crate::discover;
use crate::version::PhpVersion;

pub fn run(version_str: &str) -> Result<()> {
    let version = PhpVersion::parse(version_str)
        .ok_or_else(|| anyhow::anyhow!("invalid version: {}", version_str))?;

    // Check if installed
    let installations = discover::discover_versions()?;
    if !installations.iter().any(|i| i.version == version) {
        eprintln!("PHP {} is not installed", version);
        return Ok(());
    }

    // Prevent uninstalling the default version
    if let Some(default) = config::get_default()? {
        if default == version.to_string() {
            eprintln!(
                "{} cannot uninstall PHP {} because it is the default version",
                "error:".red().bold(),
                version
            );
            eprintln!("Set a different default first: {}", "phm default <version>".cyan());
            return Ok(());
        }
    }

    let formula = format!("php@{}", version);

    println!("Uninstalling {}...", format!("PHP {}", version).cyan());
    let status = std::process::Command::new("brew")
        .args(["uninstall", &formula])
        .status()
        .context("failed to run brew uninstall")?;

    if status.success() {
        println!("{} PHP {} uninstalled", "done:".hex("#777BB3").bold(), version);
    } else {
        anyhow::bail!("brew uninstall {} failed", formula);
    }

    Ok(())
}
