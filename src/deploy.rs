pub mod digitalocean;

use async_trait::async_trait;
use eyre::Result;
use std::sync::Arc;
use strum::{Display, EnumIter, EnumString, EnumVariantNames};

use crate::{cmd::State, Certificate};

use self::digitalocean::DigitalOceanCreds;

#[derive(Debug, Display, EnumIter, EnumString, EnumVariantNames)]
pub enum EndpointProviderType {
    DigitalOcean,
}

#[async_trait]
pub trait DeployEndpoint {
    async fn deploy_certificate(&self, cert: Certificate, endpoint_must_exist: bool) -> Result<()>;
}

pub fn create_deployer(
    state: Arc<State>,
    deployer_type: EndpointProviderType,
    subdomain: String,
    creds: String,
) -> Result<Box<dyn DeployEndpoint>> {
    let deployer = match deployer_type {
        EndpointProviderType::DigitalOcean => {
            let creds = DigitalOceanCreds::from_string_or_env(creds)?;
            Box::new(digitalocean::DigitalOcean::new(state, creds, subdomain)?)
        }
    };

    Ok(deployer)
}
