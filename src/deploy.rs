pub mod digitalocean;

use async_trait::async_trait;
use eyre::{eyre, Result};
use std::str::FromStr;
use strum::{Display, EnumIter, EnumString, EnumVariantNames};

use crate::Certificate;

#[derive(Debug, Display, EnumIter, EnumString, EnumVariantNames)]
pub enum EndpointProviderType {
    DigitalOcean,
}

#[async_trait]
pub trait DeployEndpoint {
    async fn create_deployment(&self, cert: Certificate) -> Result<()>;
    async fn update_deployment(&self, cert: Certificate) -> Result<()>;
}
