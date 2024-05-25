use crate::data::{File, Line, Object};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
pub use dir::DirParserModel;

use crate::Result;

mod dir;

pub trait Model {
    fn from_version_string(line: &str) -> Result<Box<Self>>;
    fn parse_timestamp(&self, line: &str) -> Result<Timestamp>;
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
