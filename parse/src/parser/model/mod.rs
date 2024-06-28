use crate::data::{File, Line, Object};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
pub use dir::DirParserModel;
use std::fmt::{Display, Formatter};

use crate::Result;

mod dir;

pub trait Model {
    fn from_version_string(line: &str) -> Result<Box<Self>>;
    fn parse_timestamp(&self, line: &str) -> Result<Timestamp>;
    /// Parse the `Line` and `Object` from a given log line string. `file` and `base_date` are
    /// provided to assist with additional data, such as log level or timestamp.
    /// # Errors
    /// If a valid `Line` or `Object` cannot be parsed from the line.
    fn parse_line(
        &self,
        line: &str,
        line_num: usize,
        file: &File,
        base_date: NaiveDate,
    ) -> Result<(Line, Object)>;
}

pub enum Timestamp {
    DateTime(NaiveDateTime),
    Time(NaiveTime),
}

pub struct Compatibility {
    version_range: std::ops::Range<semver::Version>,
}

impl Compatibility {
    pub fn with_versions(range: std::ops::Range<semver::Version>) -> Self {
        Self {
            version_range: range,
        }
    }
}

impl Display for Compatibility {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let start = &self.version_range.start;
        let end = &self.version_range.end;
        write!(
            f,
            "{{ {}.{}.{} <= Version < {}.{}.{} }}",
            start.major, start.minor, start.patch, end.major, end.minor, end.patch
        )
    }
}
