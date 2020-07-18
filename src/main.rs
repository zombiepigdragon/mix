use anyhow::{anyhow, Context, Result};
use indicatif::*;
use mix::database::Database;
use mix::error::MixError;
use mix::{operation::Operation, package::Package, selection::SelectResults};

use std::{
    cell::RefCell,
    env,
    fs::File,
    path::{Path, PathBuf},
    process,
    rc::Rc,
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
    match options.operation {
        Operation::Install(packages) => {
            let packages = match mix::selection::packages_from_names(
                &packages.iter().map(|s| &s[..]).collect::<Vec<&str>>()[..],
                &mut database,
            ) {
                SelectResults::Results(packages) => packages,
                SelectResults::NotFound(_) => todo!(),
            };
            if confirm_action("install", &packages)? {
                enable_progress_bar(&bar, "install", packages.len());
                for package in packages {
                    bar.set_message(&package.borrow().name);
                    package.borrow_mut().mark_as_manually_installed();
                    package.borrow_mut().install();
                    bar.inc(1);
                }
            } else {
                println!("Aborting.")
            }
        }
        Operation::Remove(packages) => {
            let packages = match mix::selection::packages_from_names(
                &packages.iter().map(|s| &s[..]).collect::<Vec<&str>>()[..],
                &mut database,
            ) {
                SelectResults::Results(packages) => packages,
                SelectResults::NotFound(_) => todo!(),
            };
            if confirm_action("remove", &packages)? {
                enable_progress_bar(&bar, "remove", packages.len());
                for package in packages {
                    bar.set_message(&package.borrow().name);
                    package.borrow_mut().remove();
                    bar.inc(1);
                }
            } else {
                println!("Aborting.")
            }
        }
        Operation::Synchronize => todo!(),
        Operation::Update(packages) => {
            let packages = match packages {
                Some(packages) => match mix::selection::packages_from_names(
                    &packages.iter().map(|s| &s[..]).collect::<Vec<&str>>()[..],
                    &mut database,
                ) {
                    SelectResults::Results(packages) => packages,
                    SelectResults::NotFound(_) => todo!(),
                },
                None => {
                    if let SelectResults::Results(packages) =
                        mix::selection::all_packages(&mut database)
                    {
                        packages
                    } else {
                        unreachable!()
                    }
                }
            };
            if confirm_action("update", &packages)? {
                enable_progress_bar(&bar, "update", packages.len());
                for package in packages {
                    package.borrow_mut().update();
                }
            } else {
                println!("Aborting.")
            }
        }
        Operation::Fetch(packages) => {
            let packages = match mix::selection::packages_from_names(
                &packages.iter().map(|s| &s[..]).collect::<Vec<&str>>()[..],
                &mut database,
            ) {
                SelectResults::Results(packages) => packages,
                SelectResults::NotFound(_) => todo!(),
            };
            let client = reqwest::blocking::Client::new();
            enable_progress_bar(&bar, "fetch", packages.len());
            for package in packages {
                let filename = format!("./{}.PKGBUILD", package.borrow().name);
                let mut file = File::create(&filename)?;
                bar.set_message(&filename);
                package
                    .borrow()
                    .fetch(&client, "https://www.example.com", &mut file)?;
                bar.inc(1);
            }
        }
        Operation::List => {
            for package in database.iter() {
                println!("{}", package.borrow());
            }
        }
    }
    bar.set_style(ProgressStyle::default_spinner().template("Finished in {elapsed}."));
    bar.disable_steady_tick();
    bar.finish();
    database
        .save(&options.database_path)
        .context("Failed to save database.")?;
    Ok(())
}
