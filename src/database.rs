use crate::{
    error::MixError,
    operation::Operation,
    package::{self, Package},
};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    fs::File,
    path::{Path, PathBuf},
    rc::Rc,
};

/// The package database. It provides all actions needed to manage packages.
#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
    packages: Vec<Rc<RefCell<Package>>>,
    #[serde(skip)]
    package_cache: PathBuf,
}

impl Database {
    /// Given the name of a package, provide the package itself.
    pub(crate) fn get_package(
        &self,
        package_name: &impl AsRef<str>,
    ) -> Option<Rc<RefCell<Package>>> {
        self.iter()
            .find(|package| package.borrow().name == package_name.as_ref())
    }
    /// Provide an iterator over the values of the database.
    pub(crate) fn iter(&self) -> impl Iterator<Item = Rc<RefCell<Package>>> + '_ {
        self.packages.iter().cloned()
    }

    /// Add the given package to the database.
    pub(crate) fn import_package(&mut self, package: Rc<RefCell<Package>>) -> Result<(), MixError> {
        if self.packages.contains(&package) {
            return Ok(());
        }
        if let Some(tarball) = &package.borrow().local_path {
            let mut tarball = File::open(tarball)?;
            let destination = self.package_cache.join(package.borrow().get_filename());
            let mut destination = File::create(destination)?;
            std::io::copy(&mut tarball, &mut destination)?;
        }
        package.borrow_mut().local_path = None;
        self.packages.push(package);
        Ok(())
    }

    /// Load the package database from disk.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, MixError> {
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(err) => match err.kind() {
                std::io::ErrorKind::NotFound => {
                    return Err(MixError::FileNotFound(path.as_ref().to_owned()))
                }
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
    pub fn new_empty(package_cache: impl Into<PathBuf>) -> Self {
        Self {
            packages: vec![],
            package_cache: package_cache.into(),
        }
    }

    /// Handle the operation, using this database.
    pub fn handle_operation(&mut self, operation: Operation) -> Result<(), MixError> {
        match operation {
            Operation::Install(packages) => {
                package::install(&packages, self)?;
            }
            Operation::Remove(packages) => {
                package::remove(&packages, self)?;
            }
            Operation::Synchronize => todo!(),
            Operation::Update(packages) => {
                let packages = match packages {
                    Some(packages) => packages,
                    None => self.iter().collect(),
                };
                package::update(&packages, self)?;
            }
            Operation::Fetch(packages) => {
                let _client = reqwest::blocking::Client::new();
                for _package in packages {
                    todo!("Fetching is not yet implemented. This should download the PKGBUILD and sources.");
                }
            }
        }
        Ok(())
    }

    /// Get the path of the package within the package cache.
    pub fn open_package_tarball(&self, package: &Package) -> Result<impl std::io::Read, MixError> {
        let filename = self.package_cache.join(package.get_filename());
        if filename.exists() {
            return Ok(File::open(filename)?);
        }
        todo!()
    }
}
