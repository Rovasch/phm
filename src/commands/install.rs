use anyhow::{Context, Result};
use colored_text::Colorize;
use crate::version::PhpVersion;

pub fn run(version_str: &str) -> Result<()> {
    let version = PhpVersion::parse(version_str)
        .ok_or_else(|| anyhow::anyhow!("invalid version: {}", version_str))?;

    // Check if already installed
    let installations = crate::discover::discover_versions()?;
    if installations.iter().any(|i| i.version == version) {
        println!("PHP {} is already installed", version.to_string().hex("#777BB3").bold());
        return Ok(());
    }

    // Determine the brew formula
    let (needs_tap, formula) = if version.major <= 7 {
        // Old versions need shivammathur tap
        (true, format!("shivammathur/php/php@{}", version))
    } else {
        (false, format!("php@{}", version))
    };

    // Tap if needed
    if needs_tap {
        println!("Tapping {}...", "shivammathur/php".cyan());
        let status = std::process::Command::new("brew")
            .args(["tap", "shivammathur/php"])
            .status()
            .context("failed to run brew tap")?;
        if !status.success() {
            anyhow::bail!("brew tap shivammathur/php failed");
        }
    }

    // Install
    println!("Installing {}...", format!("PHP {}", version).cyan());
    let status = std::process::Command::new("brew")
        .args(["install", &formula])
        .status()
        .context("failed to run brew install")?;

    if status.success() {
        println!("{} PHP {} installed", "done:".hex("#777BB3").bold(), version);
    } else {
        anyhow::bail!("brew install {} failed", formula);
    }

    Ok(())
}
