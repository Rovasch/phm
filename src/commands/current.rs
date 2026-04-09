use anyhow::Result;
use crate::multishell;

pub fn run() -> Result<()> {
    let ms_path = std::env::var("PHM_MULTISHELL_PATH")
        .map_err(|_| anyhow::anyhow!("PHM_MULTISHELL_PATH not set. Run: eval \"$(phm env)\""))?;

    match multishell::read_current(&std::path::PathBuf::from(ms_path)) {
        Some(version) => println!("{}", version),
        None => eprintln!("No PHP version active"),
    }

    Ok(())
}
