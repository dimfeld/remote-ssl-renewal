pub mod acme_account;
pub mod dns;
pub mod endpoint;
pub mod init;
pub mod subdomain;

use std::sync::Arc;

use clap::{Parser, Subcommand};
use deadpool_sqlite::Pool;
use eyre::Result;

#[derive(Parser, Debug)]
#[command(about)]
struct Args {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Manage subdomains
    Subdomain(subdomain::SubdomainArgs),
    /// Manage a LetsEncrypt account
    Account(acme_account::AccountArgs),
    /// Manage DNS provider accounts
    Dns(dns::DnsArgs),
    /// Manage hosts to deploy certificates
    Endpoint(endpoint::EndpointArgs),
    /// Set up a new account, DNS provider, and host in one command
    Init(init::InitArgs),
}

pub struct State {
    pub pool: Pool,
}

pub async fn run(pool: Pool) -> Result<()> {
    let args = Args::parse();

    let state = Arc::new(State { pool });

    match args.command {
        Commands::Init(args) => init::run(state, args).await?,
        Commands::Subdomain(args) => subdomain::run(state, args).await?,
        Commands::Account(args) => acme_account::run(state, args).await?,
        Commands::Dns(args) => dns::run(state, args).await?,
        Commands::Endpoint(args) => endpoint::run(state, args).await?,
    };

    Ok(())
}
