pub mod digitalocean;

use async_trait::async_trait;
use eyre::{eyre, Result};
use std::str::FromStr;

use crate::Certificate;

pub enum EndpointProviderType {
    DigitalOcean,
}

impl ToString for EndpointProviderType {
    fn to_string(&self) -> String {
        match self {
            EndpointProviderType::DigitalOcean => "DigitalOcean".to_string(),
        }
    }
}

impl FromStr for EndpointProviderType {
    type Err = eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "DigitalOcean" => Ok(EndpointProviderType::DigitalOcean),
            _ => Err(eyre!("Invalid endpoint provider")),
        }
    }
}

#[async_trait]
pub trait DeployEndpoint {
    async fn create_deployment(&self, cert: Certificate) -> Result<()>;
    async fn update_deployment(&self, cert: Certificate) -> Result<()>;
}
