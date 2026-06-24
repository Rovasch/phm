use crate::multishell::PHP_BINARIES;
use crate::{composer, config, discover};
use crate::version::{PhpVersion, VersionConstraint};
use anyhow::{Context, Result, bail};
use std::path::PathBuf;

pub fn shim_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".local/state/phm/shims"))
}

pub fn shim_bin_dir() -> Result<PathBuf> {
    Ok(shim_dir()?.join("bin"))
}

pub fn create_shims() -> Result<PathBuf> {
    let bin_dir = shim_bin_dir()?;
    std::fs::create_dir_all(&bin_dir)?;

    let phm_binary = find_phm_binary()?;

    for binary in PHP_BINARIES {
        let shim_path = bin_dir.join(binary);
        let _ = std::fs::remove_file(&shim_path);
        std::os::unix::fs::symlink(&phm_binary, &shim_path)
            .with_context(|| format!("failed to create shim: {}", shim_path.display()))?;
    }

    Ok(bin_dir)
}

pub fn ensure_shims() -> Result<()> {
    let bin_dir = shim_bin_dir()?;
    let php_shim = bin_dir.join("php");

    if php_shim.is_symlink() {
        if let Ok(target) = std::fs::read_link(&php_shim) {
            if let Ok(our_binary) = find_phm_binary() {
                if target == our_binary {
                    return Ok(());
                }
            }
        }
    }

    create_shims()?;
    Ok(())
}

pub fn remove_shims() -> Result<()> {
    let bin_dir = shim_bin_dir()?;
    if bin_dir.exists() {
        std::fs::remove_dir_all(&bin_dir)?;
    }
    Ok(())
}

pub fn exec_shim(binary_name: &str, args: &[String]) -> Result<()> {
    let cwd = std::env::current_dir()?;

    let constraint = match composer::find_version(&cwd)? {
        Some(c) => c,
        None => match config::get_default()? {
            Some(ver_str) => {
                let v = PhpVersion::parse(&ver_str)
                    .ok_or_else(|| anyhow::anyhow!("invalid default version: {}", ver_str))?;
                VersionConstraint::exact(v)
            }
            None => bail!(
                "no PHP version detected for {}.\n\
                 Add .php-version, composer.json require.php, or run: phm default <version>",
                cwd.display()
            ),
        },
    };

    let installations = discover::discover_versions()?;
    let versions: Vec<PhpVersion> = installations.iter().map(|i| i.version).collect();
    let resolved = constraint
        .resolve(&versions)
        .ok_or_else(|| anyhow::anyhow!("no installed PHP satisfies constraint for {}", constraint.target()))?;

    let installation = installations.iter().find(|i| i.version == resolved).unwrap();
    let real_binary = installation.bin_dir.join(binary_name);

    if !real_binary.exists() {
        bail!(
            "'{}' not found in PHP {} ({})",
            binary_name,
            resolved,
            installation.bin_dir.display()
        );
    }

    use std::os::unix::process::CommandExt;

    #[cfg(not(target_os = "macos"))]
    let err = match crate::termux::proot_wrap_args() {
        Some((proot, resolv)) => std::process::Command::new(&proot)
            .arg("-b")
            .arg(format!("{}:/etc/resolv.conf", resolv.display()))
            .arg(&real_binary)
            .args(args)
            .exec(),
        None => std::process::Command::new(&real_binary).args(args).exec(),
    };

    #[cfg(target_os = "macos")]
    let err = std::process::Command::new(&real_binary).args(args).exec();

    Err(err.into())
}

const ZSHENV_MARKER: &str = "# phm shims";

/// Dispatch to the right injection target for the detected shell.
fn find_injection_target(shell: &str, home: &std::path::Path) -> PathBuf {
    match shell {
        "bash" => find_bash_injection_target(home),
        "fish" => home.join(".config/fish/conf.d/phm.fish"),
        _ => find_zsh_injection_target(home),
    }
}

/// Find the best file to inject into for zsh.
/// Reads ~/.zshrc for sourced custom files; falls back to ~/.zshenv.
fn find_zsh_injection_target(home: &std::path::Path) -> PathBuf {
    let zshrc = home.join(".zshrc");

    if let Ok(content) = std::fs::read_to_string(&zshrc) {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') {
                continue;
            }
            if let Some(path) = extract_sourced_home_path(line, home) {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let is_custom = ["custom", "local", "extra", "personal", "private", "user", "mine"]
                    .iter()
                    .any(|kw| name.contains(kw));
                if is_custom && path.starts_with(home) {
                    return path;
                }
            }
        }

        for candidate in ["zshrc_custom", "zshrc.local", "zsh_custom", "zsh.local"] {
            if content.contains(candidate) {
                return home.join(format!(".{candidate}"));
            }
        }
    }

    home.join(".zshenv")
}

/// Find the best file to inject into for bash.
/// Reads ~/.bashrc for sourced custom files; falls back to ~/.bashrc.
fn find_bash_injection_target(home: &std::path::Path) -> PathBuf {
    let bashrc = home.join(".bashrc");

    if let Ok(content) = std::fs::read_to_string(&bashrc) {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') {
                continue;
            }
            if let Some(path) = extract_sourced_home_path(line, home) {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let is_custom = ["custom", "local", "extra", "personal", "private", "user", "mine"]
                    .iter()
                    .any(|kw| name.contains(kw));
                if is_custom && path.starts_with(home) {
                    return path;
                }
            }
        }

        for candidate in ["bashrc_custom", "bashrc.local", "bash_custom", "bash.local"] {
            if content.contains(candidate) {
                return home.join(format!(".{candidate}"));
            }
        }
    }

    bashrc
}

