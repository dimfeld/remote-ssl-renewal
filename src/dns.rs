pub mod vercel;

use std::str::FromStr;

use async_trait::async_trait;
use eyre::{eyre, Result};

pub enum DnsProviderType {
    Vercel,
}

impl ToString for DnsProviderType {
    fn to_string(&self) -> String {
        match self {
            DnsProviderType::Vercel => "Vercel".to_string(),
        }
    }
}

impl FromStr for DnsProviderType {
    type Err = eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Vercel" => Ok(DnsProviderType::Vercel),
            _ => Err(eyre!("Invalid DNS provider")),
        }
    }
}

#[async_trait]
pub trait DnsProvider {
    async fn add_challenge_record(&mut self, key: &str, value: &str) -> Result<()>;
    async fn cleanup(&self) -> Result<()>;
}
