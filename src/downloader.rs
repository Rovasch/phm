use anyhow::{Context, Result};
use std::path::Path;

const STATIC_PHP_BASE: &str = "https://dl.static-php.dev/static-php-cli/common";

/// Fetch all available PHP patch versions for the given minor version (e.g. "8.3").
/// Returns exact patch versions like ["8.3.19", "8.3.20"].
pub fn fetch_versions_for_minor(minor: &str) -> Result<Vec<String>> {
    let arch = linux_arch();
    let url = format!("{}/?format=json", STATIC_PHP_BASE);

    let response = reqwest::blocking::get(&url)
        .context("failed to fetch version list from static-php-cli")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "version list fetch failed: HTTP {}",
            response.status()
        );
    }

    let body: serde_json::Value = response.json().context("failed to parse version list JSON")?;
    let filenames = extract_filenames(&body);

    let suffix = format!("-cli-linux-{}.tar.gz", arch);

    let mut matches: Vec<String> = filenames
        .into_iter()
        .filter_map(|f| {
            let ver = f.strip_prefix("php-")?.strip_suffix(&suffix)?;
            if ver.starts_with(minor) {
                Some(ver.to_string())
            } else {
                None
            }
        })
        .collect();

    matches.sort();
    Ok(matches)
}

/// Fetch all available PHP minor versions (e.g. ["8.0", "8.1", "8.2", "8.3", "8.4"]).
pub fn fetch_available_minors() -> Result<Vec<String>> {
    use std::collections::BTreeSet;

    let arch = linux_arch();
    let url = format!("{}/?format=json", STATIC_PHP_BASE);

    let response = reqwest::blocking::get(&url)
        .context("failed to fetch version list from static-php-cli")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "version list fetch failed: HTTP {}",
            response.status()
        );
    }

    let body: serde_json::Value = response.json().context("failed to parse version list JSON")?;
    let filenames = extract_filenames(&body);

    let suffix = format!("-cli-linux-{}.tar.gz", arch);
    let mut minors: BTreeSet<String> = BTreeSet::new();

    for file in filenames {
        let Some(ver) = file.strip_prefix("php-").and_then(|f| f.strip_suffix(&suffix)) else {
            continue;
        };
        let parts: Vec<&str> = ver.splitn(3, '.').collect();
        if parts.len() >= 2 {
            minors.insert(format!("{}.{}", parts[0], parts[1]));
        }
    }

    Ok(minors.into_iter().collect())
}

/// Download and install a specific PHP patch version into dest_dir.
/// The binary will be placed at dest_dir/bin/php.
pub fn download_and_install(version: &str, dest_dir: &Path) -> Result<()> {
    let arch = linux_arch();
    let filename = format!("php-{}-cli-linux-{}.tar.gz", version, arch);
    let url = format!("{}/{}", STATIC_PHP_BASE, filename);

    let response = reqwest::blocking::get(&url)
        .with_context(|| format!("failed to download {}", url))?;

    if !response.status().is_success() {
        anyhow::bail!("download failed: HTTP {}", response.status());
    }

    let bytes = response.bytes().context("failed to read response body")?;

    let cursor = std::io::Cursor::new(bytes);
    let gz = flate2::read::GzDecoder::new(cursor);
    let mut archive = tar::Archive::new(gz);

    let bin_dir = dest_dir.join("bin");
    std::fs::create_dir_all(&bin_dir)
        .with_context(|| format!("failed to create {}", bin_dir.display()))?;

    for entry in archive.entries().context("failed to read archive")? {
        let mut entry = entry.context("failed to read archive entry")?;
        let path = entry.path().context("failed to get entry path")?;
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        if name == "php" {
            let dest = bin_dir.join("php");
            entry
                .unpack(&dest)
                .with_context(|| format!("failed to extract php binary to {}", dest.display()))?;

            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))
                .context("failed to set php binary executable")?;

            return Ok(());
        }
    }

    anyhow::bail!("no 'php' binary found in downloaded archive {}", filename)
}

fn extract_filenames(json: &serde_json::Value) -> Vec<String> {
    let mut files = Vec::new();

    if let Some(arr) = json.as_array() {
        for item in arr {
            if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                files.push(name.to_string());
            } else if let Some(s) = item.as_str() {
                files.push(s.to_string());
            }
        }
    } else if let Some(obj) = json.as_object() {
        // Some directory listing APIs return {"filename": {...}, ...}
        for key in obj.keys() {
            if key.ends_with(".tar.gz") {
                files.push(key.clone());
            }
        }
    }

    files
}

#[cfg(target_arch = "x86_64")]
fn linux_arch() -> &'static str {
    "x86_64"
}

#[cfg(target_arch = "aarch64")]
fn linux_arch() -> &'static str {
    "aarch64"
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
fn linux_arch() -> &'static str {
    panic!("phm: unsupported architecture — only x86_64 and aarch64 are supported on Linux")
}
