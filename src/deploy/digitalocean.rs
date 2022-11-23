use std::sync::Arc;

use async_trait::async_trait;
use eyre::{eyre, Result};
use indicatif::ProgressBar;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{cmd::State, Certificate};

use super::DeployEndpoint;

#[derive(Serialize, Deserialize)]
pub struct DigitalOceanCreds {
    token: String,
}

impl DigitalOceanCreds {
    pub fn from_string_or_env(creds: String) -> Result<Self> {
        if creds.is_empty() {
            Self::from_env()
        } else {
            let creds: Self = serde_json::from_str(&creds)?;
            Ok(creds)
        }
    }

    pub fn from_env() -> Result<DigitalOceanCreds> {
        let token = std::env::var("DIGITAL_OCEAN_TOKEN")?;
        Ok(DigitalOceanCreds { token })
    }

    pub fn from_console() -> Result<Option<DigitalOceanCreds>> {
        let token: String = dialoguer::Input::new()
            .with_prompt("DigitalOcean API token (or blank to use $DIGITAL_OCEAN_TOKEN)")
            .allow_empty(true)
            .interact()?;

        if token.is_empty() {
            Ok(None)
        } else {
            Ok(Some(DigitalOceanCreds { token }))
        }
    }
}

#[derive(Deserialize)]
struct DOCertificate {
    id: String,
}

#[derive(Deserialize)]
struct DOCertificateResponse {
    certificate: DOCertificate,
}

#[derive(Deserialize)]
struct DOEndpoint {
    id: String,
    // origin: String,
    // endpoint: String,
    // ttl: u16,
    certificate_id: String,
    custom_domain: String,
}

#[derive(Deserialize)]
struct DOEndpointsResponse {
    endpoints: Vec<DOEndpoint>,
}

pub struct DigitalOcean {
    state: Arc<State>,
    creds: DigitalOceanCreds,
    subdomain: String,
    client: Client,
}

impl DigitalOcean {
    pub fn new(state: Arc<State>, creds: DigitalOceanCreds, subdomain: String) -> Result<Self> {
        Ok(DigitalOcean {
            state,
            creds,
            subdomain,
            client: Client::builder().user_agent(crate::USER_AGENT).build()?,
        })
    }

    /// Upload the certificate and return its ID.
    async fn upload_certificate(&self, cert: Certificate) -> Result<String> {
        let now = time::OffsetDateTime::now_utc();
        let payload = json!({
            "name": format!("{}-{now}", self.subdomain),
            "private_key": cert.key,
            "leaf_certificate": cert.cert,
            "certificate_chain": cert.cert,
        });

        let response = self
            .client
            .post("https://api.digitalocean.com/v2/certificates")
            .bearer_auth(&self.creds.token)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?
            .json::<DOCertificateResponse>()
            .await?;

        Ok(response.certificate.id)
    }

    async fn set_endpoint_cert(&self, endpoint: &DOEndpoint, cert_id: &str) -> Result<()> {
        // Update the endpoint to use the cert
        let payload = json!({
            "certificate_id": cert_id,
        });

        self.client
            .put(format!(
                "https://api.digitalocean.com/v2/endpoints/{}",
                endpoint.id
            ))
            .bearer_auth(&self.creds.token)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    async fn remove_cert(&self, cert_id: &str) -> Result<()> {
        // Delete the old cert
        self.client
            .delete(format!(
                "https://api.digitalocean.com/v2/certificates/{cert_id}",
            ))
            .bearer_auth(&self.creds.token)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    async fn create_endpoint(&self, cert_id: &str) -> Result<()> {
        let origin: String = {
            let _hider = self.state.hide_progress();
            dialoguer::Input::new()
                .with_prompt("Enter the Spaces origin FQDN for this endpoint")
                .interact_text()?
        };

        let payload = json!({
            "origin": origin,
            "certificate_id": cert_id,
            "custom_domain": &self.subdomain,
        });

        let progress = self
            .state
            .progress
            .add(ProgressBar::new_spinner().with_message("Creating endpoint"));

        self.client
            .post("https://api.digitalocean.com/v2/endpoints")
            .bearer_auth(&self.creds.token)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?;

        progress.finish_with_message("Done");

        Ok(())
    }

    async fn find_existing_endpoint(&self) -> Result<Option<DOEndpoint>> {
        let mut page = 0;
        loop {
            let result = self
                .client
                .get(format!(
                    "https://api.digitalocean.com/v2/endpoints?page={page}&per_page=200",
                ))
                .bearer_auth(&self.creds.token)
                .send()
                .await?
                .error_for_status()?
                .json::<DOEndpointsResponse>()
                .await?;

            if result.endpoints.is_empty() {
                return Ok(None);
            }

            let found = result
                .endpoints
                .into_iter()
                .find(|endpoint| endpoint.custom_domain == self.subdomain);

            if found.is_some() {
                return Ok(found);
            }

            page += 1;
        }
    }
}

#[async_trait]
impl DeployEndpoint for DigitalOcean {
    /// Find the deployment by the subdomain, and deploy the new certificate. If the deployment
    /// does not yet exist, prompt for the relevant information.
    async fn deploy_certificate(&self, cert: Certificate, endpoint_must_exist: bool) -> Result<()> {
        let cert_id = self.upload_certificate(cert).await?;

        if let Some(endpoint) = self.find_existing_endpoint().await? {
            let progress = self
                .state
                .progress
                .add(ProgressBar::new_spinner().with_message("Uploading certificate"));
            self.set_endpoint_cert(&endpoint, &cert_id).await?;

            progress.set_message("Removing old certificate");
            self.remove_cert(&endpoint.certificate_id).await?;

            progress.finish_with_message("Done");
        } else if endpoint_must_exist {
            return Err(eyre!("CDN Endpoint for {} does not exist", self.subdomain));
        } else {
            self.create_endpoint(&cert_id).await?;
        }

        Ok(())
    }
}
