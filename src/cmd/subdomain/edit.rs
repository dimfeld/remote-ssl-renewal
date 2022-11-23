use std::sync::Arc;

use clap::Args;
use eyre::Result;
use rusqlite::params;

use crate::{cmd::State, db::PoolExtInteract};

#[derive(Debug, Args)]
pub struct EditArgs {
    /// The subdomain to edit
    subdomain: String,
}

pub async fn run(state: Arc<State>, args: EditArgs) -> Result<()> {
    let objects = crate::db::get_all_objects(&state).await?;

    let s = args.subdomain.clone();
    let (acme_account, dns_provider, endpoint) = state
        .pool
        .interact(move |conn| {
            let mut stmt = conn.prepare_cached(
                "SELECT acme_account, dns_provider, endpoint FROM subdomains WHERE name=?",
            )?;

            let result: (i64, i64, i64) =
                stmt.query_row([s], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;

            Ok::<_, eyre::Report>(result)
        })
        .await?;

    let active_account_idx = objects
        .acme_accounts
        .iter()
        .position(|a| a.id == acme_account)
        .unwrap();

    let active_dns_provider_idx = objects
        .dns_providers
        .iter()
        .position(|a| a.id == dns_provider)
        .unwrap();

    let active_endpoint_idx = objects
        .endpoints
        .iter()
        .position(|a| a.id == endpoint)
        .unwrap();

    let new_acme_account_idx = dialoguer::Select::new()
        .items(
            &objects
                .acme_accounts
                .iter()
                .map(|i| i.name.as_str())
                .collect::<Vec<_>>(),
        )
        .with_prompt("Select an ACME account")
        .default(active_account_idx)
        .interact()?;

    let new_dns_provider_idx = dialoguer::Select::new()
        .items(
            &objects
                .dns_providers
                .iter()
                .map(|i| i.name.as_str())
                .collect::<Vec<_>>(),
        )
        .with_prompt("Select a DNS provider")
        .default(active_dns_provider_idx)
        .interact()?;

    let new_endpoint_idx = dialoguer::Select::new()
        .items(
            &objects
                .endpoints
                .iter()
                .map(|i| i.name.as_str())
                .collect::<Vec<_>>(),
        )
        .with_prompt("Select a host")
        .default(active_endpoint_idx)
        .interact()?;

    let new_acme_account_id = objects.acme_accounts[new_acme_account_idx].id;
    let new_dns_provider_id = objects.dns_providers[new_dns_provider_idx].id;
    let new_endpoint_id = objects.endpoints[new_endpoint_idx].id;

    let clear_cert = new_acme_account_idx != active_account_idx;

    state.pool.interact(move |conn| {
        let query = if clear_cert {
            "UPDATE subdomains SET acme_account=?, dns_provider=?, endpoint=?, expires=0 WHERE name=?"
        } else {
            "UPDATE subdomains SET acme_account=?, dns_provider=?, endpoint=? WHERE name=?"
        };

        let mut stmt = conn.prepare_cached(query)?;
        stmt.execute(params![new_acme_account_id, new_dns_provider_id, new_endpoint_id, args.subdomain])?;

        Ok::<_, eyre::Report>(())
    }).await?;

    println!("Done!");

    Ok(())
}
