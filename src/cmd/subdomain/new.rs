use std::sync::Arc;

use clap::Args;
use eyre::{eyre, Result};
use rusqlite::params;

use crate::{
    cli::get_unique_name,
    db::{DbObjects, PoolExtInteract},
};

use super::{start_cert_process, State};

#[derive(Args, Debug)]
pub struct NewSubdomainArgs {}

pub async fn run(state: Arc<State>, _args: NewSubdomainArgs) -> Result<()> {
    let hider = state.hide_progress();

    let subdomain =
        get_unique_name(&state, "Which subdomain are you adding?", "subdomains").await?;

    let DbObjects {
        mut acme_accounts,
        mut dns_providers,
        mut endpoints,
    } = crate::db::get_all_objects(&state).await?;

    if acme_accounts.is_empty() {
        return Err(eyre!("No ACME accounts found. Please create one first. You may want to use the `init` command."));
    }

    if dns_providers.is_empty() {
        return Err(eyre!("No DNS providers found. Please create one first. You may want to use the `init` command."));
    }

    if endpoints.is_empty() {
        return Err(eyre!(
            "No hosts found. Please create one first. You may want to use the `init` command."
        ));
    }

    let account_idx = dialoguer::Select::new()
        .with_prompt("Which ACME provider do you want to use?")
        .items(&acme_accounts.iter().map(|o| &o.name).collect::<Vec<_>>())
        .default(0)
        .interact()?;
    let account = acme_accounts.drain(account_idx..).next().unwrap();

    let dns_idx = dialoguer::Select::new()
        .with_prompt("Which DNS provider manages this domain?")
        .items(&dns_providers.iter().map(|o| &o.name).collect::<Vec<_>>())
        .default(0)
        .interact()?;
    let dns_provider = dns_providers.drain(dns_idx..).next().unwrap();

    let endpoint_idx = dialoguer::Select::new()
        .with_prompt("Which host contains the content for this subdomain?")
        .items(&endpoints.iter().map(|o| &o.name).collect::<Vec<_>>())
        .default(0)
        .interact()?;
    let endpoint = endpoints.drain(endpoint_idx..).next().unwrap();

    drop(hider);

    let s = subdomain.clone();
    let account_id = account.id;
    let dns_id = dns_provider.id;
    let endpoint_id = endpoint.id;
    state.pool.interact(move |conn| {
            let mut stmt = conn.prepare_cached("INSERT INTO subdomains (name, acme_account, dns_provider, endpoint) VALUES (?, ?, ?, ?)")?;
            stmt.execute(params![s, account_id, dns_id, endpoint_id])?;
            Ok::<_, eyre::Report>(())
        }).await?;

    start_cert_process(
        state,
        super::Renewal {
            subdomain,
            acme_provider: account.provider,
            acme_creds: account.creds,
            dns_provider: dns_provider.provider,
            dns_creds: dns_provider.creds,
            endpoint_provider: endpoint.provider,
            endpoint_creds: endpoint.creds,
        },
    )
    .await?;

    Ok(())
}
