use crate::{error::MixError, operation::Operation, package::Package};
use serde::{Deserialize, Serialize};
use std::{
    cell::{Ref, RefCell},
    fs::File,
    path::Path,
    rc::Rc,
};

/// The package database. It provides all actions needed to manage packages.
#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
    packages: Vec<Rc<RefCell<Package>>>,
}

impl Database {
    /// Given the name of a package, provide the package itself.
    pub fn get_package(&self, package_name: &str) -> Option<Rc<RefCell<Package>>> {
        self.iter()
            .find(|package| package.borrow().name == package_name)
    }

    /// Provide an iterator over the values of the database.
    pub fn iter(&self) -> impl Iterator<Item = Rc<RefCell<Package>>> + '_ {
        self.packages.iter().cloned()
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
                        update(package.borrow());
                        package.borrow_mut().mark_as_manually_installed();
                        package.borrow_mut().install();
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
