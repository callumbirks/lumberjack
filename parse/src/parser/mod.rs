use semver::Version;
use std::path::Path;
use std::str::FromStr;

use crate::data::{Database, File, Line, Object};
use crate::{Error, Result};

mod dir;
mod file;
mod model;

pub use dir::DirParser;
pub use file::FileParser;

pub trait Parser {
    async fn parse(path: impl AsRef<Path>) -> Result<(Vec<File>, Vec<Line>, Vec<Object>)>;
}

pub struct CBLVersion {
    pub platform: Platform,
    pub version: Version,
    pub system: String,
    pub build: u16,
    pub commit: String,
}

mod types {
    struct DB;
    struct Repl;
    struct Pusher;
    struct Puller;
}

pub enum Platform {
    C,
    Swift,
    Java,
    Android,
    DotNET,
}

impl FromStr for Platform {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            ".NET" => Ok(Platform::DotNET),
            "Swift" => Ok(Platform::Swift),
            "Java" => Ok(Platform::Java),
            "Android" => Ok(Platform::Android),
            "C" => Ok(Platform::C),
            _ => Err(Error::NoMatches),
        }
    }
}
