use crate::action::Actionable;
use crate::error::Error as mix_Error;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::Display;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::PathBuf;

/// The database that all package operations are stored upon. It does all the basic functionality required to keep track of installed packages.
pub trait PackageDatabase: Actionable {
    /// Loads the database from the specified path.
    ///
    /// Note: The path is also used automatically to save the database.
    fn load(filename: PathBuf) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized;
    /// Saves the database back to the disk with the path set in `PackageDatabase::load`.
    ///
    /// Note: Allowed to avoid actual writes to the disk if they are deemed unnecessary. For example, querying the database without installing any packages might not be written to disk.
    fn save(&self) -> Result<(), Box<dyn Error>>;
    /// Returns whether a call to `PackageDatabase::save` would have any effect.
    fn needs_save(&self) -> bool;
    /// Returns the first package with the given name.
    fn get_package(&self, package_name: &str) -> Option<&Package>;
    /// Returns the first package with the given name.
    fn get_mut_package(&mut self, package_name: &str) -> Option<&mut Package>;
}

/// The default package database.
#[derive(Serialize, Deserialize, Debug)]
pub struct PackageList {
    /// The location of the package cache.
    filename: PathBuf,
    /// The package cache.
    cache: Vec<Package>,
    /// The cache to be rewritten.
    #[serde(skip)]
    invalidated: bool,
}

impl PackageList {
    fn all_package_names(&self) -> Vec<String> {
        self.cache
            .iter()
            .map(|package| package.name.clone())
            .collect()
    }
}

impl Actionable for PackageList {
    fn install(&mut self, packages: &[String]) -> Result<(), Box<dyn Error>> {
        self.invalidated = true;
        for package_name in packages {
            if let Some(package) = self.get_package(package_name) {
                match package.install_state {
                    InstallState::ManuallyInstalled => {
                        eprintln!("Package {} is already installed.", package_name)
                    }
                    InstallState::DependencyInstalled => eprintln!(
                        "Package {} is already installed. Marking as manually installed.",
                        package_name
                    ),
                    InstallState::NotInstalled => eprintln!(
                        "Package {} is known but not installed. Marking as manually installed.",
                        package_name
                    ),
                };
                self.get_mut_package(package_name).unwrap().install_state =
                    InstallState::ManuallyInstalled;
            } else {
                println!("Installing {}", package_name);
                let package = Package {
                    name: package_name.clone(),
                    install_state: InstallState::ManuallyInstalled,
                };
                self.cache.push(package);
            }
        }
        Ok(())
    }

    fn remove(&mut self, packages: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        self.invalidated = true;
        for package_name in packages {
            if let Some(package) = self.get_mut_package(package_name) {
                println!("Removing {}.", package);
                package.install_state = InstallState::NotInstalled;
            }
        }
        Ok(())
    }

    fn synchronize(
        &mut self,
        _next_action: &Option<Box<crate::action::Action>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.invalidated = true;
        // Using Arch Linux's `base` dependencies list, hopefully they're okay with that
        let default_packages = vec![
            "bash",
            "bzip2",
            "coreutils",
            "file",
            "filesystem",
            "findutils",
            "gawk",
            "gcc-libs",
            "gettext",
            "glibc",
            "grep",
            "gzip",
            "iproute2",
            "iputils",
            "licenses",
            "pacman",
            "pciutils",
            "procps-ng",
            "psmisc",
            "sed",
            "shadow",
            "systemd",
            "systemd-sysvcompat",
            "tar",
            "util-linux",
            "xz",
            "linux",
        ];
        for package_name in default_packages {
            if self.get_package(package_name).is_none() {
                self.cache.push(Package {
                    name: package_name.to_string(),
                    install_state: InstallState::NotInstalled,
                });
            }
        }
        Ok(())
    }

    fn update(&mut self, packages: &Option<Vec<String>>) -> Result<(), Box<dyn std::error::Error>> {
        self.invalidated = true;
        let package_names = match packages {
            Some(package_names) => package_names.clone(),
            None => self.all_package_names(),
        };
        let mut packages = Vec::new();
        for package_name in package_names {
            if let Some(package) = self.get_package(&package_name) {
                packages.push(package);
            } else {
                eprintln!("Package {} not found, and can't be updated.", package_name);
                return Err(mix_Error::PackageNotFound.into());
            }
        }
        for package in packages {
            println!("Updating {}.", package.name);
        }
        Ok(())
    }

