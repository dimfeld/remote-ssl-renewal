use eyre::Result;
use std::sync::Arc;

use crate::{cmd::State, db::PoolExtInteract};

pub async fn get_unique_name(state: &Arc<State>, prompt: &str, sql_table: &str) -> Result<String> {
    loop {
        let potential_name: String = dialoguer::Input::new()
            .with_prompt(prompt)
            .interact_text()?;

        // Better to use dialoguer's validate functionality but this is easier for MVP
        let exists = {
            let potential_name = potential_name.clone();
            let statement = format!("SELECT name FROM {sql_table} WHERE name = ?");
            state
                .pool
                .interact(move |conn| {
                    let mut stmt = conn.prepare_cached(&statement)?;
                    let exists = stmt.exists([&potential_name])?;
                    Ok::<_, eyre::Report>(exists)
                })
                .await?
        };

        if exists {
            println!("This name is already in use. Please try again.")
        } else {
            return Ok(potential_name);
        }
    }
}
