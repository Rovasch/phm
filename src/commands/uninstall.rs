use crate::config;
use crate::discover;
use crate::version::PhpVersion;
use anyhow::{Context, Result};
use colored_text::Colorize;

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
    if let Some(default) = config::get_default()?
        && default == version.to_string()
    {
        eprintln!(
            "{} cannot uninstall PHP {} because it is the default version",
            "error:".red().bold(),
            version
        );
        eprintln!(
            "Set a different default first: {}",
            "phm default <version>".cyan()
        );
        return Ok(());
    }

    ensure_brew_available()?;

    let formula = if version.major <= 7 {
        format!("shivammathur/php/php@{}", version)
    } else {
        format!("php@{}", version)
    };

    println!("{} Uninstalling {}", "[1/2]".dim(), formula.cyan());
    let status = std::process::Command::new("brew")
        .args(["uninstall", &formula])
        .status()
        .context("failed to run brew uninstall")?;

    if !status.success() {
        anyhow::bail!("brew uninstall {} failed", formula);
    }

    println!(
        "{} {}",
        "[2/2]".dim(),
        format!("Verifying PHP {}", version).cyan()
    );
    let installations = discover::discover_versions()?;
    if installations.iter().any(|i| i.version == version) {
        anyhow::bail!(
            "brew uninstall completed but PHP {} is still discoverable. Check `brew list --versions {}`",
            version,
            formula
        );
    }

    println!(
        "{} PHP {} uninstalled",
        "done:".hex("#777BB3").bold(),
        version
    );
    Ok(())
}

fn ensure_brew_available() -> Result<()> {
    let status = std::process::Command::new("brew")
        .arg("--version")
        .status()
        .context("failed to run brew --version")?;
    if status.success() {
        return Ok(());
    }

    anyhow::bail!("Homebrew is not available in PATH");
}
