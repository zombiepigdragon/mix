use crate::package::Package;
use std::{cell::RefCell, rc::Rc};

/// An action that can be performed by the package database.
#[derive(Debug, PartialEq)]
pub enum Operation {
    /// Install packages.
    Install(Vec<Rc<RefCell<Package>>>),
    /// Uninstall packages.
    Remove(Vec<Rc<RefCell<Package>>>),
    /// Bring the cache up to date.
    Synchronize,
    /// Bring packages up to date.
    Update(Option<Vec<Rc<RefCell<Package>>>>),
    /// Download files for a package.
    Fetch(Vec<Rc<RefCell<Package>>>),
    /// List the installed packages.
    List,
}
