pub mod vercel;

use serde::Serialize;
use std::{fmt::Display, str::FromStr};
use strum::{Display, EnumIter, EnumString, EnumVariantNames};

use async_trait::async_trait;
use eyre::{eyre, Result};

#[derive(Debug, Display, EnumIter, EnumString, EnumVariantNames)]
pub enum DnsProviderType {
    Vercel,
}

#[async_trait]
pub trait DnsProvider {
    async fn add_challenge_record(&mut self, key: &str, value: &str) -> Result<()>;
    async fn cleanup(&self) -> Result<()>;
}
