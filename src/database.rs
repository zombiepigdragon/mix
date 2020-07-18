use crate::error::MixError;
use crate::package::Package;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::{cell::RefCell, path::Path, rc::Rc};

/// The package database. It provides all actions needed to manage packages.
#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
    packages: Vec<Rc<RefCell<Package>>>,
}

impl Database {
    /// Given the name of a package, provide the package itself.
    pub fn get_package(&self, package_name: &str) -> Option<&Rc<RefCell<Package>>> {
        self.iter()
            .find(|package| package.borrow().name == package_name)
    }

    /// Provide an iterator over the values of the database.
    pub fn iter(&self) -> std::slice::Iter<Rc<RefCell<Package>>> {
        self.packages.iter()
    }

    /// Add the given package to the database.
    pub fn add_package(&mut self, package: Package) {
        self.packages.push(Rc::from(RefCell::from(package)))
    }

    /// Load the package database from disk.
    pub fn load(path: &Path) -> Result<Self, MixError> {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(err) => match err.kind() {
                std::io::ErrorKind::NotFound => return Err(MixError::FileNotFound(path.into())),
                _ => return Err(MixError::IOError(err)),
            },
        };
        Ok(serde_cbor::from_reader(file)?)
    }

    /// Save the current package database to the disk.
    pub fn save(&self, path: &Path) -> Result<(), MixError> {
        let file = File::create(path)?;
        Ok(serde_cbor::to_writer(file, self)?)
    }

    /// Create an empty database. Should only be used on fresh installs.
    pub fn new_empty() -> Self {
        Self { packages: vec![] }
    }
}
