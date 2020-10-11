//! # The mix package manager
//! `mix` is a package manager that is designed around the PKGBUILD system. It handles loading packages from a community maintained repository, as well as the Arch Linux AUR.
//! To get started, run
//! ```sh
//! mix --help
//! ```
//! for a list of available commands.
//! # Note
//! This documentation is not the user manual for normal mix usage. Check the man page for end user documentation.
//! # Usage
//! Using mix is a fairly straightforward process. For example, to install a package named `foo`:
//! ```no_run
//! /// The packages that will be installed.
//! let package_names = vec!["foo"];
//! /// Load the database and use it to find the needed package metadata.
//! let mut database = mix::Database::load("/var/lib/mix/mix.db")?;
//! /// If the packages are found, mix::selection::install will provide every dependency needed to install the packages.
//! let packages = mix::selection::install(&package_names, &database).unwrap();
//! /// Select the operation to perform with the packages.
//! let operation = mix::Operation::Install(packages);
//! /// Perform the operation.
//! database.handle_operation(operation)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//! To remove `foo`, it's a similar process:
//! ```no_run
//! let package_names = vec!["foo"];
//! let mut database = mix::Database::load("/var/lib/mix/mix.db")?;
//! /// This won't include any dependencies that can't be removed with the given packages.
//! let packages = mix::selection::remove(&package_names, &database).unwrap();
//! let operation = mix::Operation::Remove(packages);
//! database.handle_operation(operation)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//! Synchronizing the database lists is not automatically performed for crate consumers, but it can be done manually with:
//! ```no_run
//! # let mut database = mix::Database::load("/var/lib/mix/mix.db")?;
//! let operation = mix::Operation::Synchronize;
//! database.handle_operation(operation)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//! Other operations can be found in [Operation](crate::Operation).
//! # Todo
//! There is a few things that be be implemented still before a functional prerelease, presented here in a predicted order:
//! - Provide package removal functionality.
//! - Handle package dependencies as best as possible.
//! - Restore the unit testing, and add some integration tests to the mix.
//! - Most likely, also add some form of property based testing: it seems like some components in mix may benefit from it.
//!
//! Any other functionality is either not currently high priority or was overlooked: contact me if it's not listed below.
//!
//! The current list of features not needed for an alpha prerelease but wanted for a stable:
//! - Provide an interface to mixpkg for local package builds.
//! - Connect to an online package repository for online packages.
//! - Sync the package lists with said online repository.
//! - Allow checking for updates to packages.
//! - Clean up the inevitable flaws in the dependency resolving.

#![warn(missing_docs)] // To keep codebase familiarity possible, docs are required

/// The package database. All functionality with storing the available packages
/// and the state of the installed packages is here.
pub mod database;
/// Errors that can be raised by the package manager.
pub mod error;
/// The packages database and structures.
pub mod package;
/// Selecting packages from the database for operations.
pub mod selection;

pub use database::Database;
pub use error::{MixError as Error, Result};
pub use package::{InstallState, Package, Version};
pub use selection::{install, package_from_name, packages_from_names, remove, Selections};
