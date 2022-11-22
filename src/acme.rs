use instant_acme::LetsEncrypt;
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumIter, EnumString, EnumVariantNames};

#[derive(
    AsRefStr, Debug, Display, EnumIter, EnumString, EnumVariantNames, Serialize, Deserialize,
)]
pub enum AcmeProvider {
    LetsEncrypt,
    LetsEncryptStaging,
}

impl AcmeProvider {
    pub fn label(&self) -> &'static str {
        match self {
            AcmeProvider::LetsEncrypt => "Let's Encrypt",
            AcmeProvider::LetsEncryptStaging => "Let's Encrypt Staging Environment",
        }
    }

    pub fn url(&self) -> &'static str {
        match self {
            AcmeProvider::LetsEncrypt => LetsEncrypt::Production.url(),
            AcmeProvider::LetsEncryptStaging => LetsEncrypt::Staging.url(),
        }
    }
}
