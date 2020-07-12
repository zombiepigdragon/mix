use std::path::PathBuf;
use thiserror::Error;

/// Errors that can be produced by mix.
#[derive(Debug, Error)]
pub enum MixError {
    /// The package(s) were not in the database.
    #[error("Package not found")]
    PackageNotFound,
    /// The package(s) need to be installed, but were not.
    #[error("Package not installed")]
    PackageNotInstalled,
    /// The requested file was not found.
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),
    /// An IOError that does not recieve special treatment occurred.
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    /// An error when serializing or deserializing.
    #[error(transparent)]
    SerializationError(#[from] serde_cbor::error::Error),
    /// The manifest parsed successfully but contained invalid information.
    #[error("Invalid manifest type {0}.")]
    InvalidManifestError(toml::Value),
    /// The manifest failed to parse.
    #[error(transparent)]
    ManifestParseError(#[from] toml::de::Error),
}
