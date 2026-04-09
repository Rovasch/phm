use anyhow::Result;
use colored::Colorize;
use crate::config;
use crate::discover;
use crate::multishell;

pub fn run() -> Result<()> {
    let mut issues = 0;

    // Check: PHP versions found
    let installations = discover::discover_versions()?;
    if installations.is_empty() {
        println!("{} No PHP versions found in Homebrew", "✗".red());
        println!("  Install one with: brew install php@8.2");
        issues += 1;
    } else {
        println!(
            "{} {} PHP version(s) found: {}",
            "✓".green(),
            installations.len(),
            installations
                .iter()
                .map(|i| i.version.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    // Check: default version set
    match config::get_default()? {
        Some(ver) => {
            if installations.iter().any(|i| i.version.to_string() == ver) {
                println!("{} Default version: {}", "✓".green(), ver);
            } else {
                println!(
                    "{} Default version {} is not installed",
                    "✗".red(),
                    ver
                );
                issues += 1;
            }
        }
        None => {
            println!("{} No default version set", "✗".red());
            println!("  Set one with: phm default <version>");
            issues += 1;
        }
    }

    // Check: PHM_MULTISHELL_PATH set
    match std::env::var("PHM_MULTISHELL_PATH") {
        Ok(path) => {
            if std::path::Path::new(&path).exists() {
                println!("{} Shell integration active", "✓".green());
            } else {
                println!("{} PHM_MULTISHELL_PATH set but directory missing", "✗".red());
                issues += 1;
            }
        }
        Err(_) => {
            println!("{} Shell integration not loaded", "✗".red());
            println!("  Add to .zshrc: eval \"$(phm env --shell=zsh --use-on-cd)\"");
            issues += 1;
        }
    }

    // Check: Herd not conflicting
    let path = std::env::var("PATH").unwrap_or_default();
    if path.contains("Herd/bin") {
        println!("{} Herd is still in PATH — may conflict with phm", "✗".red());
        println!("  Remove from .zshrc: export PATH=\".../Herd/bin/:$PATH\"");
        issues += 1;
    } else {
        println!("{} No Herd conflict", "✓".green());
    }

    // Check: composer available
    let composer_check = std::process::Command::new("which")
        .arg("composer")
        .output();
    match composer_check {
        Ok(output) if output.status.success() => {
            println!("{} Composer found", "✓".green());
        }
        _ => {
            println!("{} Composer not found", "✗".red());
            println!("  Install with: brew install composer");
            issues += 1;
        }
    }

    // Check: stale multishell dirs
    let base = dirs::home_dir()
        .expect("could not determine home directory")
        .join(".local/state/phm/multishells");
    if base.exists() {
        let mut stale = 0;
        if let Ok(entries) = std::fs::read_dir(&base) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if let Some(pid_str) = name_str.split('_').next() {
                    if let Ok(pid) = pid_str.parse::<i32>() {
                        if !multishell::is_process_alive(pid) {
                            stale += 1;
                        }
                    }
                }
            }
        }
        if stale > 0 {
            println!(
                "{} {} stale multishell dir(s) (cleaned up on next shell init)",
                "!".yellow(),
                stale
            );
        } else {
            println!("{} No stale multishell directories", "✓".green());
        }
    }

    println!();
    if issues == 0 {
        println!("{}", "All checks passed!".green().bold());
    } else {
        println!(
            "{} issue(s) found",
            format!("{}", issues).red().bold()
        );
    }

    Ok(())
}
