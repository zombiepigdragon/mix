use crate::{database::Database, error::MixError};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    ffi::OsString,
    fs::{create_dir, set_permissions, OpenOptions, Permissions},
    io::{self, prelude::*},
    os::unix::prelude::*,
    path::PathBuf,
    rc::Rc,
};
use tar::Archive;
use xz2::read::XzDecoder;

/// Install the given packages. This will place files onto the filesystem, and
/// mark the packages as installed (either as a dependency if not installed, or
/// leaving the state as dependency or manually installed.)
pub fn install(packages: &[Rc<RefCell<Package>>], database: &mut Database) -> Result<(), MixError> {
    for package in packages {
        // Make sure the package is known.
        database.import_package(package.clone())?;
        // Open the package tarball for reading.
        let file = database.open_package_tarball(&package.borrow())?;
        let file = XzDecoder::new(file);
        let mut file = Archive::new(file);
        // Place the files into the filesystem.
        for entry in file.entries()? {
            let mut entry = entry?;
            match entry.path()?.to_str() {
                Some(".MANIFEST") => continue,
                _ => place_entry(&mut entry)?,
            }
        }
        // Flag the package as installed.
        let package_state = match package.borrow().state {
            InstallState::Manual => InstallState::Manual,
            InstallState::Dependency | InstallState::Uninstalled => InstallState::Dependency,
        };
        package.borrow_mut().state = package_state;
    }
    Ok(())
}

/// Remove the given packages. This will remove any files of the package from
/// the filesystem, as well as marking the package as not installed.
/// # Warning
/// A call to this function that removes dependencies of installed packages but
/// not those packages will place the package database into an an unsafe state.
pub fn remove(packages: &[Rc<RefCell<Package>>], _database: &mut Database) -> Result<(), MixError> {
    for _package in packages {
        todo!()
    }
    Ok(())
}

/// Update the given packages to the latest version. This may skip over packages
/// that are already up to date.
pub fn update(packages: &[Rc<RefCell<Package>>], _database: &mut Database) -> Result<(), MixError> {
    for _package in packages {
        todo!()
    }
    Ok(())
}

/// Download the files of the given package.
pub fn fetch(_package: Rc<RefCell<Package>>) -> Result<(), MixError> {
    todo!()
}

/// A singular package. A package is a name, list of files, and some metadata.
/// The metadata is what allows retrieving a package, viewing the files of a package, and many similar actions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Package {
    /// The package's name.
    pub name: String,
    /// The package's version.
    pub version: Version,
    /// The installation state of the package.
    pub state: InstallState,
    /// The files included in the package.
    pub files: Vec<PathBuf>,
    /// The local path of the package, either relative to the package directory or absolute.
    pub local_path: Option<PathBuf>,
}

impl Package {
    /// Provide a package from a tarball
    pub fn from_tarball(file: impl Read) -> Result<Self, MixError> {
        let file = XzDecoder::new(file);
        let mut archive = Archive::new(file);
        let mut files = vec![];
        let mut manifest = None;
        for entry in archive.entries()? {
            let mut entry = entry?;
            if entry.path()? == OsString::from(".MANIFEST") {
                let mut buf = String::new();
                entry.read_to_string(&mut buf)?;
                manifest = Some(buf.parse::<toml::Value>());
            } else {
                files.push(PathBuf::from(entry.path()?))
            }
        }
        let manifest = match manifest {
            Some(manifest) => manifest,
            None => return Err(MixError::InvalidPackageError),
        };
        let metadata = match manifest {
            Ok(toml::Value::Table(metadata)) => metadata,
            Ok(value) => return Err(MixError::InvalidManifestError(value)),
            Err(error) => return Err(MixError::ManifestParseError(error)),
        };
        let name = if let toml::Value::String(name) = metadata["name"].clone() {
            name
        } else {
            return Err(MixError::InvalidManifestError(metadata["name"].clone()));
        };
        // TODO: Read a version out of the file.
        let version = Version::Unknown;
        Ok(Self {
            name,
            version,
            state: InstallState::Uninstalled,
            files,
            local_path: None,
        })
    }

    /// Provide the filename for the tarball of the package.
    pub fn get_filename(&self) -> PathBuf {
        PathBuf::from(format!("{}-{}.tar.xz", self.name, self.version))
    }

    /// Mark the package as manually installed. This does *not* install it.
    pub fn mark_as_manually_installed(&mut self) {
        self.state = InstallState::Manual;
    }
}

impl std::fmt::Display for Package {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Some fields are allowed to differ between two packages, such as the path.
impl PartialEq for Package {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.version == other.version
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
                Self::Manual => "Manually installed",
                Self::Dependency => "Dependency installation",
                Self::Uninstalled => "Not installed",
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
            (Self::SemVer(maj1, min1, rev1), Self::SemVer(maj2, min2, rev2)) => {
                if maj1 != maj2 {
                    maj1.cmp(maj2)
                } else if min1 != min2 {
                    min1.cmp(min2)
                } else if rev1 == rev2 {
                    Ordering::Equal
                } else {
                    rev1.cmp(rev2)
                }
            }
            (Self::SemVer(_, _, _), Self::Unknown) => Ordering::Greater,
            (Self::Unknown, Self::SemVer(_, _, _)) => Ordering::Less,
            (Self::Unknown, Self::Unknown) => Ordering::Equal,
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
            Self::SemVer(maj1, min1, rev1) => match other {
                Self::SemVer(maj2, min2, rev2) => maj1 == maj2 && min1 == min2 && rev1 == rev2,
                Self::Unknown => false,
            },
            Self::Unknown => match other {
                Self::SemVer(_, _, _) => false,
                Self::Unknown => true,
            },
        }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SemVer(x, y, z) => write!(f, "{}.{}.{}", x, y, z),
            Self::Unknown => write!(f, "Unknown version"),
        }
    }
}

/// The tar crate has been reported to not be designed for unpacking tar files,
/// opting for support of creating them instead. This will handle placing files
/// on disk, as well as ensuring permissions work out. If there's a way to do
/// this transparently through tar, feel free to open a PR with this replaced.
fn place_entry(entry: &mut tar::Entry<impl Read>) -> Result<(), MixError> {
    let path = PathBuf::from("/").join(entry.path()?);
    match entry.header().entry_type() {
        tar::EntryType::Directory => {
            if !path.exists() {
                let result = create_dir(&path);
                match result {
                    Ok(_) => {
                        // Set the permissions of the new directory
                        let mode = entry.header().mode()?;
                        let permissions = Permissions::from_mode(mode);
                        set_permissions(path, permissions)?;
                    }
                    Err(error) => return Err(error.into()),
                }
            }
        }
        tar::EntryType::Regular => {
            let result = OpenOptions::new().create_new(true).write(true).open(path);
            match result {
                Ok(mut file) => {
                    io::copy(entry, &mut file)?;
                }
                Err(error) => return Err(error.into()),
            }
        }
        tar::EntryType::Link => todo!(),
        tar::EntryType::Symlink => todo!(),
        other_type => unimplemented!("{:?}", other_type),
    }
    Ok(())
}
