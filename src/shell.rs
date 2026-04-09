use std::path::Path;

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum ShellKind {
    Zsh,
    Bash,
    Fish,
}

/// Generate shell initialization code.
pub fn generate_env(shell: ShellKind, multishell_path: &Path, use_on_cd: bool) -> String {
    let path_str = multishell_path.join("bin").display().to_string();

    match shell {
        ShellKind::Zsh => generate_zsh(&path_str, multishell_path, use_on_cd),
        ShellKind::Bash => generate_bash(&path_str, multishell_path, use_on_cd),
        ShellKind::Fish => generate_fish(&path_str, multishell_path, use_on_cd),
    }
}

fn generate_zsh(bin_path: &str, multishell_path: &Path, use_on_cd: bool) -> String {
    let ms_path = multishell_path.display();
    let mut out = format!(
        r#"export PATH="{bin_path}:$PATH"
export PHM_MULTISHELL_PATH="{ms_path}"
"#
    );

    if use_on_cd {
        out.push_str(
            r#"autoload -U add-zsh-hook
_phm_autoload_hook() {
  phm use --silent-if-unchanged 2>/dev/null
  rehash
}
add-zsh-hook chpwd _phm_autoload_hook
_phm_autoload_hook
"#,
        );
    }

    out
}

fn generate_bash(bin_path: &str, multishell_path: &Path, use_on_cd: bool) -> String {
    let ms_path = multishell_path.display();
    let mut out = format!(
        r#"export PATH="{bin_path}:$PATH"
export PHM_MULTISHELL_PATH="{ms_path}"
"#
    );

    if use_on_cd {
        out.push_str(
            r#"__phm_cd() {
  \builtin cd "$@" || return
  phm use --silent-if-unchanged 2>/dev/null
  hash -r
}
alias cd=__phm_cd
__phm_cd .
"#,
        );
    }

    out
}

fn generate_fish(bin_path: &str, multishell_path: &Path, use_on_cd: bool) -> String {
    let ms_path = multishell_path.display();
    let mut out = format!(
        r#"set -gx PATH "{bin_path}" $PATH
set -gx PHM_MULTISHELL_PATH "{ms_path}"
"#
    );

    if use_on_cd {
        out.push_str(
            r#"function _phm_autoload --on-variable PWD
  phm use --silent-if-unchanged 2>/dev/null
end
_phm_autoload
"#,
        );
    }

    out
}
