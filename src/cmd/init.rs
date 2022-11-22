use std::sync::Arc;

use clap::Args;
use eyre::{eyre, Result};
use serde::Deserialize;

use crate::{db::PoolExtInteract, deploy::EndpointProviderType, dns::DnsProviderType};

use super::State;

#[derive(Args, Debug)]
pub struct InitArgs {}

pub async fn run(state: Arc<State>, args: InitArgs) -> Result<()> {
    todo!();
}
