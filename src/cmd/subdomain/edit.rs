use std::sync::Arc;

use clap::Args;
use eyre::Result;

use crate::cmd::State;

#[derive(Debug, Args)]
pub struct EditArgs {
    /// The subdomain to edit
    name: String,
}

pub async fn run(state: Arc<State>, args: EditArgs) -> Result<()> {
    Ok(())
}
