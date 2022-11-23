use std::sync::Arc;

use clap::{Args, Subcommand};
use eyre::Result;
use indicatif::ProgressBar;
use rusqlite::params;
use strum::IntoEnumIterator;

use crate::{acme::AcmeProvider, db::PoolExtInteract};

use super::State;

#[derive(Debug, Args)]
pub struct AccountArgs {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[clap(about = "Add a new account")]
    New,
}

pub async fn new_account(state: Arc<State>) -> Result<()> {
    let name = crate::cli::get_unique_name(
        &state,
        "Give this ACME (Let's Encrypt) account a label",
        "acme_accounts",
    )
    .await?;

    let providers = AcmeProvider::iter().map(|p| p.label()).collect::<Vec<_>>();
    let provider_select = dialoguer::Select::new()
        .with_prompt("Which ACME provider do you want to use?")
        .items(&providers)
        .default(0)
        .interact()?;
    let provider = AcmeProvider::iter().nth(provider_select).unwrap();

    let email: String = dialoguer::Input::new()
        .with_prompt("What email address should be on the account?")
        .interact_text()?;

    let progress = state
        .progress
        .add(ProgressBar::new_spinner().with_message("Creating account..."));

    let email = format!("mailto:{}", email);

    let account = instant_acme::Account::create(
        &instant_acme::NewAccount {
            contact: &[email.as_str()],
            terms_of_service_agreed: true,
            only_return_existing: false,
        },
        provider.url(),
    )
    .await?;

    let creds = serde_json::to_string(&account.credentials())?;

    state
        .pool
        .interact(move |conn| {
            conn.execute(
                "INSERT INTO acme_accounts (name, provider, creds) VALUES (?, ?, ?)",
                params![&name, provider.as_ref(), &creds],
            )?;

            Ok::<_, eyre::Report>(())
        })
        .await?;

    progress.set_message("Done!");
    progress.finish();

    Ok(())
}

pub async fn run(state: Arc<State>, args: AccountArgs) -> Result<()> {
    match args.command {
        Commands::New => new_account(state).await?,
    };

    Ok(())
}
