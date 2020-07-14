use anyhow::{anyhow, Context, Result};
use mix::action::Action;
use mix::error::MixError;
use mix::package::Database;

use std::env;
use std::{
    path::{Path, PathBuf},
    process,
};

/// The options to use throughout the application. These should be set by arguments.
#[derive(Debug)]
struct Options {
    action: Action,
    database_path: PathBuf,
}

impl Options {
    /// Create a new Options from the environment.
    pub fn parse() -> Result<Options> {
        let result = mix::arguments::parse_arguments(env::args());
        let action = match result {
            Ok(action) => Ok(action),
            Err(error) => match error.kind {
                clap::ErrorKind::MissingArgumentOrSubcommand => {
                    println!("{}", error);
                    process::exit(1);
                }
                clap::ErrorKind::HelpDisplayed | clap::ErrorKind::VersionDisplayed => {
                    println!("{}", error);
                    process::exit(0);
                }
                _ => Err(error),
            },
        }?;
        Ok(Options {
            action,
            database_path: ".mix.db".into(),
        })
    }
}

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
        eprintln!(
            "Blank database created. Continuing execution, but synchronizing is recommended."
        );
    } else {
        return Err(anyhow!(
            "Not creating a new package database. Restore a backup."
        ));
    }
    Ok(())
}

/// Load the package database. This will exit the process if the package database cannot be loaded for any reason.
fn get_package_database(database_path: &Path) -> Database {
    match Database::load(database_path) {
        Ok(database) => database,
        Err(error) => match error {
            MixError::FileNotFound(_) => {
                if let Err(error) = create_new_database(database_path) {
                    eprintln!("{}", error);
                    process::exit(1)
                }
                Database::load(database_path).unwrap()
            }
            // The error is of an unprepared type, so we can't deal with it
            error => unimplemented!("Unhandled error loading database: {:#?}", error),
        },
    }
}

/// The entry point of the application.
fn main() -> Result<()> {
    let options = Options::parse().context("Failed to parse arguments.")?;
    let mut database = get_package_database(&options.database_path);
    options.action.execute(&mut database)?;
    database
        .save(&options.database_path)
        .context("Failed to save database.")?;
    Ok(())
}
