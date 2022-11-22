use clap::Args;
use eyre::Result;
use serde::Deserialize;

#[derive(Args, Debug)]
pub struct NewCertArgs {
    config: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Config {}

pub async fn run(args: NewCertArgs) -> Result<()> {
    Ok(())
}
