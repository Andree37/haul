use anyhow::Result;
use haul_core::Haul;

pub async fn run(haul: &Haul) -> Result<()> {
    println!("node id: {}", haul.node.node_id());
    println!();
    println!("rooms:");
    let rooms: Vec<_> = haul.rooms.list().collect();
    if rooms.is_empty() {
        println!("  (none — run `haul room create <name>` to start)");
    } else {
        for room in rooms {
            println!("  {}", room.name);
        }
    }
    Ok(())
}
