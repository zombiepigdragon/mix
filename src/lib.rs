//! # The mix package manager
//! `mix` is a package manager that is designed around the PKGBUILD system. It handles loading packages from a community maintained repository, as well as the Arch Linux AUR.
//! To get started, run
//! ```sh
//! mix --help
//! ```
//! for a list of available commands.
//! # Status
//! Creates a dummy repository in the current directory, which can be operated on but does NOT affect packages.

#![warn(missing_docs)] // To keep codebase familiarity possible, docs are required
/// Errors that can be raised by the package manager.
pub mod error;
/// The operations that can be performed, such as installing or removing packages.
pub mod operation;
/// The packages database and structures.
pub mod package;
/// Selecting packages from the database for operations.
pub mod selection;
