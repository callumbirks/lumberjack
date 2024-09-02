use crate::data::util::impl_display_debug;
use crate::parser::regex_patterns::LevelNames;
use crate::{Error, Result};
use chrono::NaiveDateTime;
pub use events::*;
use rusqlite::{params, Transaction};
use serde::Serialize;
use std::hash::Hash;

mod events {
    include!(concat!(env!("OUT_DIR"), "/events.rs"));
}

#[derive(Debug, Clone, Serialize)]
pub struct Line {
    pub file_id: u32,
    pub line_num: u32,
    pub level: Level,
    pub timestamp: NaiveDateTime,
    pub domain: String,
    pub event_type: EventType,
    pub event_data: Option<String>,
    pub object_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct File {
    pub id: u32,
    pub path: String,
    pub timestamp: NaiveDateTime,
}

#[derive(Hash, Debug, Copy, Clone, Eq, PartialEq, Serialize)]
#[repr(u32)]
pub enum Level {
    Error,
    Warning,
    Info,
    Verbose,
    Debug,
}

impl_display_debug!(Level);

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

impl From<u32> for Level {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::Error,
            1 => Self::Warning,
            2 => Self::Info,
            3 => Self::Verbose,
            4 => Self::Debug,
            _ => panic!("Invalid level value: {}", value),
        }
    }
}

impl From<u32> for EventType {
    fn from(value: u32) -> Self {
        assert!(
            value < enum_iterator::cardinality::<EventType>() as u32,
            "Invalid event type value: {}",
            value
        );
        unsafe { std::mem::transmute::<u32, EventType>(value) }
    }
}

pub trait Insertable {
    fn db_insert(self, tx: &mut Transaction) -> Result<()>;
}

pub trait FromRow: Sized {
    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self>;
}

impl<T, It> Insertable for It
where
    for<'a> &'a T: Insertable,
    It: Iterator<Item = T>,
{
    fn db_insert(self, tx: &mut Transaction) -> Result<()> {
        for item in self {
            item.db_insert(tx)?;
        }
        Ok(())
    }
}

impl Insertable for &Line {
    fn db_insert(self, tx: &mut Transaction) -> Result<()> {
        tx.execute(
            "
            INSERT INTO lines
                (file_id, line_num, level, timestamp, domain, event_type, event_data, object_path)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            params![
                self.file_id,
                self.line_num,
                self.level as u32,
                self.timestamp,
                self.domain,
                self.event_type as u32,
                self.event_data,
                self.object_path,
            ],
        )
        .map_err(Error::Sqlite)
        .map(|_| ())
    }
}

impl Insertable for &File {
    fn db_insert(self, tx: &mut Transaction) -> Result<()> {
        tx.execute(
            "
            INSERT INTO files
                (id, path, timestamp)
            VALUES ($1, $2, $3)",
            params![self.id, self.path, self.timestamp],
        )
        .map_err(Error::Sqlite)
        .map(|_| ())
    }
}

impl Insertable for &EventType {
    fn db_insert(self, tx: &mut Transaction) -> Result<()> {
        //id: unsafe { std::mem::transmute::<EventType, i32>(value) },
        //name: value.to_string(),
        tx.execute(
            "
            INSERT INTO event_types
                (id, name)
            VALUES ($1, $2)",
            params![*self as u32, self.to_string()],
        )
        .map_err(Error::Sqlite)
        .map(|_| ())
    }
}

impl FromRow for File {
    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            path: row.get(1)?,
            timestamp: row.get(2)?,
        })
    }
}

impl FromRow for Line {
    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            file_id: row.get(0)?,
            line_num: row.get(1)?,
            level: Level::from(row.get::<_, u32>(2)?),
            timestamp: row.get(3)?,
            domain: row.get(4)?,
            event_type: EventType::from(row.get::<_, u32>(5)?),
            event_data: row.get(6)?,
            object_path: row.get(7)?,
        })
    }
}
