use crate::config;
use crate::discover;
use crate::multishell;
use anyhow::Result;
use colored_text::Colorize;

pub fn run() -> Result<()> {
    let mut issues = 0;
    let opt_dirs = discover::homebrew_opt_dirs();
    let detected_dirs = opt_dirs
        .iter()
        .filter(|path| path.exists())
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();

    // Check: PHP versions found
    let installations = discover::discover_versions()?;
    if installations.is_empty() {
        println!("{} No PHP versions found in Homebrew", "✗".red());
        println!("  Install one with: brew install php@8.2");
        issues += 1;
    } else {
        println!(
            "{} {} PHP version(s) found: {}",
            "✓".hex("#777BB3"),
            installations.len(),
            installations
                .iter()
                .map(|i| i.version.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    if detected_dirs.is_empty() {
        println!("{} No Homebrew opt directory detected", "!".yellow());
        println!(
            "  Checked: {}",
            opt_dirs
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    } else {
        println!(
            "{} Homebrew opt dirs: {}",
            "✓".hex("#777BB3"),
            detected_dirs.join(", ")
        );
    }

    // Check: default version set
    match config::get_default()? {
        Some(ver) => {
            if installations.iter().any(|i| i.version.to_string() == ver) {
                println!("{} Default version: {}", "✓".hex("#777BB3"), ver);
            } else {
                println!("{} Default version {} is not installed", "✗".red(), ver);
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
                println!("{} Shell integration active", "✓".hex("#777BB3"));
            } else {
                println!(
                    "{} PHM_MULTISHELL_PATH set but directory missing",
                    "✗".red()
                );
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
        println!(
            "{} Herd is still in PATH — may conflict with phm",
            "✗".red()
        );
        println!("  Remove from .zshrc: export PATH=\".../Herd/bin/:$PATH\"");
        issues += 1;
    } else {
        println!("{} No Herd conflict", "✓".hex("#777BB3"));
    }

    // Check: composer available
    let composer_check = std::process::Command::new("which").arg("composer").output();
    match composer_check {
        Ok(output) if output.status.success() => {
            println!("{} Composer found", "✓".hex("#777BB3"));
        }
        _ => {
            println!("{} Composer not found", "✗".red());
            println!("  Install with: brew install composer");
            issues += 1;
        }
    }

    // Check: stale multishell dirs
    let base = multishell::multishell_base()?;
    if base.exists() {
        let mut stale = 0;
        if let Ok(entries) = std::fs::read_dir(&base) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if let Some(pid_str) = name_str.split('_').next()
                    && let Ok(pid) = pid_str.parse::<i32>()
                    && !multishell::is_process_alive(pid)
                {
                    stale += 1;
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
            println!("{} No stale multishell directories", "✓".hex("#777BB3"));
        }
    }

    println!();
    if issues == 0 {
        println!("{}", "All checks passed!".hex("#777BB3").bold());
    } else {
        println!("{} issue(s) found", format!("{}", issues).red().bold());
    }

    Ok(())
}
