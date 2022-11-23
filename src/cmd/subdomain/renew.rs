use std::sync::Arc;

use clap::Args;
use eyre::{eyre, Result};
use time::OffsetDateTime;

use crate::{cmd::State, db::PoolExtInteract};

use super::{start_cert_process, Renewal};

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

fn renewal_threshold() -> i64 {
    (OffsetDateTime::now_utc() + time::Duration::days(14)).unix_timestamp()
}

async fn renew_any_needed(state: Arc<State>) -> Result<()> {
    let renewals = state
        .pool
        .interact(move |conn| {
            let mut stmt = conn.prepare_cached(
                r##"
            SELECT sd.name,
                aa.provider as acme_provider,
                aa.creds as acme_creds,
                dp.provider as dns_provider,
                dp.creds as dns_creds,
                ep.provider as endpoint_provider,
                ep.creds as endpoint_creds
            FROM subdomains sd
            JOIN acme_accounts aa ON aa.id=sd.acme_account
            JOIN dns_providers dp ON dp.id=sd.dns_provider
            JOIN endpoints ep ON ep.id=sd.endpoint
            WHERE sd.enabled AND sd.last_cert IS NOT NULL AND sd.expires < ?
        "##,
            )?;

            let threshold = renewal_threshold();
            let results = stmt
                .query_map([threshold], |row| {
                    Ok(Renewal {
                        subdomain: row.get(0)?,
                        acme_provider: row.get(1)?,
                        acme_creds: row.get(2)?,
                        dns_provider: row.get(3)?,
                        dns_creds: row.get(4)?,
                        endpoint_provider: row.get(5)?,
                        endpoint_creds: row.get(6)?,
                    })
                })?
                .collect::<Result<Vec<_>, rusqlite::Error>>()?;

            Ok::<_, eyre::Report>(results)
        })
        .await?;

    let mut errored = false;
    if renewals.is_empty() {
        println!("All certificates up to date");
    } else {
        let tasks = renewals
            .into_iter()
            .map(|r| {
                let state = state.clone();
                tokio::task::spawn(start_cert_process(state, r))
            })
            .collect::<Vec<_>>();

        let results = futures::future::join_all(tasks).await;
        for r in results {
            if let Err(e) = r {
                eprintln!("Error renewing certificate: {}", e);
                errored = true;
            }
        }
    }

    if errored {
        Err(eyre!("Encountered errors"))
    } else {
        Ok(())
    }
}

async fn renew_one_cmd(state: Arc<State>, subdomain: String, force: bool) -> Result<()> {
    let (renewal, expires) = state
        .pool
        .interact(move |conn| {
            let mut stmt = conn.prepare_cached(
                r##"
            SELECT aa.provider as acme_provider,
                aa.creds as acme_creds,
                dp.provider as dns_provider,
                dp.creds as dns_creds,
                ep.provider as endpoint_provider,
                ep.creds as endpoint_creds,
                sd.expires
            FROM subdomains sd
            JOIN acme_accounts aa ON aa.id=sd.acme_account
            JOIN dns_providers dp ON dp.id=sd.dns_provider
            JOIN endpoints ep ON ep.id=sd.endpoint
            WHERE sd.name = ?
        "##,
            )?;

            let renewal: (Renewal, Option<i64>) = stmt.query_row([subdomain.clone()], |row| {
                Ok((
                    Renewal {
                        subdomain,
                        acme_provider: row.get(0)?,
                        acme_creds: row.get(1)?,
                        dns_provider: row.get(2)?,
                        dns_creds: row.get(3)?,
                        endpoint_provider: row.get(4)?,
                        endpoint_creds: row.get(5)?,
                    },
                    row.get(6)?,
                ))
            })?;

            Ok::<_, eyre::Report>(renewal)
        })
        .await?;

    if expires.unwrap_or(0) < renewal_threshold() || force {
        start_cert_process(state, renewal).await?;
    } else {
        println!("Certificate is not due for renewal yet");
    }

    Ok(())
}

pub async fn run(state: Arc<State>, args: RenewArgs) -> Result<()> {
    if let Some(subdomain) = args.subdomain {
        renew_one_cmd(state, subdomain, args.force).await
    } else {
        renew_any_needed(state).await
    }
}
