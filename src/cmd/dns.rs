use std::sync::Arc;

use clap::{Args, Subcommand};
use eyre::Result;
use rusqlite::params;
use strum::{IntoEnumIterator, VariantNames};

use crate::{
    cli::get_unique_name,
    db::PoolExtInteract,
    dns::{vercel::VercelDnsCreds, DnsProviderType},
};

use super::State;

#[derive(Args, Debug)]
pub struct DnsArgs {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Add a new DNS provider
    New,
}

pub async fn new_dns_provider(state: Arc<State>) -> Result<String> {
    let name = get_unique_name(&state, "Give this DNS provider a label", "dns_providers").await?;

    let selection = dialoguer::Select::new()
        .with_prompt("What DNS provider do you want to use?")
        .items(DnsProviderType::VARIANTS)
        .interact()?;
    let dns_provider_type = DnsProviderType::iter().nth(selection).unwrap();

    let creds_str = match dns_provider_type {
        DnsProviderType::Vercel => VercelDnsCreds::from_console()?
            .map(|creds| serde_json::to_string(&creds))
            .transpose()?,
    };

    {
        let name = name.clone();
        state
            .pool
            .interact(move |conn| {
                conn.execute(
                    "INSERT INTO dns_providers (name, provider, creds) VALUES (?1, ?2, ?3)",
                    params![name, dns_provider_type.to_string(), creds_str],
                )?;
                Ok::<_, eyre::Report>(())
            })
            .await?;
    }

    Ok(name)
}

pub async fn run(state: Arc<State>, args: DnsArgs) -> Result<()> {
    match args.command {
        Commands::New => new_dns_provider(state).await?,
    };

    Ok(())
}
