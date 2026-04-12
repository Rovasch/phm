use crate::discover;
use crate::version::PhpVersion;
use anyhow::{Result, bail};
use colored_text::Colorize;

pub fn run(version_str: &str, command: &[String]) -> Result<()> {
    let version = PhpVersion::parse(version_str)
        .ok_or_else(|| anyhow::anyhow!("invalid version: {}", version_str))?;

    let installations = discover::discover_versions()?;
    let installation = installations.iter().find(|i| i.version == version);

    let inst = match installation {
        Some(inst) => inst,
        None => {
            bail!(
                "PHP {} is not installed. Run: {}",
                version,
                format!("phm install {}", version).cyan()
            );
        }
    };

    if command.is_empty() {
        bail!("no command specified. Usage: phm exec 8.2 -- php -v");
    }

    // Build PATH with the target version's bin dir first
    let current_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", inst.bin_dir.display(), current_path);

    let status = std::process::Command::new(&command[0])
        .args(&command[1..])
        .env("PATH", new_path)
        .status()?;

    std::process::exit(status.code().unwrap_or(1));
}
