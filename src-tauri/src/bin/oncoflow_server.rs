use clap::{Parser, Subcommand};
use std::{net::SocketAddr, path::PathBuf};

#[derive(Parser)]
#[command(about = "OncoFlow server for installed desktop clients on a private LAN")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// With the server stopped, create and validate a new database backup file.
    Backup {
        #[arg(long)]
        data_dir: PathBuf,
        #[arg(long)]
        destination: PathBuf,
    },
    /// Generate a unique server certificate. Share only server-certificate.der with clients.
    Init {
        #[arg(long)]
        data_dir: PathBuf,
    },
    /// Start the encrypted server. The data directory must be on a local disk.
    Run {
        #[arg(long)]
        data_dir: PathBuf,
        #[arg(long, default_value = "127.0.0.1:7443")]
        listen: SocketAddr,
    },
}

fn main() {
    let result = match Args::parse().command {
        Command::Backup {
            data_dir,
            destination,
        } => oncoflow_lib::lan::backup_server(&data_dir, &destination),
        Command::Init { data_dir } => oncoflow_lib::lan::initialize_server(&data_dir),
        Command::Run { data_dir, listen } => oncoflow_lib::lan::run_server(&data_dir, listen),
    };
    if result.is_err() {
        eprintln!("Server operation failed. Check the data directory, certificate/key, database compatibility, and whether another process uses the directory or port. No database contents were logged.");
        std::process::exit(1);
    }
}
