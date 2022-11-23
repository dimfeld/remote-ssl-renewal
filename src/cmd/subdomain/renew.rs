use std::sync::Arc;

use clap::Args;
use eyre::Result;
use time::OffsetDateTime;

use crate::cmd::State;

#[derive(Debug, Args)]
pub struct RenewArgs {
    /// A specific subdomain to renew
    subdomain: Option<String>,

    /// Renew the subdomain's certificate even if it's far from expiration. This is only honored
    /// when the `subdomain` option is provided.
    ///
    /// Use this with care, as providers like LetsEncrypt have very strict rate limits on how
    /// often you can renew your certificate.
    #[clap(long, default_value_t = false)]
    force: bool,
}

async fn renew_any_needed(state: Arc<State>) {
    let threshold = (OffsetDateTime::now_utc() + time::Duration::days(14)).unix_timestamp();
    // Find subdomains that are expiring within the threshold.
    // Renew them all
}

pub async fn run(state: Arc<State>, args: RenewArgs) -> Result<()> {
    if let Some(subdomain) = args.subdomain {
    } else {
    }

    Ok(())
}
