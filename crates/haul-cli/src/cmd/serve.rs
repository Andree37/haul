use anyhow::Result;
use haul_core::Haul;

pub async fn run(haul: &Haul) -> Result<()> {
    println!("node id: {}", haul.node.node_id());
    println!();

    let rooms: Vec<_> = haul.rooms.list().collect();
    if rooms.is_empty() {
        println!("no rooms — run `haul room create <name>` first");
    } else {
        println!("serving {} room(s):", rooms.len());
        for room in &rooms {
            println!("  {}", room.name);
        }
        // TODO(daemon): this belongs in Haul::open, not here. When daemon mode is added,
        // open should accept a SyncMode (Active | Passive) — Active activates all known
        // docs for sync on startup, Passive skips it for one-shot commands.
        haul.start_syncing_rooms().await?;
    }
    println!();
    println!("ready — peers can now join and sync (ctrl-c to stop)");

    // TODO(daemon): ctrl_c() catches only the first signal. A second Ctrl-C kills the
    // process before main.rs runs haul.shutdown(), potentially leaking the iroh router.
    // Handle SIGTERM + double-Ctrl-C gracefully when this becomes a proper daemon.
    tokio::signal::ctrl_c().await?;
    println!("\nstopping");
    Ok(())
}
