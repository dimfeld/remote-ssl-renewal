pub mod vercel;

use strum::{Display, EnumIter, EnumString, EnumVariantNames};

use async_trait::async_trait;
use eyre::{eyre, Result};

use self::vercel::VercelDnsCreds;

#[derive(Debug, Display, EnumIter, EnumString, EnumVariantNames)]
pub enum DnsProviderType {
    Vercel,
}

#[async_trait]
pub trait DnsProvider: Send + Sync {
    async fn add_challenge_record(&mut self, key: &str, value: &str) -> Result<()>;
    async fn cleanup(&self) -> Result<()>;
}

pub fn get_dns_provider(
    provider_type: DnsProviderType,
    subdomain: &str,
    creds: String,
) -> Result<Box<dyn DnsProvider>> {
    let components = subdomain.rsplitn(3, '.').collect::<Vec<_>>();

    if components.len() < 2 {
        return Err(eyre!("Invalid domain {subdomain}"));
    }

    let tld = components[0];
    let domain_name = components[1];
    let domain = format!("{domain_name}.{tld}");

    let provider = match provider_type {
        DnsProviderType::Vercel => {
            let creds = VercelDnsCreds::from_string_or_env(creds)?;
            Box::new(vercel::VercelDns::new(creds, domain)?)
        }
    };

    Ok(provider)
}
