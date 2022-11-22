use std::sync::Arc;

use clap::Args;
use eyre::{eyre, Result};
use serde::Deserialize;

use crate::{db::PoolExtInteract, deploy::EndpointProviderType, dns::DnsProviderType};

use super::State;

#[derive(Args, Debug)]
pub struct NewCertArgs {}

#[derive(Debug)]
struct Config {
    domain: String,
    dns_provider: DnsProviderType,
    deploy_provider: String,
}

pub async fn run(state: Arc<State>, args: NewCertArgs) -> Result<()> {
    let domain: String = dialoguer::Input::new()
        .with_prompt("Which domain do you need a certificate for?")
        .interact()?;

    // See if the domain already exists.
    let exists = state
        .pool
        .interact(move |conn| {
            let mut stmt = conn.prepare_cached("SELECT domain FROM domains WHERE domain = ?")?;
            let exists = stmt.exists(&[domain.as_str()])?;
            Ok::<_, eyre::Report>(exists)
        })
        .await?;

    if exists {
        // What to actually do here?
        return Err(eyre!("This domain already exists"));
    }

    // Disabled until we have more than one provider.
    // let dns_provider_type: String = dialoguer::FuzzySelect::new()
    //     .with_prompt("Which DNS provider do you want to use?")
    //     .items(&["Vercel"])
    //     .interact()?;
    let dns_provider_type = DnsProviderType::Vercel;

    // Disabled until we have more than one deploy endpoint.
    // let deploy_endpoint_type: String = dialoguer::FuzzySelect::new()
    //     .with_prompt("Which DNS provider do you want to use?")
    //     .items(&["Vercel"])
    //     .interact()?;
    let deploy_endpoint_type = EndpointProviderType::DigitalOcean;

    Ok(())
}
