//! # The mix package manager
//! `mix` is a package manager that is designed around the PKGBUILD system. It handles loading packages from a community maintained repository, as well as the Arch Linux AUR.
//! To get started, run
//! ```sh
//! mix --help
//! ```
//! for a list of available commands.
//! # Status
//! Creates a dummy repository in the current directory, which can be operated on but does NOT affect files.
//! In addition, it is currently impossible to add new packages without editing the database manually.
//! There is a few things that be be implemented still before a functional prerelease, presented here in a predicted order:
//! - Allow packages to define files that they install.
//! - Provide a way to load packages from tarballs (reading packages from disk.)
//! - Allow packages to place filesystem files during installation.
//! - Provide package removal functionality.
//! - Handle package dependencies as best as possible.
//! - Restore the unit testing, and add some integration tests to the mix.
//! - Most likely, also add some form of property based testing: it seems like some components in mix may benefit from it.
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
/// The operations that can be performed, such as installing or removing packages.
pub mod operation;
/// The packages database and structures.
pub mod package;
/// Selecting packages from the database for operations.
pub mod selection;
