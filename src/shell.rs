use std::path::Path;

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum ShellKind {
    Zsh,
    Bash,
    Fish,
}

/// Generate shell initialization code.
pub fn generate_env(
    shell: ShellKind,
    multishell_path: &Path,
    use_on_cd: bool,
    silent: bool,
) -> String {
    let path_str = multishell_path.join("bin").display().to_string();

    match shell {
        ShellKind::Zsh => generate_zsh(&path_str, multishell_path, use_on_cd, silent),
        ShellKind::Bash => generate_bash(&path_str, multishell_path, use_on_cd, silent),
        ShellKind::Fish => generate_fish(&path_str, multishell_path, use_on_cd, silent),
    }
}

fn generate_zsh(bin_path: &str, multishell_path: &Path, use_on_cd: bool, silent: bool) -> String {
    let ms_path = multishell_path.display();
    let mut out = format!(
        r#"export PATH="{bin_path}:$PATH"
export PHM_MULTISHELL_PATH="{ms_path}"
"#
    );
    if silent {
        out.push_str("export PHM_SILENT=1\n");
    }

    if use_on_cd {
        out.push_str(
            r#"autoload -U add-zsh-hook
_phm_autoload_hook() {
  phm use --silent-if-unchanged
  rehash
}
add-zsh-hook chpwd _phm_autoload_hook
_phm_autoload_hook
"#,
        );
    }

    out
}

fn generate_bash(bin_path: &str, multishell_path: &Path, use_on_cd: bool, silent: bool) -> String {
    let ms_path = multishell_path.display();
    let mut out = format!(
        r#"export PATH="{bin_path}:$PATH"
export PHM_MULTISHELL_PATH="{ms_path}"
"#
    );
    if silent {
        out.push_str("export PHM_SILENT=1\n");
    }

    if use_on_cd {
        out.push_str(
            r#"__phm_cd() {
  \builtin cd "$@" || return
  phm use --silent-if-unchanged
  hash -r
}
alias cd=__phm_cd
__phm_cd .
"#,
        );
    }

    out
}

fn generate_fish(bin_path: &str, multishell_path: &Path, use_on_cd: bool, silent: bool) -> String {
    let ms_path = multishell_path.display();
    let mut out = format!(
        r#"set -gx PATH "{bin_path}" $PATH
set -gx PHM_MULTISHELL_PATH "{ms_path}"
"#
    );
    if silent {
        out.push_str("set -gx PHM_SILENT 1\n");
    }

    if use_on_cd {
        out.push_str(
            r#"function _phm_autoload --on-variable PWD
  phm use --silent-if-unchanged
end
_phm_autoload
"#,
        );
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn generate_env_exports_silent_flag_for_supported_shells() {
        let path = Path::new("/tmp/phm-shell");

        let zsh = generate_env(ShellKind::Zsh, path, false, true);
        let bash = generate_env(ShellKind::Bash, path, false, true);
        let fish = generate_env(ShellKind::Fish, path, false, true);

        assert!(zsh.contains("export PHM_SILENT=1"));
        assert!(bash.contains("export PHM_SILENT=1"));
        assert!(fish.contains("set -gx PHM_SILENT 1"));
    }

    #[test]
    fn generate_env_omits_silent_flag_when_disabled() {
        let output = generate_env(ShellKind::Zsh, Path::new("/tmp/phm-shell"), true, false);

        assert!(!output.contains("PHM_SILENT"));
        assert!(output.contains("phm use --silent-if-unchanged"));
    }
}
