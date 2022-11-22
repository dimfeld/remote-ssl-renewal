use clap::Args;
use eyre::Result;

#[derive(Args, Debug)]
pub struct RenewArgs {}

pub async fn run(args: RenewArgs) -> Result<()> {
    Ok(())
}
