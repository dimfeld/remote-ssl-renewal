use clap::Args;
use eyre::Result;
use serde::Deserialize;

use crate::db::PoolExtInteract;

#[derive(Args, Debug)]
pub struct NewCertArgs {}

#[derive(Debug, Deserialize)]
struct Config {
    domain: String,
    dns_provider: String,
    deploy_provider: String,
}

pub async fn run(pool: deadpool_sqlite::Pool, args: NewCertArgs) -> Result<()> {
    let domain: String = dialoguer::Input::new()
        .with_prompt("Which domain do you need a certificate for?")
        .interact()?;

    // See if the domain already exists.
    let exists = pool
        .interact(move |conn| {
            let mut stmt = conn.prepare_cached("SELECT domain FROM domains WHERE domain = ?")?;
            stmt.exists(&[domain.as_str()])?;
            Ok::<_, eyre::Report>(())
        })
        .await?;

    Ok(())
}
