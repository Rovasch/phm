use anyhow::Result;
use colored::Colorize;
use crate::config;
use crate::discover;
use crate::multishell;

pub fn run() -> Result<()> {
    let installations = discover::discover_versions()?;

    if installations.is_empty() {
        eprintln!("No PHP versions found. Install one with: brew install php@8.2");
        return Ok(());
    }

    let default_ver = config::get_default()?;
    let current_ver = std::env::var("PHM_MULTISHELL_PATH")
        .ok()
        .and_then(|p| multishell::read_current(&std::path::PathBuf::from(p)));

    for inst in &installations {
        let ver_str = inst.version.to_string();
        let is_current = current_ver.as_deref() == Some(&ver_str);
        let is_default = default_ver.as_deref() == Some(&ver_str);

        let marker = if is_current { "*" } else { " " };
        let ver_display = if is_current {
            ver_str.green().bold().to_string()
        } else {
            ver_str.clone()
        };

        let mut tags = Vec::new();
        if is_current {
            tags.push("current".to_string());
        }
        if is_default {
            tags.push("default".to_string());
        }

        let tag_str = if tags.is_empty() {
            String::new()
        } else {
            format!(" ({})", tags.join(", "))
        };

        println!("{} {}{}", marker, ver_display, tag_str.dimmed());
    }

    Ok(())
}
