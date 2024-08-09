use semver::Version;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("DB Connection Error {0}")]
    DbConn(#[from] diesel::ConnectionError),
    #[error("Diesel Error {0}")]
    Diesel(#[from] diesel::result::Error),
    #[error("IO Error {0}")]
    Io(#[from] std::io::Error),
    #[error("Regex Error {0}")]
    Regex(#[from] regex::Error),
    #[error("Semver Error {0}")]
    Semver(#[from] semver::Error),
    #[error("chrono parse Error {0}")]
    ChronoParse(#[from] chrono::ParseError),
    #[error("Error {0}")]
    Boxed(#[from] Box<dyn std::error::Error + Send + Sync>),
    #[error("YAML Error {0}")]
    SerdeYaml(#[from] serde_yaml::Error),
    #[error("Parse Int Error {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("No valid logs at path \"{0}\"")]
    NotLogs(PathBuf),
    #[error("Unsupported CBL Version {0}")]
    UnsupportedVersion(Version),
    #[error("Unsupported Platform with identifying line '{0}'")]
    UnsupportedPlatform(String),
    #[error("Invalid Filename \"{0}\"")]
    InvalidFilename(String),
    #[error("No such log level '{0}'")]
    NoSuchLevel(String),
    #[error("No matches")]
    NoMatches,
    #[error("No event in line")]
    NoEvent,
    #[error("No parseable timestamp in line \"{0}\"")]
    NoTimestamp(String),
    #[error("Unknown object '{0}'")]
    UnknownObject(String),
    #[error("No domain in line")]
    NoDomain,
    #[error("No object in line")]
    NoObject,
    #[error("No such value {0} in enum {1}")]
    InvalidEnumValue(i32, &'static str),
    #[error("Invalid binary logs: '{0}' at {1}")]
    InvalidBinaryLogs(String, u64),
    #[error("Invalid varint in binary logs")]
    InvalidVarint,
    #[error("No Level in line")]
    NoLevel,
    #[error("Cannot parse: {0}")]
    CannotParse(String),
}

pub type Result<T> = std::result::Result<T, Error>;
