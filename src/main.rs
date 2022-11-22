mod cmd;
mod db;
mod deploy;
mod dns;
mod tracing_config;

use eyre::Result;

pub struct Certificate {
    pub cert: String,
    pub key: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_config::init_tracing();

    let db = db::create_db().await?;

    cmd::run(db).await?;
    Ok(())
}
