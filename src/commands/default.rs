use crate::config;
use crate::discover;
use crate::version::PhpVersion;
use anyhow::Result;
use colored_text::Colorize;

pub fn run(version: Option<String>) -> Result<()> {
    match version {
        Some(ver_str) => {
            let ver = PhpVersion::parse(&ver_str)
                .ok_or_else(|| anyhow::anyhow!("invalid version: {}", ver_str))?;

            // Check if it's installed
            let installations = discover::discover_versions()?;
            if !installations.iter().any(|i| i.version == ver) {
                eprintln!(
                    "{} PHP {} is not installed. Run: {}",
                    "error:".red().bold(),
                    ver,
                    format!("phm install {}", ver).cyan()
                );
                return Ok(());
            }

            config::set_default(&ver.to_string())?;
            println!(
                "Default PHP version set to {}",
                ver.to_string().hex("#777BB3").bold()
            );
        }
        None => match config::get_default()? {
            Some(ver) => println!("{}", ver),
            None => eprintln!("No default version set. Run: phm default <version>"),
        },
    }

    Ok(())
}
