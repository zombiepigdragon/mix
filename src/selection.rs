//! # Todo
//! This will see some revamp, separating dependencies from the manual targets

use crate::database::Database;
use crate::error::MixError;
use crate::package::{InstallState, Package};
use std::{cell::RefCell, rc::Rc};

/// The todo list for any given operation. For example, the list of packages
/// needing an install or upgrade.
#[derive(Debug, Default)]
pub struct Selections {
    /// Packages that will be installed by the operation.
    pub install: Vec<Rc<RefCell<Package>>>,
    /// Packages that will be removed by the operation.
    pub remove: Vec<Rc<RefCell<Package>>>,
    /// Packages that will be upgraded by the operation.
    pub upgrade: Vec<Rc<RefCell<Package>>>,
    /// Packages that will be downgraded by the operation.
    pub downgrade: Vec<Rc<RefCell<Package>>>,
}

/// Get a single package by name.
pub fn package_from_name(
    package_name: &impl AsRef<str>,
    database: &Database,
) -> Result<Rc<RefCell<Package>>, MixError> {
    match database
        .iter()
        .find(|package| package.borrow().name == package_name.as_ref())
    {
        Some(package) => Ok(package),
        None => Err(MixError::PackageNotFound(vec![String::from(
            package_name.as_ref(),
        )])),
    }
}

/// Turns a set of package names into their respective package objects.
/// # Errors
/// The error value contains first a [package not found error](crate::error::MixError::PackageNotFound),
/// followed by a [Vec](Vec) of all of the packages that were found. This allows for
/// error resolution via other means (searching for the package on disk, for
/// example.)
/// # Todo
/// The error return feels uncomfortable at best, and bad at worst.
/// If a cleaner way to handle it arises, it should be implemented as soon as
/// comfortable.
pub fn packages_from_names(
    package_names: &[impl AsRef<str>],
    database: &Database,
) -> Result<Vec<Rc<RefCell<Package>>>, (MixError, Vec<Rc<RefCell<Package>>>)> {
    let mut packages_found = Vec::new();
    let mut packages_not_found = Vec::new();
    package_names
        .iter()
        .map(|package_name| {
            if let Some(package) = database.get_package(package_name) {
                packages_found.push(package);
            } else {
                packages_not_found.push(String::from(package_name.as_ref()));
            }
        })
        .for_each(drop);
    if !packages_not_found.is_empty() {
        return Err((
            MixError::PackageNotFound(packages_not_found),
            packages_found,
        ));
    }
    Ok(packages_found)
}

/// Select the packages required for an installation of a package. This means
/// dependencies and resolution of package names to objects.
/// # Todo
/// This currently has the same error type as [packages_from_names](packages_from_names).
/// Once that function is updated, this function needs the same update.
///
/// This function needs to handle dependencies once packages support this.
///
/// Write similar functions once it makes sense at all to have them.
pub fn install(
    package_names: &[impl AsRef<str>],
    database: &Database,
) -> Result<Selections, (MixError, Vec<Rc<RefCell<Package>>>)> {
    let packages = packages_from_names(package_names, database)?;
    let mut selections = Selections::default();
    for package in packages {
        if package.borrow().state != InstallState::Uninstalled {
            continue;
        }
        selections.install.push(package.clone());
    }
    Ok(selections)
}

/// # Todo
/// Write this.
pub fn remove(
    _package_names: &[impl AsRef<str>],
    _database: &Database,
) -> Result<Selections, (MixError, Vec<Rc<RefCell<Package>>>)> {
    todo!()
}
