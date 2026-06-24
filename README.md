# phm

Fast PHP version manager for macOS and Linux, written in Rust. Inspired by [fnm](https://github.com/Schniz/fnm).

phm manages PHP versions with **per-shell switching** and **automatic version detection** from `.php-version` files and `composer.json`. Switching is instant — it just repoints symlinks, no process restarts or shims.

## Install

### macOS

```sh
brew tap Rovasch/phm
brew install phm
```

### Linux / Termux

Download the binary for your architecture from the [latest release](https://github.com/Rovasch/phm/releases/latest):

| Platform | Binary |
|---|---|
| Linux x86_64 | `phm-x86_64-unknown-linux-gnu.tar.gz` |
| Linux ARM64 | `phm-aarch64-unknown-linux-gnu.tar.gz` |
| Termux (Android ARM64) | `phm-aarch64-unknown-linux-musl.tar.gz` |

```sh
curl -fsSL https://github.com/Rovasch/phm/releases/latest/download/phm-x86_64-unknown-linux-gnu.tar.gz \
  | tar xz -C ~/.local/bin
```

### Via Cargo

```sh
cargo install phm
```

### Build from source

```sh
git clone https://github.com/Rovasch/phm.git
cd phm
cargo build --release
```

## Setup

Run `phm shim create` — this installs phm into your shell automatically:

```sh
phm shim create
```

It detects your current shell and writes to the appropriate config file:

| Shell | Config target | What gets written |
|---|---|---|
| Zsh | `~/.zshenv` (or sourced custom file) | Shim `PATH` + `eval "$(phm env --shell zsh --use-on-cd)"` |
| Bash | `~/.bashrc` (or sourced custom file) | Shim `PATH` + `eval "$(phm env --shell bash --use-on-cd)"` |
| Fish | `~/.config/fish/conf.d/phm.fish` | Shim `PATH` + `phm env --shell fish --use-on-cd \| source` |

Then open a new terminal session, or reload your config:

```sh
# Zsh
source ~/.zshrc

# Bash
source ~/.bashrc

# Fish (restart terminal, or):
source ~/.config/fish/conf.d/phm.fish
```

### Manual setup

If you prefer to configure manually, add to your shell config:

**Zsh** (`~/.zshrc`):
```sh
eval "$(phm env --shell zsh --use-on-cd)"
```

If your prompt already shows the active PHP version, you can opt into a fully silent session:
```sh
eval "$(phm env --shell zsh --use-on-cd --silent)"
```

**Bash** (`~/.bashrc`):
```sh
eval "$(phm env --shell bash --use-on-cd)"
```

**Fish** (`~/.config/fish/config.fish`):
```sh
phm env --shell fish --use-on-cd | source
```

## Installing PHP versions

### macOS

phm uses Homebrew. Versions 8.x and up use the standard formula; 7.x taps `shivammathur/php`:

```sh
phm install 8.4
phm install 7.4      # taps shivammathur/php automatically
phm default 8.4
```

### Linux

phm downloads pre-built static PHP binaries (PHP 8.x) from [static-php-cli](https://github.com/crazywhalecc/static-php-cli):

```sh
phm install 8.3      # downloads ~7MB static binary, no sudo required
phm install 8.4
phm default 8.3
```

PHP 7.x is not available as a managed install on Linux. Install it via your system package manager, and phm will discover and switch it automatically:

```sh
# Ubuntu/Debian
sudo apt install php7.4

# Arch (AUR)
yay -S php74

# Then phm sees it:
phm list
phm use 7.4
```

To browse available versions:

```sh
phm list-remote
```

### Termux (Android)

```sh
pkg install php        # or a specific version like php8.3
phm shim create        # sets up shell integration
phm list               # discovers the installed PHP
```

## How it works

When your shell starts, `phm env` creates a per-shell directory with symlinks pointing to your active PHP version's binaries:

```
~/.local/state/phm/multishells/<shell-id>/bin/
  php       -> /path/to/php@8.4/bin/php
  phpize    -> /path/to/php@8.4/bin/phpize
  ...
```

This directory is prepended to your `PATH`. Switching versions just repoints the symlinks — no process restarts, no global state changes.

### Per-shell isolation

Each terminal session gets its own symlink directory. Running `phm use 8.2` in one terminal does not affect other terminals. Work on two projects requiring different PHP versions simultaneously.

### Automatic version switching

With `--use-on-cd`, phm hooks into your shell's directory change event. When you `cd` into a directory containing `composer.json` or `.php-version`, phm switches automatically.

Version files checked:

1. **`.php-version`** — plain text file containing the version (e.g., `8.2`). Takes priority.
2. **`composer.json`** — reads the `require.php` constraint and resolves to the lowest matching installed version.

The search walks up parent directories, so a `.php-version` at the repo root covers all subdirectories.

**Constraint examples:**

| composer.json require | Resolved version |
|---|---|
| `>=8.2` | 8.2 |
| `^8.2` | 8.2 |
| `~8.2` | 8.2 |
| `^7.4 \|\| ^8.0` | 8.0 |
| `8.2.*` | 8.2 |

When the version doesn't change between directories, phm exits silently with no overhead.

## Commands

### `phm use [version]`

Switch the current shell to a specific PHP version. Without a version argument, auto-detects from `.php-version` or `composer.json`.

```sh
phm use 8.2          # Switch to PHP 8.2
phm use              # Auto-detect from project files
phm use --silent 8.2 # Suppress success output for this invocation
```

### `phm default [version]`

Set or show the default PHP version used for new shells.

```sh
phm default 8.4      # Set default
phm default          # Show current default
```

### `phm list`

List all installed PHP versions. Marks the current and default versions.

```
$ phm list
  7.4
* 8.2 (current)
  8.4 (default)
  8.5
```

### `phm list-remote` *(Linux only)*

List PHP versions available to download and install.

```
$ phm list-remote
  8.0
  8.1
  8.2
* 8.3 (installed)
  8.4
```

### `phm install <version>`

Install a PHP version.

- **macOS**: uses Homebrew
- **Linux**: downloads a static binary from static-php-cli (PHP 8.x only)

```sh
phm install 8.3
phm install 7.4      # macOS: taps shivammathur/php
                     # Linux: prints package manager hint
```

### `phm uninstall <version>`

Uninstall a phm-managed PHP version. Prevents uninstalling the default version.
On Linux, system-installed PHP (e.g. from apt/pacman) must be removed via the system package manager.

```sh
phm uninstall 8.3
```

### `phm exec <version> -- <command>`

Run a command with a specific PHP version without switching the shell.

```sh
phm exec 8.1 -- php -v
phm exec 8.1 -- composer install
```

### `phm current`

Print the active PHP version.

### `phm which`

Print the resolved path to the active `php` binary. Useful for IDE configuration.

### `phm shim create`

Set up shims for non-interactive shells (IDEs, CI, scripts). Writes the shim `PATH` and `eval` line to your shell config automatically.

```sh
phm shim create
```

### `phm doctor`

Diagnose common issues: missing versions, stale state, PATH conflicts, composer availability.

```
$ phm doctor
✓ 3 PHP version(s) found: 7.4, 8.2, 8.5
✓ Default version: 8.5
✓ Shell integration active
✓ Composer found
✓ No stale multishell directories

All checks passed!
```

### `phm completions <shell>`

Generate shell completions.

```sh
# Zsh (add to .zshrc)
eval "$(phm completions zsh)"

# Bash (macOS/Homebrew)
phm completions bash > "$(brew --prefix)/etc/bash_completion.d/phm"

# Bash (Linux)
phm completions bash > ~/.local/share/bash-completion/completions/phm

# Fish
phm completions fish > ~/.config/fish/completions/phm.fish
```

## Why phm?

| | phm | Herd | brew-php-switcher |
|---|---|---|---|
| Switch speed | ~1ms (symlink swap) | ~100ms | ~2s (brew link/unlink) |
| Per-shell versions | Yes | No (global) | No (global) |
| Auto-switch on cd | Yes | No | No |
| Multi-terminal | Yes | No | No |
| Linux support | Yes | No | No |
| Written in | Rust | PHP/Electron | Bash/Ruby |

## Requirements

### macOS
- Apple Silicon or Intel
- [Homebrew](https://brew.sh)

### Linux
- x86_64 or ARM64
- No root required for phm-managed versions (PHP 8.x)

### Termux
- ARM64 Android device
- Use the `aarch64-unknown-linux-musl` binary

## License

[MIT](LICENSE)
