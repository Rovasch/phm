use std::collections::HashMap;
use std::path::Path;
use anyhow::Result;
use serde::Deserialize;
use crate::version::PhpVersion;

#[derive(Deserialize)]
struct ComposerJson {
    require: Option<HashMap<String, String>>,
}

/// Walk up from `start_dir` looking for .php-version or composer.json with require.php.
/// .php-version takes priority over composer.json at the same directory level.
pub fn find_version(start_dir: &Path) -> Result<Option<PhpVersion>> {
    let mut current = start_dir.to_path_buf();

    loop {
        // Check .php-version first (higher priority)
        let php_version_file = current.join(".php-version");
        if php_version_file.exists() {
            let content = std::fs::read_to_string(&php_version_file)?;
            if let Some(version) = PhpVersion::parse(content.trim()) {
                return Ok(Some(version));
            }
        }

        // Check composer.json
        let composer_file = current.join("composer.json");
        if composer_file.exists() {
            if let Some(version) = parse_composer_json(&composer_file)? {
                return Ok(Some(version));
            }
        }

        // Move to parent directory
        if !current.pop() {
            break;
        }
    }

    Ok(None)
}

fn parse_composer_json(path: &Path) -> Result<Option<PhpVersion>> {
    let content = std::fs::read_to_string(path)?;
    let composer: ComposerJson = match serde_json::from_str(&content) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };

    if let Some(require) = composer.require {
        if let Some(php_constraint) = require.get("php") {
            return Ok(PhpVersion::from_constraint(php_constraint));
        }
    }

    Ok(None)
}
