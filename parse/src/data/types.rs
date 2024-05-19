mod repl;

use crate::data::impl_sqlx_type;
use crate::data::util::impl_display_debug;
use chrono::NaiveDateTime;
use sqlx::database::{HasArguments, HasValueRef};
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::sqlite::SqliteRow;
use sqlx::{Error, Row, Sqlite};
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::path::PathBuf;
use std::str::FromStr;

pub use repl::Collection as ReplCollection;
pub use repl::Config as ReplConfig;
pub use repl::Mode as ReplMode;
pub use repl::Repl;

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Line {
    pub level: Level,
    pub line_num: u32,
    pub timestamp: NaiveDateTime,
    pub message: String,
    pub event_type: EventType,
    #[sqlx(flatten)]
    pub object: Object,
    #[sqlx(flatten)]
    pub file: File,
}

#[derive(sqlx::Type, Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u16)]
pub enum Level {
    Info,
    Verbose,
    Debug,
    Warn,
    Error,
}

impl_display_debug!(Level);

#[derive(sqlx::FromRow, Debug, Clone, Eq, PartialEq)]
pub struct Object {
    pub id: u32,
    pub type_: ObjectType,
    #[sqlx(skip)]
    pub extra: ObjectExtra,
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub enum ObjectExtra {
    #[default]
    None,
    Repl(Box<Repl>),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct File {
    pub path: PathBuf,
}

impl<'r> sqlx::FromRow<'r, SqliteRow> for File {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        let path_str: String = row.get("path");
        Ok(Self {
            path: PathBuf::from_str(&path_str).unwrap(),
        })
    }
}

#[derive(sqlx::Type, Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u16)]
pub enum ObjectType {
    DB,
    Repl,
    Pusher,
    Puller,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u16)]
pub enum EventType {
    Common(CommonEvent),
    DB(DBEvent),
}

impl_sqlx_type!(<Sqlite> EventType as u32);

#[derive(sqlx::Type, Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u16)]
pub enum CommonEvent {
    Created,
    Destroyed,
}

#[derive(sqlx::Type, Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u16)]
pub enum DBEvent {
    Opening,
    TxBegin,
    TxCommit,
    TxEnd,
    TxAbort,
}

impl Object {
    pub fn name(&self) -> String {
        format!("{}#{}", self.type_, self.id)
    }
}

impl Display for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut builder = f.debug_struct("Object");
        builder.field("id", &self.id);
        builder.field("type", &self.type_);
        match &self.extra {
            ObjectExtra::None => {}
            ObjectExtra::Repl(r) => {
                builder.field("repl", r.as_ref());
            }
        }
        builder.finish()
    }
}

impl Display for ObjectType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        <ObjectType as Debug>::fmt(self, f)
    }
}

impl PartialEq<Self> for Line {
    fn eq(&self, other: &Self) -> bool {
        self.level == other.level && self.line_num == other.line_num
    }
}

impl Eq for Line {}

impl PartialOrd<Self> for Line {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.timestamp.partial_cmp(&other.timestamp)
    }
}

impl Ord for Line {
    fn cmp(&self, other: &Self) -> Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}
