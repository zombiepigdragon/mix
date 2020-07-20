use anyhow::{anyhow, Context, Result};
use indicatif::*;
use mix::{database::Database, error::MixError, operation::Operation, package::Package};
use structopt::StructOpt;

use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    process,
    rc::Rc,
};

#[derive(Debug, StructOpt)]
#[structopt()]
struct Options {
    /// The configuration file containing various system options.
    #[structopt(short = "C", long, default_value = "mix.conf", parse(from_os_str))]
    configuration: PathBuf,

    /// The file containing the package database.
    #[structopt(long, default_value = ".mix.db", parse(from_os_str))]
    database: PathBuf,

    #[structopt(subcommand)]
    command: SubCommands,
}

impl Options {
    fn packages_from_names(
        package_names: &Vec<String>,
        database: &mut Database,
    ) -> Option<Vec<Rc<RefCell<Package>>>> {
        let packages: Vec<Rc<RefCell<Package>>> = package_names
            .iter()
            .filter_map(|package_name| database.get_package(package_name))
            .collect();
        if packages.len() == package_names.len() {
            Some(packages)
        } else {
            None
        }
    }

    fn get_operation(&self, database: &mut Database) -> Option<Operation> {
        Some(match &self.command {
            SubCommands::Install { targets } => {
                Operation::Install(Self::packages_from_names(targets, database).unwrap())
            }
            SubCommands::Remove { targets } => {
                Operation::Remove(Self::packages_from_names(targets, database).unwrap())
            }
            SubCommands::Update { targets } => {
                Operation::Update(Self::packages_from_names(targets, database))
            }
            SubCommands::Sync => Operation::Synchronize,
            SubCommands::Fetch { targets } => {
                Operation::Fetch(Self::packages_from_names(targets, database).unwrap())
            }
            SubCommands::List => Operation::List,
        })
    }
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
fn confirm_action(verb: &str, packages: &Vec<Rc<RefCell<Package>>>) -> Result<bool> {
    println!("This action will {} the following packages:", verb);
    for package in packages {
        println!("\t{}", package.borrow().name);
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
    //let options = Options::parse().context("Failed to parse arguments.")?;
    let options = Options::from_args();
    let mut database = get_package_database(&options.database);
    let operation = options.get_operation(&mut database).unwrap();
    let bar = ProgressBar::new(0).with_style(
        ProgressStyle::default_spinner()
            .template("{spinner} {pos}/{len} {prefix} {msg} {percent}% {wide_bar} {eta}"),
    );
    database.handle_operation(
        &operation,
        || {
            let (verb, packages) = match &operation {
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
            match confirm_action(verb, packages) {
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
        .save(&options.database)
        .context("Failed to save database.")?;
    Ok(())
}
