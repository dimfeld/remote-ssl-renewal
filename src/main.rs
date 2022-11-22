mod cmd;
mod db;
mod deploy;
mod dns;
mod tracing_config;

use eyre::Result;

async fn add_new_cert() {}

async fn check_cert_renewal() {}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_config::init_tracing();

    let db = db::create_db().await?;

    cmd::run(db).await?;
    Ok(())
}
