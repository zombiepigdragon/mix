use anyhow::{anyhow, Context, Result};
use indicatif::*;
use mix::{database::Database, error::MixError, operation::Operation, package::Package, selection};
use structopt::StructOpt;

use std::{cell::RefCell, path::PathBuf, process, rc::Rc};

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

impl Options {
    fn get_operation(&self, database: &mut Database) -> Result<Operation> {
        Ok(match &self.command {
            SubCommands::Install { targets } => {
                let targets = selection::packages_from_names(
                    &targets.iter().map(String::as_str).collect::<Vec<_>>()[..],
                    database,
                );
                let targets = match targets {
                    selection::SelectResults::Results(targets) => targets,
                    selection::SelectResults::NotFound(missing, mut found) => {
                        let mut truly_missing = vec![];
                        for package_name in missing {
                            match std::fs::File::open(&package_name) {
                                Ok(file) => {
                                    let mut package = Package::from_tarball(file)?;
                                    package.local_path = Some(package_name.into());
                                    found.push(Rc::new(RefCell::new(package)));
                                }
                                Err(error) => match error.kind() {
                                    std::io::ErrorKind::NotFound => {
                                        truly_missing.push(package_name);
                                    }
                                    _ => {
                                        return Err(error)
                                            .context("Failed to read package as file.")
                                    }
                                },
                            }
                        }
                        if !truly_missing.is_empty() {
                            return Err(anyhow!("Failed to find packages {:?}", truly_missing));
                        }
                        found
                    }
                };
                Operation::Install(targets)
            }
            SubCommands::Remove { targets } => {
                let targets = selection::packages_from_names(
                    &targets.iter().map(String::as_str).collect::<Vec<_>>()[..],
                    database,
                );
                let targets = match targets {
                    selection::SelectResults::Results(targets) => targets,
                    selection::SelectResults::NotFound(_, missing) => {
                        return Err(anyhow!("Failed to find packages {:?}", missing));
                    }
                };
                Operation::Remove(targets)
            }
            SubCommands::Update { targets } => {
                if targets.is_empty() {
                    return Ok(Operation::Update(None));
                }
                let targets = selection::packages_from_names(
                    &targets.iter().map(String::as_str).collect::<Vec<_>>()[..],
                    database,
                );
                let targets = match targets {
                    selection::SelectResults::Results(targets) => targets,
                    selection::SelectResults::NotFound(_, missing) => {
                        return Err(anyhow!("Failed to find packages {:?}", missing));
                    }
                };
                Operation::Update(Some(targets))
            }
            SubCommands::Sync => Operation::Synchronize,
            SubCommands::Fetch { targets } => {
                let targets = selection::packages_from_names(
                    &targets.iter().map(String::as_str).collect::<Vec<_>>()[..],
                    database,
                );
                let targets = match targets {
                    selection::SelectResults::Results(targets) => targets,
                    selection::SelectResults::NotFound(_, missing) => {
                        return Err(anyhow!("Failed to find packages {:?}", missing));
                    }
                };
                Operation::Fetch(targets)
            }
            SubCommands::List => unreachable!("The list subcommand was not handled."),
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

/// Ask the user to confirm if they wish to perform the action about to be executed.
fn confirm_action(verb: &str, packages: &[Rc<RefCell<Package>>]) -> Result<bool> {
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
pub fn run() -> Result<()> {
    let options = Options::from_args();
    let mut database = get_package_database(&options);
    // Handle this case early.
    if let SubCommands::List = options.command {
        for package in selection::all_packages(&database) {
            let package = package.borrow();
            println!("{}\t{}\t{}", package.name, package.version, package.state);
        }
        return Ok(());
    }
    let operation = options.get_operation(&mut database)?;
    let confirmation = match &operation {
        Operation::Install(packages) => confirm_action("install", &packages)?,
        Operation::Remove(packages) => confirm_action("remove", &packages)?,
        Operation::Synchronize => true,
        Operation::Update(packages) => confirm_action(
            "update",
            &match packages {
                Some(packages) => packages.clone(),
                None => vec![],
            },
        )?,
        Operation::Fetch(_) => true,
    };
    if !confirmation {
        return Err(MixError::Aborted.into());
    }
    let bar = ProgressBar::new(0).with_style(
        ProgressStyle::default_spinner()
            .template("{spinner} {pos}/{len} {prefix} {msg} {percent}% {wide_bar} {eta}"),
    );
    match &operation {
        Operation::Install(packages) => enable_progress_bar(&bar, "Installing", packages.len()),
        Operation::Remove(packages) => enable_progress_bar(&bar, "Removing", packages.len()),
        Operation::Synchronize => {}
        Operation::Update(packages) => match packages {
            Some(packages) => enable_progress_bar(&bar, "Updating", packages.len()),
            None => enable_progress_bar(&bar, "Updating everything", 0),
        },
        Operation::Fetch(packages) => enable_progress_bar(&bar, "Fetching", packages.len()),
    }
    // BUG/TODO: The progress bar obscures output. This can't be solved until
    // progress reporting is better implemented, so it's being left for now.
    bar.finish_and_clear();
    database.handle_operation(operation)?;
    bar.set_style(ProgressStyle::default_spinner().template("Finished in {elapsed}."));
    bar.disable_steady_tick();
    bar.finish();
    database
        .save(&options.database)
        .context("Failed to save database.")?;
    Ok(())
}
