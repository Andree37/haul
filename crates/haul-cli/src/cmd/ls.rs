use anyhow::Result;
use haul_core::Haul;

pub async fn run(haul: &Haul, room: &str, prefix: &str) -> Result<()> {
    let entries = haul.ls(room, prefix).await?;
    if entries.is_empty() {
        println!("(no files in '{room}'{}", if prefix.is_empty() { String::new() } else { format!(" under '{prefix}'") });
        return Ok(());
    }
    for (path, entry) in &entries {
        let node_short = if entry.from_node.len() >= 8 { &entry.from_node[..8] } else { &entry.from_node };
        println!("{:60} {:>10} bytes  node:{}", path, entry.size, node_short);
    }
    println!("\n{} file(s)", entries.len());
    Ok(())
}
