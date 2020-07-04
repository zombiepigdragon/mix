use clap::ErrorKind;
use mix::arguments::parse_arguments;
use mix::package::Database;
use std::env;
use std::error::Error;
use std::{path::Path, process};

/// When there is no database found, prompt to create a new database.
fn create_new_database(path: &Path) -> Result<(), Box<dyn Error>> {
    eprintln!("The database was not found on disk. This can happen for 2 reasons:");
    eprintln!("1: The database was removed, and this installation is corrupt.");
    eprintln!("2: This is a new install of mix, and no such file exists.");
    eprintln!("\nIf you are in scenario 1 and do not have a backup of the database file, answer no and reinstall.");
    if dialoguer::Confirm::new()
        .with_prompt("Create a new package database?")
        .interact()?
    {
        println!("Creating a new database.");
        let database = Database::new_empty();
        database.save(path)?;
    } else {
        println!("Not creating a new database. Restore a backup.");
        return Err("No database available.".into());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
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
    // TODO: Should only call this when not finding the database
    //create_new_database(database_path)?;
    let package_list = Database::load(database_path);
    match package_list {
        Ok(mut package_list) => {
            action.execute(&mut package_list)?;
            package_list.save(database_path)?;
            Ok(())
        }
        // TODO: Handle errors here (difficult because dyn err)
        Err(err) => Err(err),
    }
}
