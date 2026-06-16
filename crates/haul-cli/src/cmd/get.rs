use anyhow::Result;
use haul_core::Haul;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

pub async fn run(haul: &Haul, room: &str, file_path: &str, out: &Path) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);
    spinner.set_message(format!("fetching '{file_path}' from room '{room}'..."));
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));

    haul.get(room, file_path, out).await?;

    spinner.finish_and_clear();
    println!("saved to {}/{file_path}", out.display());
    Ok(())
}
