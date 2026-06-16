use anyhow::Result;
use clap::Subcommand;
use haul_core::Haul;

#[derive(Subcommand)]
pub enum RoomCommand {
    /// Create a new room and print the invite ticket
    Create { name: String },
    /// Join a room from an invite ticket
    Join { ticket: String },
    /// Generate a new invite ticket for an existing room
    Invite { room: String },
}

pub async fn run(haul: &mut Haul, action: RoomCommand) -> Result<()> {
    match action {
        RoomCommand::Create { name } => {
            let ticket = haul.room_create(&name).await?;
            println!("room '{}' created", name);
            println!();
            println!("invite ticket:");
            println!("{}", ticket.encode()?);
        }
        RoomCommand::Join { ticket } => {
            println!("joining room — syncing with peers (up to 30s)...");
            let (name, synced) = haul.room_join(&ticket).await?;
            if synced > 0 {
                println!("joined room '{name}' — synced {synced} file(s)");
            } else {
                println!("joined room '{name}' — no peers online yet, sync will happen when a peer connects");
            }
        }
        RoomCommand::Invite { room } => {
            let ticket = haul.room_invite(&room).await?;
            println!("invite ticket for '{room}':");
            println!("{}", ticket.encode()?);
        }
    }
    Ok(())
}
