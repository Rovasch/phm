# Changelog

## [0.2.3] - 2026-04-12

### Fixed

- `phm use` now always switches to the resolved lowest installed version for standard Composer constraints like `>=8.2`, instead of staying on any already-satisfying higher version
- Added regression coverage for the resolved-target fast path so open-ended constraints continue to pick the exact version phm intends to use

## [0.2.2] - 2026-04-12

### Fixed

- Homebrew PHP discovery now works across both `/opt/homebrew` and `/usr/local`, instead of assuming Apple Silicon paths only
- `phm use` now reports already-active explicit version requests and fails clearly if a freshly installed version still cannot be resolved
- `phm uninstall` now targets legacy `shivammathur/php` formula names for older PHP releases and verifies that the version disappears afterwards

### Changed

- Install and uninstall flows now show staged status output and check that Homebrew is actually available before running brew commands
- Doctor output now reports detected Homebrew opt directories and no longer relies on panic-prone home directory handling
- Documentation examples now use `brew --prefix` instead of hardcoded `/opt/homebrew` paths

## [0.2.1] - 2026-04-12

### Changed

- Changed green text to PHP brand color

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
