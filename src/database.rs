use crate::{
    package::{self, Package, RcRefCellPackage},
    Error, Selections,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    path::{Path, PathBuf},
};

/// The package database. It provides all actions needed to manage packages.
#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
    packages: Vec<RcRefCellPackage>,
    #[serde(skip)]
    package_cache: PathBuf,
}

impl Database {
    /// Given the name of a package, provide the package itself.
    pub(crate) fn get_package(&self, package_name: &impl AsRef<str>) -> Option<RcRefCellPackage> {
        self.iter()
            .find(|package| package.borrow().name == package_name.as_ref())
    }
    /// Provide an iterator over the values of the database.
    pub(crate) fn iter(&self) -> impl Iterator<Item = RcRefCellPackage> + '_ {
        self.packages.iter().cloned()
    }

    /// Add the given package to the database.
    pub(crate) fn import_package(&mut self, package: RcRefCellPackage) -> crate::Result<()> {
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
    pub fn load(path: impl AsRef<Path>) -> crate::Result<Self> {
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(err) => match err.kind() {
                std::io::ErrorKind::NotFound => {
                    return Err(Error::FileNotFound(path.as_ref().to_owned()))
                }
                _ => return Err(Error::IOError(err)),
            },
        };
        Ok(serde_cbor::from_reader(file)?)
    }

    /// Save the current package database to the disk.
    pub fn save(&self, path: &Path) -> crate::Result<()> {
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
    pub fn apply(&mut self, selections: Selections) -> crate::Result<()> {
        package::install(&selections.install, self)?;
        package::remove(&selections.remove, self)?;
        package::update(&selections.upgrade, self)?;
        // TODO: Handle downgrades. For now, this is just warned on.
        eprintln!(
            "Not downgrading the following packages (Not yet implemented): {:?}",
            &selections.downgrade
        );
        Ok(())
    }

    /// Get the path of the package within the package cache.
    pub fn open_package_tarball(&self, package: &Package) -> crate::Result<impl std::io::Read> {
        let filename = self.package_cache.join(package.get_filename());
        if filename.exists() {
            return Ok(File::open(filename)?);
        }
        todo!()
    }

    /// Provide a way to iterate over all packages.
    /// # Todo:
    /// This is not an ideal way to handle it, but this commit is large enough
    /// that I'm going to use the path of least resistance. This should ideally
    /// iterate over immutable references.
    pub fn all_packages(&self) -> Vec<Package> {
        self.packages
            .iter()
            .map(|package| package.borrow().clone())
            .collect()
    }
}
