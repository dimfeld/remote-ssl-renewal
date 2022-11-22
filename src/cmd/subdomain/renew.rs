use std::sync::Arc;

use clap::Args;
use eyre::Result;

use crate::cmd::State;

#[derive(Debug, Args)]
pub struct RenewArgs {
    /// A specific subdomain to renew
    name: Option<String>,

    /// Renew the subdomain's certificate even if it's far from expiration.
    ///
    /// Use this with care, as providers like LetsEncrypt have very strict rate limits on how
    /// often you can renew your certificate.
    #[clap(long, default_value_t = false)]
    force: bool,
}

pub async fn run(state: Arc<State>, args: RenewArgs) -> Result<()> {
    Ok(())
}
