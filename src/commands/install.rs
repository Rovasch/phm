use crate::version::PhpVersion;
use anyhow::{Context, Result};
use colored_text::Colorize;

pub fn run(version_str: &str) -> Result<()> {
    let version = PhpVersion::parse(version_str)
        .ok_or_else(|| anyhow::anyhow!("invalid version: {}", version_str))?;

    // Check if already installed
    let installations = crate::discover::discover_versions()?;
    if installations.iter().any(|i| i.version == version) {
        println!(
            "PHP {} is already installed",
            version.to_string().hex("#777BB3").bold()
        );
        return Ok(());
    }

    ensure_brew_available()?;

    // Determine the brew formula
    let (needs_tap, formula) = if version.major <= 7 {
        // Old versions need shivammathur tap
        (true, format!("shivammathur/php/php@{}", version))
    } else {
        (false, format!("php@{}", version))
    };

    // Tap if needed
    if needs_tap {
        println!("{} Tapping {}", "[1/3]".dim(), "shivammathur/php".cyan());
        let status = std::process::Command::new("brew")
            .args(["tap", "shivammathur/php"])
            .status()
            .context("failed to run brew tap")?;
        if !status.success() {
            anyhow::bail!("brew tap shivammathur/php failed");
        }
    }

    // Install
    let install_step = if needs_tap { "[2/3]" } else { "[1/2]" };
    println!("{} Installing {}", install_step.dim(), formula.cyan());
    let status = std::process::Command::new("brew")
        .args(["install", &formula])
        .status()
        .context("failed to run brew install")?;

    if !status.success() {
        anyhow::bail!("brew install {} failed", formula);
    }

    let verify_step = if needs_tap { "[3/3]" } else { "[2/2]" };
    println!(
        "{} {}",
        verify_step.dim(),
        format!("Verifying PHP {}", version).cyan()
    );
    let installations = crate::discover::discover_versions()?;
    if installations.iter().any(|i| i.version == version) {
        println!(
            "{} PHP {} installed",
            "done:".hex("#777BB3").bold(),
            version
        );
        return Ok(());
    }

    anyhow::bail!(
        "brew install completed but PHP {} was not discovered afterwards. Check `brew --prefix {}` and `phm doctor`",
        version,
        formula
    );
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
