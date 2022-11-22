use std::sync::Arc;

use clap::{Args, Subcommand};
use eyre::Result;
use indicatif::ProgressBar;
use rusqlite::params;

use crate::db::PoolExtInteract;

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

struct Account {
    name: String,
    email: String,
}

pub async fn new_account(state: Arc<State>) -> Result<()> {
    let name = crate::cli::get_unique_name(&state, "account", "acme_accounts").await?;

    let email: String = dialoguer::Input::new()
        .with_prompt("What email address should you use?")
        .interact_text()?;

    let progress = ProgressBar::new_spinner().with_message("Creating account...");

    let email = format!("mailto:{}", email);

    let account = instant_acme::Account::create(
        &instant_acme::NewAccount {
            contact: &[email.as_str()],
            terms_of_service_agreed: true,
            only_return_existing: false,
        },
        state.acme_url(),
    )
    .await?;

    let creds = serde_json::to_string(&account.credentials())?;

    let provider = match state.staging {
        true => "lets_encrypt_staging",
        false => "lets_encrypt",
    };

    state
        .pool
        .interact(move |conn| {
            conn.execute(
                "INSERT INTO acme_accounts (name, provider, creds) VALUES (?, ?, ?)",
                params![&name, provider, &creds],
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
