use crate::data::util::{diesel_tosql_transmute, impl_display_debug};
use crate::parser::regex_patterns::LevelNames;
use crate::schema::{files, lines};
use crate::Result;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{sql_types, AsExpression, FromSqlRow};
pub use events::*;
use serde::Serialize;
use std::hash::Hash;

mod events {
    include!(concat!(env!("OUT_DIR"), "/events.rs"));
}

#[derive(
    Insertable, Serialize, Identifiable, Queryable, Selectable, Associations, Debug, Clone,
)]
#[diesel(primary_key(file_id, line_num))]
#[diesel(belongs_to(File))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Line {
    pub file_id: i32,
    pub line_num: i32,
    pub level: Level,
    pub timestamp: NaiveDateTime,
    pub domain: String,
    pub event_type: EventType,
    pub event_data: Option<String>,
    pub object_path: Option<String>,
}

#[derive(Insertable, Identifiable, Queryable, Selectable, Serialize, Debug, Clone)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct File {
    pub id: i32,
    pub path: String,
    pub level: Option<Level>,
    pub timestamp: NaiveDateTime,
}

#[derive(AsExpression, FromSqlRow, Serialize, Hash, Debug, Copy, Clone, Eq, PartialEq)]
#[repr(i32)]
#[diesel(sql_type = sql_types::Integer)]
pub enum Level {
    Error,
    Warning,
    Info,
    Verbose,
    Debug,
}

impl_display_debug!(Level);
diesel_tosql_transmute!(Level, i32, sql_types::Integer);

impl PartialEq<Self> for Line {
    fn eq(&self, other: &Self) -> bool {
        self.level == other.level && self.line_num == other.line_num
    }
}

impl Eq for Line {}

impl PartialOrd<Self> for Line {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Line {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}

impl Level {
    pub fn from_str(s: &str, level_names: &LevelNames) -> Result<Self> {
        match s {
            s if s == level_names.error => Ok(Self::Error),
            s if s == level_names.warn => Ok(Self::Warning),
            s if s == level_names.info => Ok(Self::Info),
            s if s == level_names.verbose => Ok(Self::Verbose),
            s if s == level_names.debug => Ok(Self::Debug),
            _ => Err(crate::Error::NoSuchLevel(s.to_string())),
        }
    }
}
