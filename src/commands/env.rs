use crate::config;
use crate::discover;
use crate::multishell;
use crate::shell::ShellKind;
use crate::version::PhpVersion;
use anyhow::Result;

pub fn run(shell: ShellKind, use_on_cd: bool, silent: bool) -> Result<()> {
    // Get the parent shell's PID
    let ppid = std::os::unix::process::parent_id();

    // Create a multishell directory for this shell session
    let ms_path = multishell::create_multishell(ppid)?;

    // Get default version and link it
    let installations = discover::discover_versions()?;

    if installations.is_empty() {
        eprintln!("phm: no PHP versions found. Install one with: brew install php@8.2");
    } else {
        // Determine which version to link
        let default_ver = config::get_default()?;
        let installation = if let Some(ref ver_str) = default_ver {
            if let Some(ver) = PhpVersion::parse(ver_str) {
                installations.iter().find(|i| i.version == ver)
            } else {
                None
            }
        } else {
            None
        };

        // Fall back to the highest installed version
        let installation = match installation.or_else(|| installations.last()) {
            Some(inst) => inst,
            None => anyhow::bail!("no PHP versions found"),
        };
        multishell::link_version(&ms_path, installation)?;
    }

    // Clean up stale multishell directories
    multishell::cleanup_stale();

    // Output shell initialization code
    let output = crate::shell::generate_env(shell, &ms_path, use_on_cd, silent);
    print!("{}", output);

    Ok(())
}
