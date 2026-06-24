use crate::ShimAction;
use crate::shim;
use anyhow::Result;
use colored_text::Colorize;

pub fn run(action: ShimAction) -> Result<()> {
    match action {
        ShimAction::Create => {
            let path = shim::create_shims()?;
            println!(
                "Shims created in {}",
                path.display().to_string().hex("#777BB3").bold()
            );

            match shim::inject_shim_path() {
                Ok(Some(target)) => println!(
                    "Added shim PATH to {}",
                    target.display().to_string().bold()
                ),
                Ok(None) => println!("Shim PATH already configured"),
                Err(e) => {
                    eprintln!("phm: warning: could not update shell config: {}", e);
                    println!("Add manually to ~/.zshenv or ~/.zshrc_custom:");
                    println!("  export PATH=\"{}:$PATH\"", path.display());
                }
            }

            match shim::inject_shell_eval() {
                Ok(Some(target)) => println!(
                    "Added shell integration to {}",
                    target.display().to_string().bold()
                ),
                Ok(None) => println!("Shell integration already configured"),
                Err(e) => {
                    eprintln!("phm: warning: could not write shell eval: {}", e);
                    println!("Add manually to ~/.zshrc_custom:");
                    println!("  eval \"$(phm env --shell zsh --use-on-cd)\"");
                }
            }
        }
        ShimAction::Path => {
            let path = shim::shim_bin_dir()?;
            println!("{}", path.display());
        }
        ShimAction::Remove => {
            shim::remove_shims()?;
            shim::remove_shell_eval()?;
            if shim::remove_shim_path()? {
                println!("Shims removed and shell config cleaned up");
            } else {
                println!("Shims removed");
            }
        }
    }
    Ok(())
}
