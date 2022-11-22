use async_trait::async_trait;
use eyre::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::DnsProvider;

#[derive(Serialize, Deserialize)]
pub struct VercelDnsCreds {
    token: String,
}

impl VercelDnsCreds {
    pub fn from_env() -> Result<VercelDnsCreds> {
        let token = std::env::var("VERCEL_TOKEN")?;
        Ok(VercelDnsCreds { token })
    }

    pub fn from_console() -> Result<Option<VercelDnsCreds>> {
        let token: String = dialoguer::Input::new()
            .with_prompt("Vercel API token (or blank to use $VERCEL_TOKEN)")
            .allow_empty(true)
            .interact_text()?;

        if token.is_empty() {
            Ok(None)
        } else {
            Ok(Some(VercelDnsCreds { token }))
        }
    }
}

pub struct VercelDns {
    creds: VercelDnsCreds,
    domain: String,
    client: reqwest::Client,
    record_id: Option<String>,
}

#[derive(Deserialize)]
struct AddRecordResponse {
    uid: String,
}

impl VercelDns {
    pub fn new(creds: Option<VercelDnsCreds>, domain: String) -> Result<VercelDns> {
        let creds = match creds {
            Some(creds) => creds,
            None => VercelDnsCreds::from_env()?,
        };

        Ok(VercelDns {
            creds,
            domain,
            client: Client::builder()
                .user_agent("remote-ssl-renewal/0.1.0")
                .build()?,
            record_id: None,
        })
    }
}

#[async_trait]
impl DnsProvider for VercelDns {
    async fn add_challenge_record(&mut self, key: &str, value: &str) -> Result<()> {
        let url = format!("https://api.vercel.com/v2/domains/{}/records", self.domain);
        let body = serde_json::json!({
            "name": key,
            "type": "TXT",
            "value": value,
        });

        let res = self
            .client
            .post(&url)
            .bearer_auth(&self.creds.token)
            .json(&body)
            .send()
            .await?;
        if !res.status().is_success() {
            return Err(eyre::eyre!(
                "Failed to add challenge record: {}",
                res.text().await?
            ));
        }

        let response: AddRecordResponse = res.json().await?;

        self.record_id = Some(response.uid);

        Ok(())
    }

    async fn cleanup(&self) -> Result<()> {
        let record_id = match self.record_id.as_ref() {
            Some(r) => r,
            None => return Ok(()),
        };

        let url = format!(
            "https://api.vercel.com/v2/domains/{}/records/{}",
            self.domain, record_id
        );

        let res = self
            .client
            .delete(&url)
            .bearer_auth(&self.creds.token)
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(eyre::eyre!(
                "Failed to delete challenge record: {}",
                res.text().await?
            ));
        }

        Ok(())
    }
}
