use std::io;
use std::io::Error;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum LumberjackError {
    #[error("Did not open a valid directory")]
    DirectoryInvalid,
    #[error("File {0} is not a log file")]
    FileNotLog(String),
    #[error("No matches for {0}")]
    NoMatches(Box<str>),
    #[error("Unable to parse timestamp from line {line}")]
    ParseTimestampError { line: Box<str> },
    #[error("Parsing error {0}")]
    ParseError(String),
    #[error("IO Error")]
    Io(io::ErrorKind),
    #[error("Async Task join error {0}")]
    TokioJoin(String),
    #[error("Grep regex error")]
    GrepRegex(#[from] grep::regex::Error),
    #[error("Regex error")]
    Regex(#[from] regex::Error),
}

impl From<io::Error> for LumberjackError {
    fn from(err: Error) -> Self {
        Self::Io(err.kind())
    }
}

impl From<tokio::task::JoinError> for LumberjackError {
    fn from(err: tokio::task::JoinError) -> Self {
        Self::TokioJoin(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, LumberjackError>;
