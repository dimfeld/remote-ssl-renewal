pub mod acme_account;
pub mod dns;
pub mod endpoint;
pub mod init;
pub mod new_cert;
pub mod renew;

use std::sync::Arc;

use clap::{Parser, Subcommand};
use deadpool_sqlite::Pool;
use eyre::Result;
use instant_acme::LetsEncrypt;

#[derive(Parser, Debug)]
#[command(about)]
struct Args {
    #[clap(subcommand)]
    command: Commands,

    #[clap(
        long,
        help = "Connect to the staging environment",
        default_value_t = false
    )]
    staging: bool,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[clap(about = "Add a new certificate")]
    NewCert(new_cert::NewCertArgs),
    #[clap(about = "Renew certificates that are near expiration")]
    Renew(renew::RenewArgs),
    #[clap(about = "Manage a LetsEncrypt account")]
    Account(acme_account::AccountArgs),
    #[clap(about = "Manage DNS provider accounts")]
    Dns(dns::DnsArgs),
    #[clap(about = "Manage endpoints to deploy certificates")]
    Endpoint(endpoint::EndpointArgs),
    #[clap(about = "Set up a new account, DNS provider, and endpoint in one command")]
    Init(init::InitArgs),
}

pub struct State {
    pub pool: Pool,
    pub staging: bool,
}

impl State {
    pub fn acme_url(&self) -> &'static str {
        if self.staging {
            LetsEncrypt::Staging.url()
        } else {
            LetsEncrypt::Production.url()
        }
    }
}

pub async fn run(pool: Pool) -> Result<()> {
    let args = Args::parse();

    let state = Arc::new(State {
        pool,
        staging: args.staging,
    });

    match args.command {
        Commands::Init(args) => init::run(state, args).await?,
        Commands::NewCert(args) => new_cert::run(state, args).await?,
        Commands::Renew(args) => renew::run(args).await?,
        Commands::Account(args) => acme_account::run(state, args).await?,
        Commands::Dns(args) => dns::run(state, args).await?,
        Commands::Endpoint(args) => endpoint::run(state, args).await?,
    };

    Ok(())
}
