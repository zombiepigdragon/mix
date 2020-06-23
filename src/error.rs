/// The errors than can come from the package manager.
#[derive(Debug, PartialEq)]
pub enum Error {
    /// A package was needed but was not found in the cache.
    PackageNotFound,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::PackageNotFound => "Package Not Found",
            }
        )
    }
}
