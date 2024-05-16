use std::cmp::Ordering;
use std::ffi::OsStr;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::slice::SliceIndex;
use std::sync;
use std::sync::Arc;

use chrono::{DateTime, NaiveDateTime};

use crate::data::LogLevel;
use crate::error::LumberjackError;

#[derive(Clone)]
pub struct LogFile {
    pub path: PathBuf,
    pub timestamp: NaiveDateTime,
    pub level: LogLevel,
}

impl LogFile {
    pub fn with_path(path: PathBuf) -> crate::error::Result<LogFile> {
        // cbllog file names look like "cbl_info_1702053563433.cbllog"
        // First we strip the ".cbllog" extension
        let Some(file_name) = path
            .file_name()
            .and_then(OsStr::to_str)
            .and_then(|f| f.strip_suffix(".cbllog"))
        else {
            return Err(LumberjackError::FileNotLog(
                path.to_string_lossy().to_string(),
            ));
        };

        // Then we parse the timestamp from the end of the file name
        let Some(timestamp) = file_name
            .split('_')
            .last()
            .and_then(|t| t.parse::<i64>().ok())
            .and_then(DateTime::from_timestamp_millis)
        else {
            return Err(LumberjackError::ParseTimestampError {
                line: path.to_string_lossy().to_string().into_boxed_str(),
            });
        };

        let Some(level) = file_name.split('_').nth(1).and_then(LogLevel::from_str) else {
            return Err(LumberjackError::FileNotLog(file_name.to_string()));
        };

        Ok(LogFile {
            path,
            timestamp: timestamp.naive_utc(),
            level,
        })
    }
}

impl Hash for LogFile {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.timestamp.hash(state);
        self.level.hash(state)
    }
}

impl PartialEq for LogFile {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp.eq(&other.timestamp) && self.level.eq(&other.level)
    }
}
impl Eq for LogFile {}

impl PartialOrd<Self> for LogFile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.timestamp.cmp(&other.timestamp))
    }
}

impl Ord for LogFile {
    fn cmp(&self, other: &Self) -> Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}

impl Debug for LogFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogFile")
            .field("path", &self.path)
            .field("timestamp", &self.timestamp)
            .field("level", &self.level)
            .finish()
    }
}
