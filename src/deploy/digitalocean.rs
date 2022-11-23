use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use eyre::{eyre, Result};
use indicatif::ProgressBar;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use time::macros::format_description;

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
    name: String,
}

#[derive(Deserialize)]
struct DOCertificateResponse {
    certificate: DOCertificate,
}

#[derive(Deserialize)]
struct DOCertificatesResponse {
    certificates: Vec<DOCertificate>,
}

#[derive(Debug, Deserialize)]
struct DOEndpoint {
    id: String,
    // origin: String,
    // endpoint: String,
    ttl: u16,
    certificate_id: String,
    custom_domain: String,
}

#[derive(Deserialize)]
struct DOEndpointResponse {
    endpoint: DOEndpoint,
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
        let formatter = format_description!("[year]-[month]-[day]-[hour]-[minute]-[second]");
        let payload = json!({
            "name": format!("{}-{}", self.subdomain, now.format(&formatter)?),
            "type": "custom",
            "private_key": cert.key,
            "leaf_certificate": cert.get_leaf_certificate(),
            // DO doesn't like the double newline.
            "certificate_chain": cert.cert.replace("\n\n", "\n"),
        });

        let response = self
            .client
            .post("https://api.digitalocean.com/v2/certificates")
            .bearer_auth(&self.creds.token)
            .json(&payload)
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            let result = response.json::<DOCertificateResponse>().await?;
            Ok(result.certificate.id)
        } else if status == StatusCode::UNPROCESSABLE_ENTITY {
            // DO doesn't let you upload a cert that it already knows about, and this can happen if
            // a command failed halfway through an you're trying to force a reinstall. Handle this
            // case here.
            let body = response.text().await?;
            let same_sha =
                regex::Regex::new(r"found certificate (.*) with the same SHA-1 fingerprint")?;
            if let Some(captures) = same_sha.captures(&body) {
                let name = captures.get(1).unwrap().as_str();
                self.get_certificate_by_name(name).await?.ok_or_else(|| {
                    eyre!(
                        "Certificate with name {} already exists, but we couldn't find it",
                        name
                    )
                })
            } else {
                Err(eyre!("Failed to upload certificate: {status} {body}"))
            }
        } else {
            let body = response.text().await?;
            Err(eyre!("failed to upload certificate: {status} {body}"))
        }
    }

    async fn get_certificate_by_name(&self, name: &str) -> Result<Option<String>> {
        let mut page = 1;
        loop {
            let certs = self
                .client
                .get("https://api.digitalocean.com/v2/certificates")
                .query(&[("page", page)])
                .bearer_auth(&self.creds.token)
                .send()
                .await?
                .json::<DOCertificatesResponse>()
                .await?;

            if certs.certificates.is_empty() {
                return Ok(None);
            }

            let found_cert = certs
                .certificates
                .into_iter()
                .find(|cert| cert.name == name);

            if let Some(cert) = found_cert {
                return Ok(Some(cert.id));
            }

            page += 1;
        }
    }

    async fn set_endpoint_cert(&self, endpoint: &DOEndpoint, cert_id: &str) -> Result<()> {
        // Update the endpoint to use the cert. Although we're only changing the certificate,
        // we have to pass all the other fields too or it silently fails.
        let payload = json!({
            "certificate_id": cert_id,
            "custom_domain": endpoint.custom_domain,
            "ttl": endpoint.ttl
        });

        self.client
            .put(format!(
                "https://api.digitalocean.com/v2/cdn/endpoints/{}",
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
                .with_prompt("Enter the Spaces origin FQDN for this endpoint. This can be found in the Digital Ocean Spaces configuration")
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
        progress.enable_steady_tick(Duration::from_millis(125));

        self.client
            .post("https://api.digitalocean.com/v2/cdn/endpoints")
            .bearer_auth(&self.creds.token)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?;

        progress.finish_with_message("Done");

        Ok(())
    }

    async fn find_existing_endpoint(&self) -> Result<Option<DOEndpoint>> {
        let mut page = 1;
        loop {
            let result = self
                .client
                .get(format!(
                    "https://api.digitalocean.com/v2/cdn/endpoints?page={page}&per_page=200",
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
            progress.enable_steady_tick(Duration::from_millis(125));

            if endpoint.certificate_id != cert_id {
                self.set_endpoint_cert(&endpoint, &cert_id).await?;

                progress.set_message("Removing old certificate");
                self.remove_cert(&endpoint.certificate_id).await?;
            }

            progress.finish_with_message("Done");
        } else if endpoint_must_exist {
            return Err(eyre!("CDN Endpoint for {} does not exist", self.subdomain));
        } else {
            self.create_endpoint(&cert_id).await?;
        }

        Ok(())
    }
}
