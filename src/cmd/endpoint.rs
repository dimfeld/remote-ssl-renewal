use std::sync::Arc;

use clap::{Args, Subcommand};
use eyre::{eyre, Result};
use rusqlite::params;
use serde::Deserialize;
use strum::{IntoEnumIterator, VariantNames};

use crate::{
    cli::get_unique_name,
    db::PoolExtInteract,
    deploy::{digitalocean::DigitalOceanCreds, EndpointProviderType},
};

use super::State;

#[derive(Args, Debug)]
pub struct EndpointArgs {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[clap(about = "Add a new endpoint account")]
    New,
}

pub async fn new_endpoint(state: Arc<State>) -> Result<String> {
    let name = get_unique_name(&state, "endpoint", "endpoints").await?;

    let selection = dialoguer::Select::new()
        .with_prompt("What endpoint provider are you using?")
        .items(EndpointProviderType::VARIANTS)
        .interact()?;
    let endpoint_type = EndpointProviderType::iter().nth(selection).unwrap();

    let creds_str = match endpoint_type {
        EndpointProviderType::DigitalOcean => DigitalOceanCreds::from_console()?
            .map(|creds| serde_json::to_string(&creds))
            .transpose()?,
    };

    {
        let name = name.clone();
        state
            .pool
            .interact(move |conn| {
                conn.execute(
                    "INSERT INTO endpoints (name, provider, creds) VALUES (?1, ?2, ?3)",
                    params![name, endpoint_type.to_string(), creds_str],
                )?;
                Ok::<_, eyre::Report>(())
            })
            .await?;
    }

    Ok(name)
}

pub async fn run(state: Arc<State>, args: EndpointArgs) -> Result<()> {
    match args.command {
        Commands::New => new_endpoint(state).await?,
    };

    Ok(())
}
