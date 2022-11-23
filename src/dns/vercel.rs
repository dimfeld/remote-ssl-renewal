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
    pub fn from_string_or_env(creds: String) -> Result<VercelDnsCreds> {
        if creds.is_empty() {
            Self::from_env()
        } else {
            let creds: Self = serde_json::from_str(&creds)?;
            Ok(creds)
        }
    }

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
}

#[derive(Deserialize)]
struct AddRecordResponse {
    uid: String,
}

impl VercelDns {
    pub fn new(creds: VercelDnsCreds, domain: String) -> Result<VercelDns> {
        Ok(VercelDns {
            creds,
            domain,
            client: Client::builder().user_agent(crate::USER_AGENT).build()?,
        })
    }
}

#[async_trait]
impl DnsProvider for VercelDns {
    async fn add_challenge_record(&self, key: &str, value: &str) -> Result<String> {
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

        Ok(response.uid)
    }

    async fn cleanup(&self, record_id: &str) -> Result<()> {
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
