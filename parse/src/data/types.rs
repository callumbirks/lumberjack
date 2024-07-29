pub mod event;
mod object;

use crate::data::util::{diesel_tosql_transmute, impl_display_debug};
use crate::schema::{files, lines, objects};
use crate::Error;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{sql_types, AsExpression, FromSqlRow};
use serde::Serialize;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

#[derive(
    Insertable, Serialize, Identifiable, Queryable, Selectable, Associations, Debug, Clone,
)]
#[diesel(primary_key(file_id, line_num))]
#[diesel(belongs_to(Object))]
#[diesel(belongs_to(File))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Line {
    pub file_id: i32,
    pub line_num: i64,
    pub level: Level,
    pub timestamp: NaiveDateTime,
    pub domain: Domain,
    pub event_type: EventType,
    pub event_data: Option<String>,
    pub object_id: i32,
}

#[derive(
    Insertable, Identifiable, Queryable, Selectable, Serialize, PartialEq, Eq, Debug, Clone,
)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Object {
    pub id: i32,
    pub object_type: ObjectType,
    // JSON
    pub data: Option<String>,
}

#[derive(Insertable, Identifiable, Queryable, Selectable, Serialize, Debug, Clone)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct File {
    pub id: i32,
    pub path: String,
    pub level: Option<Level>,
    pub timestamp: NaiveDateTime,
}

#[derive(Serialize, PartialEq, Eq, Debug, Clone)]
pub struct Event {
    pub event_type: EventType,
    pub data: Option<String>,
}

#[derive(AsExpression, FromSqlRow, Serialize, Hash, Debug, Copy, Clone, Eq, PartialEq)]
#[repr(i32)]
#[diesel(sql_type = sql_types::Integer)]
pub enum Domain {
    Actor,
    BLIP,
    DB,
    Sync,
    Query,
    WS,
}

impl_display_debug!(Domain);
diesel_tosql_transmute!(Domain, i32, sql_types::Integer);

#[derive(AsExpression, FromSqlRow, Serialize, Hash, Debug, Copy, Clone, Eq, PartialEq)]
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

#[derive(AsExpression, FromSqlRow, Serialize, Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(i32)]
#[diesel(sql_type = sql_types::Integer)]
pub enum ObjectType {
    DB,
    StaticDB,
    Repl,
    Pusher,
    Puller,
    Inserter,
    BLIPIO,
    IncomingRev,
    Connection,
    C4SocketImpl,
    RevFinder,
    ReplicatorChangesFeed,
    QueryEnum,
    C4Replicator,
    Housekeeper,
    Shared,
    CollectionImpl,
    Query,
    DBAccess,
}

impl_display_debug!(ObjectType);
diesel_tosql_transmute!(ObjectType, i32, sql_types::Integer);

#[derive(AsExpression, FromSqlRow, Serialize, Debug, Copy, Clone, Eq, PartialEq)]
#[repr(i32)]
#[diesel(sql_type = sql_types::Integer)]
pub enum EventType {
    None,
    // Common Events
    Created,
    Destroyed,
    // Database Events
    DBOpening,
    DBUpgrade,
    DBTxBegin,
    DBTxCommit,
    DBTxAbort,
    DBSavedRev,
    // Query Events
    QueryCreateIndex,
    // Subrepl Events
    SubreplStart,
    PullerHandledRevs,
    // BLIP Events
    BLIPSendRequestStart,
    BLIPQueueRequest,
    BLIPWSWriteStart,
    BLIPSendFrame,
    BLIPSendRequestEnd,
    BLIPWSWriteEnd,
    BLIPReceiveFrame,
    // Housekeeper Events
    HousekeeperMonitoring,
    // Repl Events
    ReplConflictScan,
    ReplConnected,
    ReplActivityUpdate,
    ReplStatusUpdate,
    ReplStart,
}

diesel_tosql_transmute!(EventType, i32, sql_types::Integer);
impl_display_debug!(EventType);

impl Object {
    pub fn name(&self) -> String {
        format!("{}#{}", self.object_type, self.id)
    }
}

// Hashed by `id`, because this is always unique across objects.
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

impl FromStr for Level {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "info" => Ok(Self::Info),
            "warning" => Ok(Self::Warn),
            "debug" => Ok(Self::Debug),
            "verbose" => Ok(Self::Verbose),
            "error" => Ok(Self::Error),
            _ => Err(crate::Error::NoSuchLevel(s.to_string())),
        }
    }
}

impl FromStr for ObjectType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "DB" => Ok(ObjectType::DB),
            "Repl" | "repl" => Ok(ObjectType::Repl),
            "Pusher" => Ok(ObjectType::Pusher),
            "Puller" => Ok(ObjectType::Puller),
            "Inserter" => Ok(ObjectType::Inserter),
            "BLIPIO" => Ok(ObjectType::BLIPIO),
            "IncomingRev" => Ok(ObjectType::IncomingRev),
            "Connection" => Ok(ObjectType::Connection),
            "C4SocketImpl" => Ok(ObjectType::C4SocketImpl),
            "RevFinder" => Ok(ObjectType::RevFinder),
            "ReplicatorChangesFeed" => Ok(ObjectType::ReplicatorChangesFeed),
            "QueryEnum" => Ok(ObjectType::QueryEnum),
            "C4Replicator" => Ok(ObjectType::C4Replicator),
            "Housekeeper" => Ok(ObjectType::Housekeeper),
            "Shared" => Ok(ObjectType::Shared),
            "CollectionImpl" => Ok(ObjectType::CollectionImpl),
            "Query" => Ok(ObjectType::Query),
            "DBAccess" => Ok(ObjectType::DBAccess),
            _ => Err(Error::UnknownObject(s.to_string())),
        }
    }
}

impl FromStr for Domain {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Actor" => Ok(Domain::Actor),
            "BLIP" => Ok(Domain::BLIP),
            "DB" => Ok(Domain::DB),
            "Sync" => Ok(Domain::Sync),
            "Query" => Ok(Domain::Query),
            "WS" => Ok(Domain::WS),
            _ => Err(Error::UnknownDomain(s.to_string())),
        }
    }
}
