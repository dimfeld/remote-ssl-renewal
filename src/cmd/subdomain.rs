mod edit;
mod new;
mod renew;

use std::sync::Arc;

use clap::{Args, Subcommand};
use eyre::Result;

use super::State;

#[derive(Args, Debug)]
pub struct SubdomainArgs {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Add a new subdomain
    New(new::NewCertArgs),
    /// Renew subdomains as needed
    Renew(renew::RenewArgs),
    /// Edit the settings for a subdomain
    Edit(edit::EditArgs),
}

pub async fn run(state: Arc<State>, args: SubdomainArgs) -> Result<()> {
    match args.command {
        Commands::New(args) => new::run(state, args).await?,
        Commands::Renew(args) => renew::run(state, args).await?,
        Commands::Edit(args) => edit::run(state, args).await?,
    };

    Ok(())
}
