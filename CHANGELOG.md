# Changelog

## [0.2.0] - 2026-04-11

### Added

- Interactive install prompt: when a required PHP version is missing, phm asks to install it (like fnm)
- Shell hooks now run without stderr suppression so prompts work interactively

### Fixed

- Composer wildcard constraints (`8.4.*`) no longer fall back to higher versions — if 8.4 is not installed, phm now correctly reports the error instead of silently switching to 8.5

### Changed

- Version resolution now tracks constraint upper bounds via `VersionConstraint` struct, properly modeling Composer semantics (`8.4.*` = exact, `^8.4` = same major, `>=8.4` = open-ended)
- Fast-path check uses `satisfies()` instead of exact string match, avoiding redundant re-linking

## [0.1.1] - 2026-04-09

### Fixed

- Critical panic during shell init when no PHP versions are installed
- Path deduplication logic for bare `php` vs `php@X.Y` Homebrew formulae
- Double clone in version resolution (added `Copy` derive to `PhpVersion`)

### Changed

- Simplified composer.json parsing API (removed unused `VersionSource` enum)
- Extracted process liveness check into shared utility
- Removed all dead code (8 compiler warnings → 0)
- Updated GitHub Actions to latest versions (checkout v6, upload-artifact v7, download-artifact v8)

## [0.1.0] - 2026-04-08

### Added

- Initial release
- Per-shell PHP version switching via symlinks
- Auto-detect PHP version from `.php-version` and `composer.json`
- Automatic switching on `cd` via shell hook
- Commands: `env`, `use`, `default`, `list`, `current`, `which`, `install`, `uninstall`, `exec`, `doctor`, `completions`
- Shell support: zsh, bash, fish
- Homebrew tap installation (`brew tap Rovasch/phm`)
