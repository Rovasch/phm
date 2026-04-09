use anyhow::Result;
use crate::multishell;

pub fn run() -> Result<()> {
    let ms_path = std::env::var("PHM_MULTISHELL_PATH")
        .map_err(|_| anyhow::anyhow!("PHM_MULTISHELL_PATH not set. Run: eval \"$(phm env)\""))?;

    let php_path = std::path::PathBuf::from(&ms_path).join("bin/php");

    if php_path.exists() {
        // Resolve the symlink to show the actual binary
        let resolved = std::fs::read_link(&php_path).unwrap_or(php_path);
        println!("{}", resolved.display());
    } else {
        let version = multishell::read_current(&std::path::PathBuf::from(&ms_path));
        match version {
            Some(v) => eprintln!("PHP {} is linked but binary not found", v),
            None => eprintln!("No PHP version active"),
        }
    }

    Ok(())
}
