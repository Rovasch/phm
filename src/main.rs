mod commands;
mod composer;
mod config;
mod discover;
#[cfg(any(target_os = "linux", target_os = "android"))]
mod downloader;
mod multishell;
mod shell;
mod shim;
#[cfg(not(target_os = "macos"))]
mod termux;
mod version;

use clap::{Parser, Subcommand};
use clap_complete::Shell;
use shell::ShellKind;

#[derive(Parser)]
#[command(name = "phm", about = "Fast PHP version manager", version)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Output shell initialization code
    Env {
        /// Shell type
        #[arg(long, default_value = "zsh")]
        shell: ShellKind,

        /// Hook into cd to auto-switch PHP versions
        #[arg(long)]
        use_on_cd: bool,

        /// Export PHM_SILENT=1 for this shell session
        #[arg(long)]
        silent: bool,
    },

    /// Switch the current shell's PHP version
    Use {
        /// PHP version (e.g., 8.2). Omit to auto-detect from .php-version/composer.json
        version: Option<String>,

        /// Suppress output when the version doesn't change
        #[arg(long)]
        silent_if_unchanged: bool,

        /// Suppress success output for this invocation
        #[arg(long)]
        silent: bool,
    },

    /// Set or show the default PHP version
    Default {
        /// PHP version to set as default. Omit to show current default
        version: Option<String>,
    },

    /// List installed PHP versions
    List,

    /// List PHP versions available to install (Linux only)
    ListRemote,

    /// Show the active PHP version
    Current,

    /// Print the path to the active PHP binary
    Which,

    /// Install a PHP version via Homebrew
    Install {
        /// PHP version to install (e.g., 8.2)
        version: String,
    },

    /// Uninstall a PHP version via Homebrew
    Uninstall {
        /// PHP version to uninstall (e.g., 8.2)
        version: String,
    },

    /// Run a command with a specific PHP version
    Exec {
        /// PHP version to use (e.g., 8.2)
        version: String,

        /// Command and arguments to run
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },

    /// Manage shim binaries for non-interactive shells
    Shim {
        #[command(subcommand)]
        action: ShimAction,
    },

    /// Diagnose common issues
    Doctor,
}

#[derive(Subcommand)]
enum ShimAction {
    /// Create shim symlinks in the shim directory
    Create,
    /// Print the shim bin directory path
    Path,
    /// Remove all shim symlinks
    Remove,
}

fn main() {
    #[cfg(not(target_os = "macos"))]
    termux::ensure_dns();

    let arg0 = std::env::args().next().unwrap_or_default();
    let invoked_as = std::path::Path::new(&arg0)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("");

    if invoked_as != "phm" && multishell::PHP_BINARIES.contains(&invoked_as) {
        let args: Vec<String> = std::env::args().skip(1).collect();
        if let Err(e) = shim::exec_shim(invoked_as, &args) {
            eprintln!("phm: {}", e);
            std::process::exit(1);
        }
        return;
    }

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Env {
            shell,
            use_on_cd,
            silent,
        } => commands::env::run(shell, use_on_cd, silent),
        Commands::Use {
            version,
            silent_if_unchanged,
            silent,
        } => commands::use_version::run(version, silent_if_unchanged, silent),
        Commands::Default { version } => commands::default::run(version),
        Commands::List => commands::list::run(),
        Commands::ListRemote => commands::list_remote::run(),
        Commands::Current => commands::current::run(),
        Commands::Which => commands::which::run(),
        Commands::Install { version } => commands::install::run(&version),
        Commands::Uninstall { version } => commands::uninstall::run(&version),
        Commands::Exec { version, command } => commands::exec::run(&version, &command),
        Commands::Shim { action } => commands::shim::run(action),
        Commands::Completions { shell } => commands::completions::run(shell),
        Commands::Doctor => commands::doctor::run(),
    };

    if let Err(e) = result {
        eprintln!("phm: {}", e);
        std::process::exit(1);
    }
}
