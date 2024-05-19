use semver::Version;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO Error {0}")]
    Io(#[from] std::io::Error),
    #[error("Regex Error {0}")]
    Regex(#[from] regex::Error),
    #[error("YAML Error {0}")]
    SerdeYaml(#[from] serde_yaml::Error),
    #[error("Parse Int Error {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("No valid logs at path {0}")]
    NotLogs(PathBuf),
    #[error("Unsupported CBL Version {0}")]
    UnsupportedVersion(Version),
    #[error("No matches")]
    NoMatches,
}

pub type Result<T> = std::result::Result<T, Error>;
