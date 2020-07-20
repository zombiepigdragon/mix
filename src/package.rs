use crate::error::MixError;
use serde::{Deserialize, Serialize};

/// A singular package. A package is a name, list of files, and some metadata.
/// The metadata is what allows retrieving a package, viewing the files of a package, and many similar actions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Package {
    /// The package's name.
    pub name: String,
    /// The package's version.
    pub version: Version,
    /// The installation state of the package.
    pub state: InstallState,
}

impl Package {
    /// Provide a package from its toml metadata
    pub fn from_toml(data: &str) -> Result<Self, MixError> {
        let metadata = match data.parse::<toml::Value>() {
            Ok(toml::Value::Table(metadata)) => metadata,
            Ok(value) => return Err(MixError::InvalidManifestError(value)),
            Err(error) => return Err(MixError::ManifestParseError(error)),
        };
        let name = if let toml::Value::String(name) = metadata["name"].clone() {
            name
        } else {
            return Err(MixError::InvalidManifestError(metadata["name"].clone()));
        };
        let version = Version::Unknown;
        Ok(Package {
            name,
            version,
            state: InstallState::Uninstalled,
        })
    }

    /// Mark the package as manually installed. This does *not* install it.
    pub fn mark_as_manually_installed(&mut self) {
        self.state = InstallState::Manual;
    }

    /// Install the package.
    /// # Todo
    /// This does nothing whatsoever. It should place files in the correct locations.
    pub fn install(&mut self) {
        ()
    }

    /// Remove the package.
    /// # Todo
    /// This should remove files from the filesystem.
    pub fn remove(&mut self) {
        self.state = InstallState::Uninstalled;
    }

    /// Update the package.
    /// # Todo
    /// This only increments the major version by one, it should actually work instead.
    pub fn update(&mut self) {
        self.version = match self.version {
            Version::SemVer(maj, min, rev) => Version::SemVer(maj + 1, min, rev),
            Version::Unknown => Version::SemVer(1, 0, 0),
        };
    }

    /// Download the files for the package.
    /// # Todo
    /// This API doesn't even make sense, that should be fixed.
    pub fn fetch(
        &self,
        client: &reqwest::blocking::Client,
        url: &str,
        file: &mut std::fs::File,
    ) -> Result<(), MixError> {
        let mut data = client.get(url).send()?;
        data.copy_to(file)?;
        Ok(())
    }
}

impl std::fmt::Display for Package {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:\t{};\t{}", self.name, self.version, self.state)
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
