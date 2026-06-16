mod cmd;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "haul", about = "Private encrypted file rooms — no server, no NAT config")]
struct Cli {
    /// Override data directory (default: platform app data dir)
    #[arg(long, global = true)]
    data_dir: Option<std::path::PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show node ID, rooms, and connectivity status
    Status,
    /// Manage rooms
    Room {
        #[command(subcommand)]
        action: cmd::room::RoomCommand,
    },
    /// List all rooms
    Rooms,
    /// Add a file or folder to a room
    Add {
        path: std::path::PathBuf,
        #[arg(long)]
        room: String,
    },
    /// List files in a room
    Ls {
        room: String,
        #[arg(default_value = "")]
        prefix: String,
    },
    /// Fetch a file from a room
    Get {
        room: String,
        path: String,
        #[arg(long, default_value = ".")]
        out: std::path::PathBuf,
    },
    /// Keep node running so peers can connect and sync
    Serve,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("haul=info".parse()?),
        )
        .init();

    let cli = Cli::parse();
    let data_dir = match cli.data_dir {
        Some(d) => d,
        None => haul_core::data_dir()?,
    };
    let mut haul = haul_core::Haul::open(data_dir).await?;

    match cli.command {
        Command::Status => cmd::status::run(&haul).await?,
        Command::Room { action } => cmd::room::run(&mut haul, action).await?,
        Command::Rooms => cmd::rooms::run(&haul).await?,
        Command::Add { path, room } => cmd::add::run(&haul, &path, &room).await?,
        Command::Ls { room, prefix } => cmd::ls::run(&haul, &room, &prefix).await?,
        Command::Get { room, path, out } => cmd::get::run(&haul, &room, &path, &out).await?,
        Command::Serve => cmd::serve::run(&haul).await?,
    }

    haul.shutdown().await?;
    Ok(())
}
