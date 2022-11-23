mod edit;
mod new;
mod reinstall_cert;
mod renew;

use std::{str::FromStr, sync::Arc};

use clap::{Args, Subcommand};
use eyre::Result;
use rusqlite::params;

use crate::{db::PoolExtInteract, deploy::EndpointProviderType, dns::DnsProviderType};

use super::State;

#[derive(Args, Debug)]
pub struct SubdomainArgs {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Add a new subdomain
    New(new::NewSubdomainArgs),
    /// Renew subdomains as needed
    Renew(renew::RenewArgs),
    /// Edit the settings for a subdomain
    Edit(edit::EditArgs),
    /// Reinstall the certificate that has already been issued
    ///
    /// This requires that the certificate is cached in the local database.
    ReinstallCert(reinstall_cert::ReinstallCertArgs),
}

pub async fn run(state: Arc<State>, args: SubdomainArgs) -> Result<()> {
    match args.command {
        Commands::New(args) => new::run(state, args).await?,
        Commands::Renew(args) => renew::run(state, args).await?,
        Commands::Edit(args) => edit::run(state, args).await?,
        Commands::ReinstallCert(args) => reinstall_cert::run(state, args).await?,
    };

    Ok(())
}

pub struct DbObject {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub creds: String,
}

impl DbObject {
    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            provider: row.get(2)?,
            creds: row.get(3)?,
        })
    }
}

async fn start_cert_process(
    state: Arc<State>,
    subdomain: String,
    account_obj: DbObject,
    dns_provider: DbObject,
    endpoint: DbObject,
) -> Result<()> {
    let dns_provider_type = DnsProviderType::from_str(&dns_provider.provider)?;
    let dns_provider =
        crate::dns::get_dns_provider(dns_provider_type, &subdomain, dns_provider.creds)?;

    let acme_creds = serde_json::from_str::<instant_acme::AccountCredentials>(&account_obj.creds)?;
    let account = instant_acme::Account::from_credentials(acme_creds)?;

    let deployer_type = EndpointProviderType::from_str(&endpoint.provider)?;
    let deployer = crate::deploy::create_deployer(
        state.clone(),
        deployer_type,
        subdomain.clone(),
        endpoint.creds,
    )?;

    let (cert, expires) =
        crate::acme::get_certificate(state.clone(), dns_provider, account, subdomain.clone())
            .await?;

    let saved_cert = serde_json::to_string(&cert)?;

    state
        .pool
        .interact(move |conn| {
            let mut stmt = conn
                .prepare_cached(r##"UPDATE subdomains SET last_cert=?, expires=? WHERE name=?"##)?;

            stmt.execute(params![saved_cert, expires, subdomain])?;

            Ok::<_, eyre::Report>(())
        })
        .await?;

    deployer.deploy_certificate(cert, false).await?;
    Ok(())
}
