use async_trait::async_trait;
use eyre::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::Certificate;

use super::DeployEndpoint;

#[derive(Serialize, Deserialize)]
pub struct DigitalOceanCreds {
    token: String,
}

impl DigitalOceanCreds {
    pub fn from_env() -> Result<DigitalOceanCreds> {
        let token = std::env::var("DIGITAL_OCEAN_TOKEN")?;
        Ok(DigitalOceanCreds { token })
    }

    pub fn from_console() -> Result<DigitalOceanCreds> {
        let token: String = dialoguer::Input::new()
            .with_prompt("DigitalOcean API token (or blank to use $DIGITAL_OCEAN_TOKEN)")
            .allow_empty(true)
            .interact()?;

        if token.is_empty() {
            return DigitalOceanCreds::from_env();
        }

        Ok(DigitalOceanCreds { token })
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
    origin: String,
    endpoint: String,
    ttl: u16,
    certificate_id: String,
    custom_domain: String,
}

#[derive(Deserialize)]
struct DOEndpointResponse {
    endpoint: DOEndpoint,
}

pub struct DigitalOcean {
    creds: DigitalOceanCreds,
    domain: String,
    client: Client,
}

impl DigitalOcean {
    pub fn new(creds: Option<DigitalOceanCreds>, domain: String) -> Result<Self> {
        let creds = match creds {
            Some(creds) => creds,
            None => DigitalOceanCreds::from_env()?,
        };

        Ok(DigitalOcean {
            creds,
            domain,
            client: Client::builder()
                .user_agent("remote-ssl-renewal/0.1.0")
                .build()?,
        })
    }

    /// Upload the certificate and return its ID.
    async fn upload_certificate(&self, cert: Certificate) -> Result<String> {
        let now = time::OffsetDateTime::now_utc();
        let payload = json!({
            "name": format!("{}-{now}", self.domain),
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

    async fn update_endpoint(&self, endpoint: DOEndpoint, cert: Certificate) -> Result<()> {
        let cert_id = self.upload_certificate(cert).await?;

        // Update the endpoint to use the cert
        let payload = json!({
            "certificate_id": cert_id,
        });

        let result = self
            .client
            .put(format!(
                "https://api.digitalocean.com/v2/endpoints/{}",
                endpoint.id
            ))
            .bearer_auth(&self.creds.token)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?;

        // Delete the old cert
        self.client
            .delete(format!(
                "https://api.digitalocean.com/v2/certificates/{}",
                endpoint.certificate_id
            ))
            .bearer_auth(&self.creds.token)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[async_trait]
impl DeployEndpoint for DigitalOcean {
    /// Create the CDN endpoint for the domain and then upload the cert
    async fn create_deployment(&self, cert: Certificate) -> Result<()> {
        todo!();
    }

    /// Find the deployment by the domain, and deploy the new certificate.
    async fn update_deployment(&self, cert: Certificate) -> Result<()> {
        todo!();
    }
}
