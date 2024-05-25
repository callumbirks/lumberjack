pub mod repl;

use crate::data::util::{diesel_tosql_transmute, impl_display_debug};
use crate::schema::{files, lines, objects};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{sql_types, AsExpression, FromSqlRow};
use std::hash::{Hash, Hasher};

#[derive(Insertable, Identifiable, Queryable, Selectable, Associations, Debug, Clone)]
#[diesel(primary_key(level, line_num))]
#[diesel(belongs_to(Object))]
#[diesel(belongs_to(File))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Line {
    pub level: Level,
    pub line_num: i64,
    pub timestamp: NaiveDateTime,
    pub message: String,
    pub event_type: EventType,
    pub object_id: i32,
    pub file_id: i32,
}

#[derive(Insertable, Identifiable, Queryable, Selectable, PartialEq, Eq, Debug, Clone)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Object {
    pub id: i32,
    pub ty: ObjectType,
}

#[derive(Insertable, Identifiable, Queryable, Selectable, Debug, Clone)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct File {
    pub id: i32,
    pub path: String,
    pub level: Level,
    pub timestamp: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub enum ObjectExtra {
    None,
    Repl(Box<repl::Repl>),
}

#[derive(AsExpression, FromSqlRow, Hash, Debug, Copy, Clone, Eq, PartialEq)]
#[repr(i32)]
#[diesel(sql_type = sql_types::Integer)]
pub enum Level {
    Info,
    Verbose,
    Debug,
    Warn,
    Error,
}

impl_display_debug!(Level);
diesel_tosql_transmute!(Level, i32, sql_types::Integer);

#[derive(AsExpression, FromSqlRow, Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(i32)]
#[diesel(sql_type = sql_types::Integer)]
pub enum ObjectType {
    DB,
    Repl,
    Pusher,
    Puller,
}

impl_display_debug!(ObjectType);
diesel_tosql_transmute!(ObjectType, i32, sql_types::Integer);

#[derive(AsExpression, FromSqlRow, Debug, Copy, Clone, Eq, PartialEq)]
#[repr(i16)]
#[diesel(sql_type = sql_types::Integer)]
pub enum EventType {
    None,
    Common(CommonEvent),
    DB(DBEvent),
}

diesel_tosql_transmute!(EventType, i32, sql_types::Integer);

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(i16)]
pub enum CommonEvent {
    Created,
    Destroyed,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(i16)]
pub enum DBEvent {
    Opening,
    TxBegin,
    TxCommit,
    TxEnd,
    TxAbort,
}

impl Object {
    pub fn name(&self) -> String {
        format!("{}#{}", self.ty, self.id)
    }
}

impl Hash for Object {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq<Self> for Line {
    fn eq(&self, other: &Self) -> bool {
        self.level == other.level && self.line_num == other.line_num
    }
}

impl Eq for Line {}

impl PartialOrd<Self> for Line {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.timestamp.partial_cmp(&other.timestamp)
    }
}

impl Ord for Line {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}
