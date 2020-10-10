use anyhow::{anyhow, Context, Result};
use indicatif::*;
use mix::{database::Database, error::MixError, selection::Selections};
use std::{path::PathBuf, process};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt()]
struct Options {
    /// The configuration file containing various system options.
    #[structopt(short = "C", long, default_value = "mix.conf", parse(from_os_str))]
    configuration: PathBuf,

    /// The file containing the package database.
    #[structopt(long, default_value = ".mix.db", parse(from_os_str))]
    database: PathBuf,

    #[structopt(long, default_value = ".mix.cache/", parse(from_os_str))]
    /// Where downloaded packages are stored prior to installing.
    package_cache: PathBuf,

    #[structopt(subcommand)]
    command: SubCommands,
}

#[derive(Debug, StructOpt)]
enum SubCommands {
    /// Install the given packages.
    #[structopt(alias = "in")]
    Install {
        #[structopt()]
        /// The packages to install.
        targets: Vec<String>,
    },
    /// Remove the given packages.
    #[structopt(alias = "re")]
    Remove {
        #[structopt()]
        /// The packages to uninstall.
        targets: Vec<String>,
    },
    /// Update the given packages, or every out of date package if no arguments are given.
    #[structopt(alias = "up")]
    Update {
        #[structopt()]
        /// The packages to update (defaults to every package)
        targets: Vec<String>,
    },
    /// Bring the package database up to date.
    #[structopt(alias = "sy")]
    Sync,
    /// Download the files of the given packages.
    #[structopt(alias = "fe")]
    Fetch {
        #[structopt()]
        /// The packages to download.
        targets: Vec<String>,
    },
    /// List every known package.
    #[structopt(alias = "li")]
    List,
}

/// When there is no database found, prompt to create a new database.
fn create_new_database(options: &Options) -> Result<()> {
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
        let database = Database::new_empty(&options.package_cache);
        database
            .save(&options.database)
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
fn get_package_database(options: &Options) -> Database {
    match Database::load(&options.database) {
        Ok(database) => database,
        Err(error) => match error {
            MixError::FileNotFound(_) => {
                if let Err(error) = create_new_database(options) {
                    eprintln!("{}", error);
                    process::exit(1)
                }
                Database::load(&options.database).unwrap()
            }
            // The error is of an unprepared type, so we can't deal with it
            error => unimplemented!("Unhandled error loading database: {:#?}", error),
        },
    }
}

/// Perform the subcommand if it does not require modifying the database, and
/// get the needed changes if it does.
fn process_subcommand(
    subcommand: &SubCommands,
    database: &Database,
) -> Result<Option<Selections>, MixError> {
    use SubCommands::*;
    Ok(match subcommand {
        Install { targets: _ } => todo!("Installing packages is not yet implemented."),
        Remove { targets: _ } => todo!("Removing packages is not yet implemented."),
        Update { targets: _ } => todo!("Updating packages is not yet implemented."),
        SubCommands::Sync => todo!("Synchronizing with remote servers is not yet implemented."),
        SubCommands::Fetch { targets: _ } => {
            todo!("Fetching packages from remote servers is not yet implemented.")
        }
        SubCommands::List => {
            for package in database.all_packages() {
                let package = package;
                println!("{}\t{}\t{}", package.name, package.version, package.state);
            }
            None
        }
    })
}

/// Ask the user to confirm if they wish to perform the action about to be executed.
fn confirm_action(selections: &Selections) -> Result<bool> {
    if !selections.install.is_empty() {
        println!("Packages to be installed:");
        for package in &selections.install {
            println!("\t{}", package.borrow().name);
        }
    }
    if !selections.upgrade.is_empty() {
        println!("Packages to be upgraded:");
        for package in &selections.upgrade {
            println!("\t{}", package.borrow().name);
        }
    }
    if !selections.downgrade.is_empty() {
        println!("Packages to be downgraded:");
        for package in &selections.downgrade {
            println!("\t{}", package.borrow().name);
        }
    }
    if !selections.remove.is_empty() {
        println!("Packages to be remove:");
        for package in &selections.remove {
            println!("\t{}", package.borrow().name);
        }
    }
    dialoguer::Confirm::new()
        .with_prompt("Do you want to apply these changes?")
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
pub fn run() -> Result<()> {
    let options = Options::from_args();
    let mut database = get_package_database(&options);
    let selections = process_subcommand(&options.command, &database)?;
    if let Some(selections) = selections {
        //TODO: Add a progress bar back into the application.
        if !confirm_action(&selections)? {
            return Err(MixError::Aborted.into());
        }
        database.apply(selections)?;
    }
    database
        .save(&options.database)
        .context("Failed to save database.")?;
    Ok(())
}
