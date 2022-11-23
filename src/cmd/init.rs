use std::sync::Arc;

use clap::Args;
use eyre::Result;

use super::{acme_account::new_account, dns::new_dns_provider, endpoint::new_endpoint, State};

#[derive(Args, Debug)]
pub struct InitArgs {}

pub async fn run(state: Arc<State>, _args: InitArgs) -> Result<()> {
    new_account(state.clone()).await?;
    new_dns_provider(state.clone()).await?;
    new_endpoint(state.clone()).await?;
    Ok(())
}
