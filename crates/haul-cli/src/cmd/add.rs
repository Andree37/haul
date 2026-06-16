use anyhow::Result;
use haul_core::Haul;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

pub async fn run(haul: &Haul, path: &Path, room: &str) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);
    spinner.set_message(format!("adding to room '{room}'..."));
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));

    let count = haul.add(path, room).await?;

    spinner.finish_and_clear();
    println!("added {count} file(s) to '{room}'");
    Ok(())
}
