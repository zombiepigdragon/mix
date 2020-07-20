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
pub fn package_from_name(package_name: &str, database: &Database) -> SelectResults {
    match database
        .iter()
        .find(|package| package.borrow().name == package_name)
    {
        Some(package) => SelectResults::Results(vec![package]),
        None => SelectResults::NotFound(vec![package_name.to_owned()], vec![]),
    }
}

/// Turns a set of package names into their respective package objects.
pub fn packages_from_names(package_names: &[&str], database: &Database) -> SelectResults {
    let mut packages_found = Vec::new();
    let mut packages_not_found = Vec::new();
    package_names
        .iter()
        .map(|package_name| match database.get_package(package_name) {
            Some(package) => {
                packages_found.push(package);
            }
            None => {
                packages_not_found.push(String::from(*package_name));
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