fn extract_sourced_home_path(line: &str, home: &std::path::Path) -> Option<PathBuf> {
    // Match: `source ~/foo`, `. ~/foo`, and guarded forms like `[[ -f ~/foo ]] && source ~/foo`
    for prefix in ["source ~/", ". ~/"] {
        if let Some(pos) = line.find(prefix) {
            let rest = line[pos + prefix.len()..].trim();
            let token = rest.split_whitespace().next()?;
            // Strip trailing quotes or semicolons
            let token = token.trim_matches(|c| c == '"' || c == '\'' || c == ';');
            return Some(home.join(token));
        }
    }
    None
}

/// Inject the shim PATH into the best available shell config file.
/// Returns the path that was written to, or None if already present.
pub fn inject_shim_path() -> Result<Option<PathBuf>> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    let shell = detect_shell_name();
    let target = find_injection_target(shell, &home);
    let bin_dir = shim_bin_dir()?;
    let line = if shell == "fish" {
        format!("set -gx PATH \"{}\" $PATH {}", bin_dir.display(), ZSHENV_MARKER)
    } else {
        format!("export PATH=\"{}:$PATH\" {}", bin_dir.display(), ZSHENV_MARKER)
    };

    for candidate in shim_path_candidates(&home) {
        if let Ok(content) = std::fs::read_to_string(&candidate) {
            if content.contains(ZSHENV_MARKER) {
                return Ok(None);
            }
        }
    }

    use std::io::Write;
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&target)?;

    writeln!(file)?;
    writeln!(file, "{}", line)?;

    Ok(Some(target))
}

/// Remove the shim PATH injection from whichever file it was written to.
pub fn remove_shim_path() -> Result<bool> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    let mut removed = false;

    for candidate in shim_path_candidates(&home) {
        if !candidate.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&candidate)?;
        if !content.contains(ZSHENV_MARKER) {
            continue;
        }
        let filtered: Vec<&str> = content
            .lines()
            .filter(|line| !line.contains(ZSHENV_MARKER))
            .collect();
        std::fs::write(&candidate, filtered.join("\n") + "\n")?;
        removed = true;
    }

    Ok(removed)
}

fn shim_path_candidates(home: &std::path::Path) -> Vec<PathBuf> {
    vec![
        home.join(".zshenv"),
        home.join(".zshrc_custom"),
        home.join(".zshrc.local"),
        home.join(".zsh_custom"),
        home.join(".zsh.local"),
        home.join(".bashrc"),
        home.join(".bash_profile"),
        home.join(".config/fish/conf.d/phm.fish"),
        home.join(".config/fish/config.fish"),
    ]
}

const SHELL_EVAL_MARKER: &str = "# phm shell";

fn detect_shell_name() -> &'static str {
    let shell = std::env::var("SHELL").unwrap_or_default();
    if shell.contains("bash") {
        "bash"
    } else if shell.contains("fish") {
        "fish"
    } else {
        "zsh"
    }
}

/// Inject shell integration into the best config file for the detected shell.
/// Returns the path written to, or None if already present.
pub fn inject_shell_eval() -> Result<Option<PathBuf>> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    let shell = detect_shell_name();
    let target = find_injection_target(shell, &home);
    let line = if shell == "fish" {
        format!("phm env --shell fish --use-on-cd | source {}", SHELL_EVAL_MARKER)
    } else {
        format!(
            "eval \"$(phm env --shell {} --use-on-cd)\" {}",
            shell, SHELL_EVAL_MARKER
        )
    };

    for candidate in shell_eval_candidates(&home) {
        if let Ok(content) = std::fs::read_to_string(&candidate) {
            if content.contains(SHELL_EVAL_MARKER) || content.contains("phm env") {
                return Ok(None);
            }
        }
    }

    use std::io::Write;
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&target)?;

    writeln!(file)?;
    writeln!(file, "{}", line)?;

    Ok(Some(target))
}

/// Remove the shell eval line from whichever file it was written to.
pub fn remove_shell_eval() -> Result<bool> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    let mut removed = false;

    for candidate in shell_eval_candidates(&home) {
        if !candidate.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&candidate)?;
        if !content.contains(SHELL_EVAL_MARKER) {
            continue;
        }
        let filtered: Vec<&str> = content
            .lines()
            .filter(|line| !line.contains(SHELL_EVAL_MARKER))
            .collect();
        std::fs::write(&candidate, filtered.join("\n") + "\n")?;
        removed = true;
    }

    Ok(removed)
}

fn shell_eval_candidates(home: &std::path::Path) -> Vec<PathBuf> {
    vec![
        home.join(".zshrc_custom"),
        home.join(".zshrc.local"),
        home.join(".zsh_custom"),
        home.join(".zsh.local"),
        home.join(".zshrc"),
        home.join(".bashrc"),
        home.join(".bash_profile"),
        home.join(".config/fish/conf.d/phm.fish"),
        home.join(".config/fish/config.fish"),
    ]
}


fn find_phm_binary() -> Result<PathBuf> {
    // current_exe() is most reliable — it's the actual binary being run, not whatever
    // `which phm` finds in PATH (which could be a different version).
    if let Ok(path) = std::env::current_exe() {
        return Ok(path);
    }

    if let Ok(output) = std::process::Command::new("which").arg("phm").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(PathBuf::from(path));
            }
        }
    }

    Err(anyhow::anyhow!("could not determine phm binary path"))
}
