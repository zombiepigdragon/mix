use crate::action::Actionable;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{fs::File, io::prelude::*};

/// A singular package. A package is a name, list of files, and some metadata.
/// The metadata is what allows retrieving a package, viewing the files of a package, and many similar actions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Package {
    name: String,
    version: Version,
    state: InstallState,
}

impl std::fmt::Display for Package {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:\t{};\t{}", self.name, self.version, self.state)
    }
}

/// The package database. It provides all actions needed to manage packages.
#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
    packages: Vec<Package>,
}

impl Database {
    /// Given the name of a package, provide the package itself.
    pub fn get_package(&self, package_name: &str) -> Option<&Package> {
        for package in &self.packages {
            if package.name == package_name {
                return Some(package);
            }
        }
        None
    }
    /// Given the name of a package, provide the package itself.
    pub fn get_mut_package(&mut self, package_name: &str) -> Option<&mut Package> {
        for package in &mut self.packages {
            if package.name == package_name {
                return Some(package);
            }
        }
        None
    }

    /// Load the package database from disk.
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        Ok(serde_cbor::from_reader(file)?)
    }

    /// Save the current package database to the disk.
    pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::create(path)?;
        Ok(serde_cbor::to_writer(file, self)?)
    }

    /// Create an empty database. Should only be used on fresh installs.
    pub fn new_empty() -> Self {
        Self { packages: vec![] }
    }
}

impl Actionable for Database {
    fn install(&mut self, packages: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        for package_name in packages {
            if let Some(package) = self.get_mut_package(package_name) {
                println!("Installing {}", package_name);
                package.state = InstallState::Manual;
            } else {
                return Err(format!("Failed to find package {}.", package_name).into());
            }
        }
        Ok(())
    }

    fn remove(&mut self, packages: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        for package_name in packages {
            if let Some(package) = self.get_mut_package(package_name) {
                println!("Removing {}", package_name);
                package.state = InstallState::Uninstalled;
            } else {
                return Err(format!("Didn't recognize package {}.", package_name).into());
            }
        }
        Ok(())
    }

    fn synchronize(
        &mut self,
        next_action: &Option<Box<crate::action::Action>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
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
                self.packages.push(Package {
                    name: package_name.to_string(),
                    version: Version::Unknown,
                    state: InstallState::Uninstalled,
                });
            }
        }
        if let Some(action) = next_action {
            action.execute(self)?;
        }
        Ok(())
    }

    fn update(&mut self, packages: &Option<Vec<String>>) -> Result<(), Box<dyn std::error::Error>> {
        if packages.is_none() {
            todo!("Currently needs a package list!");
        }
        let packages = packages.clone().unwrap();
        for package_name in &packages {
            let mut package = match self.get_mut_package(package_name) {
                Some(package) => package,
                None => return Err(format!("Failed to find package {}", package_name).into()),
            };
            if let InstallState::Uninstalled = package.state {
                return Err(format!("Package {} is not installed.", package_name).into());
            }
            println!("Updating {}", package.name);
            package.version = match package.version {
                Version::SemVer(x, y, z) => Version::SemVer(x + 1, y, z),
                Version::Unknown => Version::SemVer(0, 0, 0),
            };
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

    fn list(&self) -> Result<(), Box<dyn std::error::Error>> {
        for package in &self.packages {
            println!("{}", package);
        }
        Ok(())
    }
}

/// The current state of the package.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InstallState {
    /// The package was installed intentionally, and can not be automatically removed.
    Manual,
    /// The package was installed to build another package or as a runtime dependency of a package.
    /// It can be removed if and only if no other packages depend on it.
    Dependency,
    /// The package is not currently installed.
    Uninstalled,
}

impl std::fmt::Display for InstallState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                InstallState::Manual => "Manually installed",
                InstallState::Dependency => "Dependency installation",
                InstallState::Uninstalled => "Not installed",
            }
        )
    }
}

/// A package's version.
/// # Examples:
/// ```rust
/// # use mix::package::Version;
/// // Everything is greater than Version::Unknown
/// assert!(Version::SemVer(0, 0, 0) > Version::Unknown);
/// assert!(Version::SemVer(1, 0, 0) > Version::Unknown);
/// // Check twice for asymmetry
/// assert!(Version::Unknown < Version::SemVer(0, 0, 0));
/// assert!(Version::Unknown < Version::SemVer(1, 0, 0));
/// // Equal versions are the same
/// assert_eq!(Version::SemVer(1, 0, 0), Version::SemVer(1, 0, 0));
/// assert_eq!(Version::SemVer(0, 1, 0), Version::SemVer(0, 1, 0));
/// assert_eq!(Version::SemVer(1, 0, 1), Version::SemVer(1, 0, 1));
/// assert_eq!(Version::Unknown, Version::Unknown);
/// // Normal version checks
/// assert!(Version::SemVer(1, 0, 0) > Version::SemVer(0, 1, 0));
/// assert!(Version::SemVer(0, 0, 1) > Version::SemVer(0, 0, 0));
/// assert!(Version::SemVer(1, 0, 0) < Version::SemVer(2, 1, 0));
/// ```
#[derive(Clone, Debug, Eq, Serialize, Deserialize)]
pub enum Version {
    /// A semantic version.
    SemVer(u32, u32, u32),
    /// The version is unknown and/or doesn't matter. It's always smaller than any other version.
    Unknown,
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        match (self, other) {
            (Version::SemVer(maj1, min1, rev1), Version::SemVer(maj2, min2, rev2)) => {
                if maj1 != maj2 {
                    maj1.cmp(maj2)
                } else if min1 != min2 {
                    min1.cmp(min2)
                } else if rev1 != rev2 {
                    rev1.cmp(rev2)
                } else {
                    Ordering::Equal
                }
            }
            (Version::SemVer(_, _, _), Version::Unknown) => Ordering::Greater,
            (Version::Unknown, Version::SemVer(_, _, _)) => Ordering::Less,
            (Version::Unknown, Version::Unknown) => Ordering::Equal,
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Version::SemVer(maj1, min1, rev1) => match other {
                Version::SemVer(maj2, min2, rev2) => maj1 == maj2 && min1 == min2 && rev1 == rev2,
                Version::Unknown => false,
            },
            Version::Unknown => match other {
                Version::SemVer(_, _, _) => false,
                Version::Unknown => true,
            },
        }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Version::SemVer(x, y, z) => write!(f, "{}.{}.{}", x, y, z),
            Version::Unknown => write!(f, "Unknown version"),
        }
    }
}
