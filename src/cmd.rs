pub mod new_cert;
pub mod renew;

use clap::{Parser, Subcommand};
use deadpool_sqlite::Pool;
use eyre::Result;

#[derive(Parser, Debug)]
struct Args {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[clap(about = "Add a new certificate")]
    New(new_cert::NewCertArgs),
    #[clap(about = "Renew certificates that are near expiration")]
    Renew(renew::RenewArgs),
}

pub async fn run(pool: Pool) -> Result<()> {
    let args = Args::parse();
    match args.command {
        Commands::New(args) => new_cert::run(args).await?,
        Commands::Renew(args) => renew::run(args).await?,
    };

    Ok(())
}
