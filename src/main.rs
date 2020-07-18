use anyhow::{anyhow, Context, Result};
use indicatif::*;
use mix::database::Database;
use mix::error::MixError;
use mix::operation::Operation;

use std::{
    env,
    path::{Path, PathBuf},
    process,
};

/// The options to use throughout the application. These should be set by arguments.
#[derive(Debug)]
struct Options {
    operation: Operation,
    database_path: PathBuf,
}

impl Options {
    /// Create a new Options from the environment.
    pub fn parse() -> Result<Options> {
        use clap::{app_from_crate, crate_authors, crate_description, crate_name, crate_version};
        use clap::{AppSettings, Arg, SubCommand};
        let result: clap::Result<Operation> = {
            let app = app_from_crate!()
                .subcommand(
                    SubCommand::with_name("install")
                        .about("Installs a package")
                        .arg(
                            Arg::with_name("target")
                                .help("The package(s) to install")
                                .min_values(1)
                                .required(true)
                                .index(1),
                        )
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .visible_alias("in"),
                )
                .subcommand(
                    SubCommand::with_name("remove")
                        .about("Removes a package")
                        .arg(
                            Arg::with_name("target")
                                .help("The package(s) to remove")
                                .min_values(1)
                                .required(true)
                                .index(1),
                        )
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .visible_alias("re"),
                )
                .subcommand(
                    SubCommand::with_name("synchronize")
                        .about("Synchronizes the package database")
                        .visible_alias("sy"),
                )
                .subcommand(
                    SubCommand::with_name("update")
                        .about("Updates a package")
                        .arg(
                            Arg::with_name("target")
                                .help("The packages to update")
                                .min_values(1)
                                .index(1),
                        )
                        .visible_alias("up"),
                )
                .subcommand(
                    SubCommand::with_name("fetch")
                        .about("Downloads a package without installing it")
                        .arg(
                            Arg::with_name("target")
                                .help("The package(s) to fetch")
                                .min_values(1)
                                .required(true)
                                .index(1),
                        )
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .visible_alias("fe"),
                )
                .subcommand(
                    SubCommand::with_name("list")
                        .about("Lists the installed packages")
                        .visible_alias("li"),
                )
                .setting(AppSettings::SubcommandRequiredElseHelp);

            let matches = app.get_matches_safe()?;

            let (subcommand_name, subcommand_arguments) = matches.subcommand();
            Ok(Operation::new(
                subcommand_name,
                &match subcommand_arguments {
                    Some(values) => {
                        if let Some(packages) = values.values_of("target") {
                            Some(packages.map(String::from).collect())
                        } else {
                            None
                        }
                    }
                    None => None,
                },
            ))
        };
        let operation = match result {
            Ok(operation) => Ok(operation),
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
            operation,
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

/// Ask the user to confirm if they wish to perform the action about to be executed.
fn confirm_action(verb: &str, package_names: &[&str]) -> Result<bool> {
    println!("This action will {} the following packages:", verb);
    for package_name in package_names {
        println!("\t{}", package_name);
    }
    dialoguer::Confirm::new()
        .with_prompt(format!("Do you want to {} these packages?", verb))
        .interact()
        .context("Failed to display prompt!")
}

/// Prepare the progress bar for usage in mix operations.
fn enable_progress_bar(bar: &ProgressBar, verb: &str, packages_count: usize) {
    bar.set_length(packages_count as u64);
    bar.set_prefix(verb);
    bar.reset_elapsed();
    bar.enable_steady_tick(20);
}

/// The entry point of the application.
fn main() -> Result<()> {
    let options = Options::parse().context("Failed to parse arguments.")?;
    let mut database = get_package_database(&options.database_path);
    let bar = ProgressBar::new(0).with_style(
        ProgressStyle::default_spinner()
            .template("{spinner} {pos}/{len} {prefix} {msg} {percent}% {wide_bar} {eta}"),
    );
    database.handle_operation(
        &options.operation,
        || {
            let (verb, package_names) = match &options.operation {
                Operation::Install(packages) => ("install", packages),
                Operation::Remove(packages) => ("remove", packages),
                // Don't verify for a manual synchronization
                Operation::Synchronize => return Ok(true),
                Operation::Update(packages) => {
                    match packages {
                        Some(packages) => ("update", packages),
                        // Don't verify all package updating
                        // TODO: This should be verified.
                        None => return Ok(true),
                    }
                }
                // Don't verify a fetch.
                Operation::Fetch(_) => return Ok(true),
                // Don't verify a list.
                Operation::List => return Ok(true),
            };
            match confirm_action(
                verb,
                &package_names.iter().map(|s| &s[..]).collect::<Vec<&str>>()[..],
            ) {
                Ok(result) => Ok(result),
                Err(error) => {
                    eprintln!("Error: {:#?}", error);
                    Err(MixError::Aborted)
                }
            }
        },
        |packages| enable_progress_bar(&bar, "TODO: Name this", packages.len()),
        &mut |package| {
            bar.inc(1);
            bar.println(format!("{} {}", "Placeholder verb-ing", package.name));
            bar.set_message(&package.name)
        },
    )?;
    bar.set_style(ProgressStyle::default_spinner().template("Finished in {elapsed}."));
    bar.disable_steady_tick();
    bar.finish();
    database
        .save(&options.database_path)
        .context("Failed to save database.")?;
    Ok(())
}
