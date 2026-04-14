use crate::composer;
use crate::config;
use crate::discover;
use crate::multishell;
use crate::version::{PhpVersion, VersionConstraint};
use anyhow::{Result, bail};
use colored_text::Colorize;
use std::io::{self, Write};

pub fn run(version: Option<String>, silent_if_unchanged: bool, silent: bool) -> Result<()> {
    let ms_path = std::env::var("PHM_MULTISHELL_PATH")
        .map_err(|_| anyhow::anyhow!("PHM_MULTISHELL_PATH not set. Run: eval \"$(phm env)\""))?;
    let ms_path = std::path::PathBuf::from(ms_path);
    let output = UseOutput::new(silent_if_unchanged, silent);

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
                        let v = PhpVersion::parse(&ver_str).ok_or_else(|| {
                            anyhow::anyhow!("invalid default version: {}", ver_str)
                        })?;
                        VersionConstraint::exact(v)
                    }
                    None => {
                        if output.skip_when_no_version_target() {
                            return Ok(());
                        }
                        bail!(
                            "no PHP version specified and no default set. Run: phm default <version>"
                        );
                    }
                }
            }
        }
    };

    // Find the best installed version satisfying the constraint
    let installations = discover::discover_versions()?;
    let versions: Vec<PhpVersion> = installations.iter().map(|i| i.version).collect();
    let resolved = constraint.resolve(&versions);

    // Fast path: current version already matches the resolved target.
    if let Some(target) = resolved
        && current_matches_target(current.as_deref(), target)
    {
        if output.should_print_unchanged(version.is_some()) {
            println!(
                "Already using {}",
                format!("PHP {}", target).hex("#777BB3").bold()
            );
        }
        return Ok(());
    }

    match resolved.and_then(|v| installations.iter().find(|i| i.version == v)) {
        Some(inst) => {
            multishell::link_version(&ms_path, inst)?;
            output.print_success(format!(
                "Using {}",
                format!("PHP {}", inst.version).hex("#777BB3").bold()
            ));
        }
        None => {
            let target = constraint.target();

            // Prompt to install if running in an interactive terminal
            if atty::is(atty::Stream::Stdin) {
                print!(
                    "PHP {} is not installed. Do you want to install it? {} ",
                    target.to_string().bold(),
                    "[y/N]".dim()
                );
                io::stdout().flush()?;

                let mut answer = String::new();
                io::stdin().read_line(&mut answer)?;

                if answer.trim().eq_ignore_ascii_case("y") {
                    super::install::run(&target.to_string())?;

                    // Switch to the newly installed version
                    let new_installations = discover::discover_versions()?;
                    let new_versions: Vec<_> =
                        new_installations.iter().map(|i| i.version).collect();
                    if let Some(v) = constraint.resolve(&new_versions)
                        && let Some(inst) = new_installations.iter().find(|i| i.version == v)
                    {
                        multishell::link_version(&ms_path, inst)?;
                        output.print_success(format!(
                            "Using {}",
                            format!("PHP {}", inst.version).hex("#777BB3").bold()
                        ));
                    } else {
                        bail!(
                            "PHP {} was installed but could not be resolved afterwards. Run: phm doctor",
                            target
                        );
                    }
                }
            } else {
                println!(
                    "{} PHP {} is not installed. Run: {}",
                    "warning:".yellow().bold(),
                    target,
                    format!("phm install {}", target).cyan()
                );
            }
        }
    }

    Ok(())
}

fn current_matches_target(current: Option<&str>, target: PhpVersion) -> bool {
    current.and_then(PhpVersion::parse) == Some(target)
}

#[derive(Debug, Clone, Copy)]
struct UseOutput {
    silent_if_unchanged: bool,
    silent: bool,
}

impl UseOutput {
    fn new(silent_if_unchanged: bool, silent_flag: bool) -> Self {
        Self {
            silent_if_unchanged,
            silent: silent_flag || shell_prefers_silent(),
        }
    }

    fn skip_when_no_version_target(self) -> bool {
        self.silent_if_unchanged
    }

    fn should_print_unchanged(self, explicit_version: bool) -> bool {
        explicit_version && !self.silent
    }

    fn print_success(self, message: String) {
        if !self.silent {
            println!("{}", message);
        }
    }
}

fn shell_prefers_silent() -> bool {
    matches!(
        std::env::var("PHM_SILENT").as_deref(),
        Ok("1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn current_only_matches_when_it_equals_the_resolved_target() {
        let target = PhpVersion::new(8, 2);

        assert!(current_matches_target(Some("8.2"), target));
        assert!(!current_matches_target(Some("8.5"), target));
        assert!(!current_matches_target(None, target));
    }

    #[test]
    fn open_ended_constraints_still_resolve_to_the_lowest_matching_version() {
        let installed = vec![
            PhpVersion::new(8, 2),
            PhpVersion::new(8, 4),
            PhpVersion::new(8, 5),
        ];

        let resolved = VersionConstraint::from_constraint(">=8.2")
            .unwrap()
            .resolve(&installed)
            .unwrap();

        assert_eq!(resolved, PhpVersion::new(8, 2));
        assert!(!current_matches_target(Some("8.5"), resolved));
    }

    #[test]
    fn use_output_suppresses_success_when_silent_flag_is_set() {
        let output = UseOutput::new(false, true);

        assert!(!output.should_print_unchanged(true));
    }

    #[test]
    fn use_output_reads_silent_preference_from_shell_env() {
        let _guard = env_lock().lock().unwrap();
        unsafe {
            std::env::set_var("PHM_SILENT", "1");
        }

        let output = UseOutput::new(false, false);

        unsafe {
            std::env::remove_var("PHM_SILENT");
        }

        assert!(!output.should_print_unchanged(true));
    }

    #[test]
    fn use_output_keeps_unchanged_message_when_not_silent() {
        let _guard = env_lock().lock().unwrap();
        unsafe {
            std::env::remove_var("PHM_SILENT");
        }

        let output = UseOutput::new(false, false);

        assert!(output.should_print_unchanged(true));
        assert!(!output.skip_when_no_version_target());
    }

    #[test]
    fn silent_if_unchanged_only_controls_missing_target_fast_path() {
        let output = UseOutput::new(true, false);

        assert!(output.skip_when_no_version_target());
        assert!(output.should_print_unchanged(true));
    }
}
