use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{Shell, generate};

pub fn run(shell: Shell) -> Result<()> {
    let mut cmd = crate::Cli::command();
    generate(shell, &mut cmd, "phm", &mut std::io::stdout());
    Ok(())
}
