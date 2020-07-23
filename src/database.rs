use crate::{error::MixError, operation::Operation, package::Package};
use serde::{Deserialize, Serialize};
use std::{
    cell::{Ref, RefCell},
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
    pub(crate) fn get_package(&self, package_name: &str) -> Option<Rc<RefCell<Package>>> {
        self.iter()
            .find(|package| package.borrow().name == package_name)
    }
    /// Provide an iterator over the values of the database.
    pub(crate) fn iter(&self) -> impl Iterator<Item = Rc<RefCell<Package>>> + '_ {
        self.packages.iter().cloned()
    }

    /// Add the given package to the database.
    pub(crate) fn import_package(&mut self, package: Rc<RefCell<Package>>) {
        if self.packages.contains(&package) {
            return;
        }
        if let Some(tarball) = &package.borrow().local_path {
            let mut tarball = File::open(tarball).unwrap();
            let destination = package.borrow().get_filename(&self.package_cache);
            let mut destination = File::create(destination).unwrap();
            std::io::copy(&mut tarball, &mut destination).unwrap();
        }
        package.borrow_mut().local_path = None;
        self.packages.push(package)
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
    pub fn new_empty(package_cache: impl Into<PathBuf>) -> Self {
        Self {
            packages: vec![],
            package_cache: package_cache.into(),
        }
    }

    /// Handle the operation, using this database.
    /// # Todo
    /// - Allow synchronization to take place at all.
    /// - Make the closures more general, and call them whenever needed.
    /// - Don't let every package download [example.com](https://www.example.com).
    pub fn handle_operation(
        &mut self,
        operation: &Operation,
        confirm: impl FnOnce() -> Result<bool, MixError>,
        before_run: impl FnOnce(&Vec<Rc<RefCell<Package>>>),
        update: &mut impl FnMut(Ref<Package>),
    ) -> Result<(), MixError> {
        match operation {
            Operation::Install(packages) => {
                if confirm()? {
                    before_run(&packages);
                    for package in packages {
                        self.import_package(package.clone());
                        update(package.borrow());
                        package.borrow_mut().mark_as_manually_installed();
                        let filename = package.borrow().get_filename(&self.package_cache);
                        package.borrow_mut().install(filename)?;
                    }
                } else {
                    return Ok(());
                }
            }
            Operation::Remove(packages) => {
                if confirm()? {
                    before_run(&packages);
                    for package in packages {
                        update(package.borrow());
                        package.borrow_mut().remove();
                    }
                } else {
                    println!("Aborting.")
                }
            }
            Operation::Synchronize => todo!(),
            Operation::Update(packages) => {
                let packages = match packages {
                    Some(packages) => packages.clone(),
                    None => self.iter().collect(),
                };
                if confirm()? {
                    before_run(&packages);
                    for package in packages {
                        update(package.borrow());
                        package.borrow_mut().update();
                    }
                } else {
                    return Err(MixError::Aborted);
                }
            }
            Operation::Fetch(packages) => {
                let client = reqwest::blocking::Client::new();
                before_run(&packages);
                for package in packages {
                    let filename = format!("./{}.PKGBUILD", package.borrow().name);
                    let mut file = File::create(&filename)?;
                    update(package.borrow());
                    package
                        .borrow()
                        .fetch(&client, "https://www.example.com", &mut file)?;
                }
            }
            // TODO: Call the callbacks here.
            Operation::List => {
                for package in self.iter() {
                    println!("{}", package.borrow());
                }
            }
        }
        Ok(())
    }
}
