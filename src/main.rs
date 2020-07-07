use anyhow::{anyhow, Context, Result};
use clap::ErrorKind;
use mix::arguments::parse_arguments;
use mix::error::MixError;
use mix::package::Database;
use std::env;
use std::{path::Path, process};

/// When there is no database found, prompt to create a new database.
fn create_new_database(path: &Path) -> Result<()> {
    eprintln!("The database was not found on disk. This can happen for 2 reasons:");
    eprintln!("1: The database was removed, and this installation is corrupt.");
    eprintln!("2: This is a new install of mix, and no such file exists.");
    eprintln!("\nIf you are in scenario 1 and do not have a backup of the database file, answer no and reinstall.");
    if dialoguer::Confirm::new()
        .with_prompt("Create a new package database?")
        .interact()
        .context("Failed to display prompt.")?
    {
        println!("Creating a new database.");
        let database = Database::new_empty();
        database
            .save(path)
            .context("Failed to save the blank database to the disk.")?;
        println!("Blank database created. It's recommended to synchronize now.");
    } else {
        return Err(anyhow!(
            "Not creating a new package database. Restore a backup."
        ));
    }
    Ok(())
}

fn main() -> Result<()> {
    let action = parse_arguments(env::args_os());
    let action = match action {
        Ok(action) => action,
        Err(error) => {
            eprintln!("{}", error);
            let result = match error.kind {
                ErrorKind::HelpDisplayed => 0,
                ErrorKind::VersionDisplayed => 0,
                _ => 1,
            };
            process::exit(result);
        }
    };
    let database_path = Path::new(".mix.db");
    let package_list = Database::load(database_path).context("Failed to load package database");
    match package_list {
        Ok(mut package_list) => {
            action.execute(&mut package_list)?;
            package_list.save(database_path)?;
            Ok(())
        }
        Err(err) => {
            // The error is from mix.
            if let Some(err) = err.downcast_ref::<MixError>() {
                match err {
                    MixError::FileNotFound(_) => create_new_database(database_path)?,
                    _ => {
                        eprintln!("{}", err);
                    }
                };
                process::exit(1);
            }
            // The error is not a MixError, so it shouldn't be from the mix api.
            panic!("{}", err);
        }
    }
}
