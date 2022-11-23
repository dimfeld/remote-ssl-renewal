use std::sync::Arc;

use clap::Args;
use eyre::{eyre, Result};

use crate::{cli::get_unique_name, db::PoolExtInteract};

use super::{start_cert_process, DbObject, State};

#[derive(Args, Debug)]
pub struct NewSubdomainArgs {}

pub async fn run(state: Arc<State>, _args: NewSubdomainArgs) -> Result<()> {
    let hider = state.hide_progress();

    let subdomain =
        get_unique_name(&state, "Which subdomain are you adding?", "subdomains").await?;

    let s = subdomain.clone();
    let (account, dns_provider, endpoint) = state
        .pool
        .interact(move |conn| {
            let mut stmt =
                conn.prepare_cached("SELECT id, name, provider, creds FROM acme_accounts ORDER BY name")?;
            let mut acme_accounts = stmt
                .query_map([], DbObject::from_row)?
                .collect::<Result<Vec<DbObject>, _>>()?;

            if acme_accounts.is_empty() {
                return Err(eyre!("No ACME accounts found. Please create one first. You may want to use the `init` command."));
            }

            let mut stmt =
                conn.prepare_cached("SELECT id, name, provider, creds FROM dns_providers ORDER BY name")?;
            let mut dns_providers = stmt
                .query_map([], DbObject::from_row)?
                .collect::<Result<Vec<DbObject>, _>>()?;
            if dns_providers.is_empty() {
                return Err(eyre!("No DNS providers found. Please create one first. You may want to use the `init` command."));
            }

            let mut stmt = conn.prepare_cached("SELECT id, name, provider, creds FROM endpoints ORDER BY name")?;
            let mut endpoints = stmt
                .query_map([], DbObject::from_row)?
                .collect::<Result<Vec<DbObject>, _>>()?;
            if endpoints.is_empty() {
                return Err(eyre!("No hosts found. Please create one first. You may want to use the `init` command."));
            }

            let account_idx = dialoguer::Select::new()
                .with_prompt("Which ACME account do you want to use?")
                .items(&acme_accounts.iter().map(|o| &o.name).collect::<Vec<_>>())
                .interact()?;
            let account = acme_accounts.drain(account_idx..).next().unwrap();

            let dns_idx = dialoguer::Select::new()
                .with_prompt("Which DNS provider manages this domain?")
                .items(&dns_providers.iter().map(|o| &o.name).collect::<Vec<_>>())
                .interact()?;
            let dns_provider = dns_providers.drain(dns_idx..).next().unwrap();

            let endpoint_idx = dialoguer::Select::new()
                .with_prompt("Which host contains the content for this subdomain?")
                .items(&endpoints.iter().map(|o| &o.name).collect::<Vec<_>>())
                .interact()?;
            let endpoint = endpoints.drain(endpoint_idx..).next().unwrap();

            let mut stmt = conn.prepare_cached("INSERT INTO subdomains (subdomain, acme_account, dns_provider, endpoint) VALUES (?, ?, ?, ?)")?;
            stmt.execute([&s, &account.id, &dns_provider.id, &endpoint.id])?;

            Ok((account, dns_provider, endpoint))
        })
        .await?;

    drop(hider);
    start_cert_process(state, subdomain, account, dns_provider, endpoint).await?;

    Ok(())
}
