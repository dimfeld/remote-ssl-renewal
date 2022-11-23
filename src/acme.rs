use std::{sync::Arc, time::Duration};

use eyre::{eyre, Result};
use indicatif::ProgressBar;
use instant_acme::{
    AuthorizationStatus, ChallengeType, Identifier, LetsEncrypt, NewOrder, Order, OrderState,
    OrderStatus,
};
use rcgen::{CertificateParams, DistinguishedName};
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumIter, EnumString, EnumVariantNames};
use tracing::{event, Level};

use crate::{cmd::State, dns::DnsProvider, Certificate};

#[derive(
    AsRefStr, Debug, Display, EnumIter, EnumString, EnumVariantNames, Serialize, Deserialize,
)]
pub enum AcmeProvider {
    LetsEncrypt,
    LetsEncryptStaging,
}

impl AcmeProvider {
    pub fn label(&self) -> &'static str {
        match self {
            AcmeProvider::LetsEncrypt => "Let's Encrypt",
            AcmeProvider::LetsEncryptStaging => "Let's Encrypt Staging Environment",
        }
    }

    pub fn url(&self) -> &'static str {
        match self {
            AcmeProvider::LetsEncrypt => LetsEncrypt::Production.url(),
            AcmeProvider::LetsEncryptStaging => LetsEncrypt::Staging.url(),
        }
    }
}

pub async fn get_certificate(
    state: Arc<State>,
    mut dns_provider: Box<dyn DnsProvider>,
    acme_account: instant_acme::Account,
    subdomain: String,
) -> Result<(Certificate, i64)> {
    let progress = state.progress.add(
        ProgressBar::new_spinner()
            .with_prefix(subdomain.clone())
            .with_message("Starting certificate process..."),
    );

    let identifiers = vec![instant_acme::Identifier::Dns(subdomain)];

    let (mut order, state) = acme_account
        .new_order(&NewOrder {
            identifiers: &identifiers,
        })
        .await?;

    let authorizations = order.authorizations(&state.authorizations).await?;
    let mut challenges = Vec::with_capacity(authorizations.len());

    // Get all the challenges first.
    for authz in &authorizations {
        match authz.status {
            AuthorizationStatus::Pending => {}
            _ => continue,
        }

        let challenge = authz
            .challenges
            .iter()
            .find(|c| c.r#type == ChallengeType::Dns01)
            .ok_or_else(|| eyre!("No DNS-01 challenge found for {authz:?}"))?;

        let Identifier::Dns(identifier) = &authz.identifier;

        let response_value = order.key_authorization(challenge).dns_value();

        challenges.push((identifier, &challenge.url, response_value));
    }

    // We currently only support one identifier at a time. Future implementations should put each
    // of the challenges into a new task to process them concurrently, and use a MultiProgressBar
    // to track them on the console.
    for (identifier, challenge_url, response) in &challenges {
        progress.set_message("Creating DNS record");
        let key = format!("_acme_challenge.{identifier}");
        dns_provider.add_challenge_record(&key, response).await?;

        // TODO Do we need to wait for the propagation here?

        order.set_challenge_ready(challenge_url).await?;

        progress.set_message("Waiting for challenge to be verified");
        let max_tries = 10;
        let mut tries = 0;
        let mut delay = Duration::from_millis(250);
        let result = loop {
            tokio::time::sleep(delay).await;
            match check_order_result(&mut order).await {
                Ok(Some(state)) => break Ok(state),
                Ok(None) => {}
                Err(e) => {
                    event!(Level::ERROR, "Error checking order status: {}", e);
                }
            }

            delay *= 2;
            delay = std::cmp::min(delay, Duration::from_secs(60));
            tries += 1;
            if tries >= max_tries {
                break Err(eyre!("Failed to verify challenge after {} tries", tries));
            }
        };

        dns_provider.cleanup().await?;

        match result {
            Ok(state) => match state.status {
                OrderStatus::Ready => {}
                _ => {
                    progress.set_message("Challenge failed");
                    return Err(eyre!("Challenge failed: {:?}", state.error));
                }
            },
            Err(e) => {
                progress.set_message("Failed to verify challenge");
                return Err(e);
            }
        };
    }

    progress.set_message("Requesting certificate");
    let identifiers = challenges.iter().map(|c| c.0.clone()).collect::<Vec<_>>();
    let mut params = CertificateParams::new(identifiers);
    params.distinguished_name = DistinguishedName::new();
    let cert = rcgen::Certificate::from_params(params).unwrap();
    let csr = cert.serialize_request_der()?;
    let cert_chain_pem = order.finalize(&csr, &state.finalize).await?;

    let first_cert_container = x509_parser::pem::Pem::iter_from_buffer(cert_chain_pem.as_bytes())
        .next()
        .ok_or_else(|| eyre!("Certificate was empty"))??;
    let first_cert = first_cert_container.parse_x509()?;
    let expires = first_cert.tbs_certificate.validity.not_after.timestamp();

    progress.finish_with_message("Certificate obtained");

    Ok((
        Certificate {
            cert: cert_chain_pem,
            key: cert.serialize_private_key_pem(),
        },
        expires,
    ))
}

async fn check_order_result(order: &mut Order) -> Result<Option<OrderState>> {
    let state = order.state().await?;
    match state.status {
        OrderStatus::Ready | OrderStatus::Valid | OrderStatus::Invalid => Ok(Some(state)),
        OrderStatus::Pending | OrderStatus::Processing => Ok(None),
    }
}
