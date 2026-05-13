mod acme;
mod cli;
mod cmd;
mod db;
mod deploy;
mod dns;
mod tracing_config;

use eyre::Result;
use serde::{Deserialize, Serialize};

pub const USER_AGENT: &str = concat!("remote-ssl-renewal/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Serialize, Deserialize)]
pub struct Certificate {
    pub cert: String,
    pub key: String,
}

impl Certificate {
    pub fn get_leaf_certificate(&self) -> &str {
        self.cert
            .split_inclusive("-----END CERTIFICATE-----\n")
            .next()
            .unwrap()
    }

    pub fn get_certificate_chain(&self) -> &str {
        self.cert
            .split_once("-----END CERTIFICATE-----\n")
            .map(|(_, chain)| chain)
            .unwrap_or("")
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_config::init_tracing();

    let db = db::create_db().await?;

    cmd::run(db).await?;
    Ok(())
}