    fn fetch(&self, packages: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        for package_name in packages {
            let path = PathBuf::from(format!("{}.PKGBUILD", package_name));
            if path.exists() {
                eprintln!(
                    "File {} exists, skipping package {}.",
                    path.to_str().unwrap(),
                    package_name
                );
                continue;
            }
            println!("Fetching {}", package_name);
            let mut file = File::create(path)?;
            file.write_all(&format!("# This is a fake PKGBUILD for {}. It will be downloaded in future versions of mix.\n", package_name).into_bytes())?;
        }
        Ok(())
    }

    fn list(&self) -> Result<(), Box<dyn Error>> {
        for package in &self.cache {
            println!("Package '{}': {}", &package.name, &package.install_state);
        }
        Ok(())
    }
}

impl PackageDatabase for PackageList {
    fn load(path: PathBuf) -> Result<Self, Box<dyn Error>> {
        match File::open(&path) {
            Ok(file) => match serde_cbor::from_reader(file) {
                Ok(package_database) => Ok(package_database),
                Err(error) => match error.classify() {
                    serde_cbor::error::Category::Syntax => {
                        eprintln!("Database file had invalid syntax! Restore a backup.");
                        Err(error.into())
                    }
                    serde_cbor::error::Category::Data => {
                        eprintln!("Database file contains invalid data! Restore a backup.");
                        Err(error.into())
                    }
                    _ => {
                        eprintln!("Error parsing database: {}", error);
                        Err(error.into())
                    }
                },
            },
            Err(error) => match error.kind() {
                io::ErrorKind::NotFound => {
                    eprintln!("Warning: Failed to load package database from disk, creating an empty database.");
                    Ok(PackageList {
                        filename: path,
                        cache: Vec::new(),
                        invalidated: false,
                    })
                }
                io::ErrorKind::PermissionDenied => {
                    eprintln!("Permission denied opening package database.");
                    eprintln!("Are you root?");
                    Err(error.into())
                }
                _ => {
                    eprintln!("Error opening database: {}", error);
                    Err(error.into())
                }
            },
        }
    }

    fn save(&self) -> Result<(), Box<dyn Error>> {
        match File::create(&self.filename) {
            Ok(file) => Ok(serde_cbor::to_writer(file, &self)?),
            Err(error) => {
                match error.kind() {
                    io::ErrorKind::PermissionDenied => {
                        eprintln!("Permission denied opening package database.\nChanges could not be saved.");
                        Err(error.into())
                    }
                    _ => Err(error.into()),
                }
            }
        }
    }

    fn needs_save(&self) -> bool {
        self.invalidated
    }

    fn get_package(&self, package_name: &str) -> Option<&Package> {
        for package in &self.cache {
            if package.name == package_name {
                return Some(&package);
            }
        }
        None
    }

    fn get_mut_package(&mut self, package_name: &str) -> Option<&mut Package> {
        let mut index = None;
        for (i, package) in self.cache.iter().enumerate() {
            if package.name == package_name {
                index = Some(i);
                break;
            }
        }
        if let Some(index) = index {
            return self.cache.get_mut(index);
        }
        None
    }
}

/// The current installation state of a package
#[derive(Serialize, Deserialize, Debug)]
pub enum InstallState {
    /// The package was installed by a user.
    ManuallyInstalled,
    /// The package was installed as a dependency and can be removed during a clean.
    DependencyInstalled,
    /// The package is stored in the cache, but not installed.
    NotInstalled,
}

impl Display for InstallState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match &self {
            Self::ManuallyInstalled => "Manually Installed",
            Self::DependencyInstalled => "Dependency Installed",
            Self::NotInstalled => "Not Installed",
        };
        write!(f, "{}", message)
    }
}

/// The combination of all information making up a package.
#[derive(Serialize, Deserialize, Debug)]
pub struct Package {
    /// The current installation state of the package.
    pub install_state: InstallState,
    /// The name of the package.
    pub name: String,
}

impl Display for Package {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}
