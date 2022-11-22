use std::sync::Arc;

use clap::Args;
use eyre::{eyre, Result};
use serde::Deserialize;

use crate::{
    cli::get_unique_name, db::PoolExtInteract, deploy::EndpointProviderType, dns::DnsProviderType,
};

use super::State;

#[derive(Args, Debug)]
pub struct NewCertArgs {}

pub async fn run(state: Arc<State>, args: NewCertArgs) -> Result<()> {
    let subdomain =
        get_unique_name(&state, "Which subdomain are you adding?", "subdomains").await?;

    state
        .pool
        .interact(|conn| {
            let mut stmt =
                conn.prepare_cached("SELECT id, name FROM acme_accounts ORDER BY name")?;
            let mut acme_accounts = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<Result<Vec<(i64, String)>, _>>()?;

            if acme_accounts.is_empty() {
                return Err(eyre!("No ACME accounts found. Please create one first. You may want to use the `init` command."));
            }

            let mut stmt =
                conn.prepare_cached("SELECT id, name FROM dns_providers ORDER BY name")?;
            let mut dns_providers = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<Result<Vec<(i64, String)>, _>>()?;
            if dns_providers.is_empty() {
                return Err(eyre!("No DNS providers found. Please create one first. You may want to use the `init` command."));
            }

            let mut stmt = conn.prepare_cached("SELECT id, name FROM endpoints ORDER BY name")?;
            let mut endpoints = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<Result<Vec<(i64, String)>, _>>()?;
            if endpoints.is_empty() {
                return Err(eyre!("No hosts found. Please create one first. You may want to use the `init` command."));
            }

            let account_idx = dialoguer::Select::new()
                .with_prompt("Which ACME account do you want to use?")
                .items(&acme_accounts.iter().map(|(_, name)| name).collect::<Vec<_>>())
                .interact()?;
            let account = acme_accounts.drain(account_idx..).nth(0).unwrap();

            let dns_idx = dialoguer::Select::new()
                .with_prompt("Which DNS provider manages this domain?")
                .items(&dns_providers.iter().map(|(_, name)| name).collect::<Vec<_>>())
                .interact()?;
            let dns_provider = dns_providers.drain(dns_idx..).nth(0).unwrap();

            let endpoint_idx = dialoguer::Select::new()
                .with_prompt("Which host contains the content for this subdomain?")
                .items(&endpoints.iter().map(|(_, name)| name).collect::<Vec<_>>())
                .interact()?;
            let endpoint = endpoints.drain(endpoint_idx..).nth(0).unwrap();

            Ok((account, dns_provider, endpoint))
        })
        .await?;

    Ok(())
}
