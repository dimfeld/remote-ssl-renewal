use std::{str::FromStr, sync::Arc};

use clap::Args;
use eyre::{eyre, Result};
use indicatif::ProgressBar;

use crate::{db::PoolExtInteract, deploy::EndpointProviderType, Certificate};

use super::State;

#[derive(Args, Debug)]
pub struct ReinstallCertArgs {
    /// The subdomain to reinstall the certificate for
    subdomain: String,
}

pub async fn run(state: Arc<State>, args: ReinstallCertArgs) -> Result<()> {
    let s = args.subdomain.clone();
    let (last_cert, provider, creds): (Option<String>, String, String) = state
        .pool
        .interact(move |conn| {
            let mut stmt = conn.prepare_cached(
                r##"SELECT last_cert, provider, creds
            FROM subdomains sd
            JOIN endpoints ep ON ep.id=sd.endpoint
            WHERE sd.name=?"##,
            )?;

            let (last_cert, provider, creds) =
                stmt.query_row([s], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;

            Ok::<_, eyre::Report>((last_cert, provider, creds))
        })
        .await?;

    let last_cert =
        last_cert.ok_or_else(|| eyre!("This subdomain does not yet have a certificate"))?;
    let last_cert: Certificate = serde_json::from_str(&last_cert)?;

    let deployer_type = EndpointProviderType::from_str(&provider)?;
    let deployer =
        crate::deploy::create_deployer(state.clone(), deployer_type, args.subdomain, creds)?;

    deployer.deploy_certificate(last_cert, false).await?;

    Ok(())
}
