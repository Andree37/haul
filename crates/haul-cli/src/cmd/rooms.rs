use anyhow::Result;
use haul_core::Haul;

pub async fn run(haul: &Haul) -> Result<()> {
    let rooms: Vec<_> = haul.rooms.list().collect();
    if rooms.is_empty() {
        println!("no rooms yet — run `haul room create <name>`");
        return Ok(());
    }
    for room in rooms {
        println!("{}", room.name);
    }
    Ok(())
}
