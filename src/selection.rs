//! # Todo
//! This will see some revamp, separating dependencies from the manual targets
//! and maybe even making the SelectResults an actual Result.

use crate::database::Database;
use crate::package::Package;
use std::{cell::RefCell, rc::Rc};

/// The results of selecting a set of packages.
pub enum SelectResults {
    /// All of the packages selected were found.
    Results(Vec<Rc<RefCell<Package>>>),
    /// Of the selected packages, these were not found.
    NotFound(Vec<String>, Vec<Rc<RefCell<Package>>>),
}

/// Get a single package by name.
pub fn package_from_name(package_name: &impl AsRef<str>, database: &Database) -> SelectResults {
    match database
        .iter()
        .find(|package| package.borrow().name == package_name.as_ref())
    {
        Some(package) => SelectResults::Results(vec![package]),
        None => SelectResults::NotFound(vec![package_name.as_ref().to_owned()], vec![]),
    }
}

/// Turns a set of package names into their respective package objects.
pub fn packages_from_names(
    package_names: &[impl AsRef<str>],
    database: &Database,
) -> SelectResults {
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
        return SelectResults::NotFound(packages_not_found, packages_found);
    }
    SelectResults::Results(packages_found)
}

/// Gets every package in the database.
pub fn all_packages(database: &mut Database) -> SelectResults {
    SelectResults::Results(database.iter().collect())
}

/// Get all the dependencies of the package.
/// # Todo
/// This does not actually account for dependencies, it just finds the named packages.
pub fn with_dependencies(package_names: &[impl AsRef<str>], database: &Database) -> SelectResults {
    packages_from_names(package_names, database)
}
