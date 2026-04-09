# phm

Fast PHP version manager for macOS, written in Rust. Inspired by [fnm](https://github.com/Schniz/fnm).

phm manages Homebrew-installed PHP versions with **per-shell switching** and **automatic version detection** from `.php-version` files and `composer.json`. Switching is instant — it just repoints symlinks, no process restarts or shims.

## Install

```sh
brew tap Rovasch/phm
brew install phm
```

Or build from source:

```sh
git clone https://github.com/Rovasch/phm.git
cd phm
cargo build --release
cp target/release/phm /opt/homebrew/bin/phm
```

## Setup

Add to your shell config:

**Zsh** (`~/.zshrc`):
```sh
eval "$(phm env --shell zsh --use-on-cd)"
```

**Bash** (`~/.bashrc`):
```sh
eval "$(phm env --shell bash --use-on-cd)"
```

**Fish** (`~/.config/fish/config.fish`):
```sh
phm env --shell fish --use-on-cd | source
```

Then install PHP versions via Homebrew:

```sh
phm install 8.4
phm install 8.2
phm default 8.4
```

## How it works

When your shell starts, `phm env` creates a per-shell directory with symlinks pointing to your default PHP version's binaries:

```
~/.local/state/phm/multishells/<shell-id>/bin/
  php       -> /opt/homebrew/opt/php@8.4/bin/php
  phpize    -> /opt/homebrew/opt/php@8.4/bin/phpize
  pecl      -> /opt/homebrew/opt/php@8.4/bin/pecl
  ...
```

This directory is prepended to your `PATH`. Switching versions just repoints the symlinks — there are no shims, no process restarts, and no global state changes.

### Per-shell isolation

Each terminal session gets its own symlink directory. Running `phm use 8.2` in one terminal does not affect other terminals. This means you can work on two projects requiring different PHP versions simultaneously.

### Automatic version switching

With `--use-on-cd`, phm hooks into your shell's directory change event. When you `cd` into a project, it looks for:

1. **`.php-version`** — a plain text file containing the version (e.g., `8.2`). Takes priority.
2. **`composer.json`** — reads the `require.php` constraint and resolves it to the lowest matching installed version.

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

### `phm install <version>`

Install a PHP version via Homebrew. Automatically taps `shivammathur/php` for older versions.

```sh
phm install 8.3      # brew install php@8.3
phm install 7.4      # brew tap shivammathur/php && brew install ...
```

### `phm uninstall <version>`

Uninstall a PHP version via Homebrew. Prevents uninstalling the default version.

```sh
phm uninstall 7.4
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

```
$ phm which
/opt/homebrew/opt/php@8.2/bin/php
```

### `phm doctor`

Diagnose common issues: missing versions, stale state, PATH conflicts, composer availability.

```
$ phm doctor
✓ 3 PHP version(s) found: 7.4, 8.2, 8.5
✓ Default version: 8.5
✓ Shell integration active
✓ No Herd conflict
✓ Composer found
✓ No stale multishell directories

All checks passed!
```

### `phm completions <shell>`

Generate shell completions.

```sh
# Zsh (add to .zshrc)
eval "$(phm completions zsh)"

# Bash
phm completions bash > /opt/homebrew/etc/bash_completion.d/phm

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
| Written in | Rust | PHP/Electron | Bash/Ruby |

## Requirements

- macOS (Apple Silicon or Intel)
- [Homebrew](https://brew.sh)
- PHP installed via Homebrew (`brew install php@8.2`)

## License

[MIT](LICENSE)
