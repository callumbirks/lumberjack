use std::path::PathBuf;

use chrono::NaiveDateTime;
use sqlx::database::{HasArguments, HasValueRef};
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::Sqlite;

#[derive(sqlx::FromRow)]
pub struct Line {
    line_num: u64,
    message: String,
    #[sqlx(flatten)]
    file: File,
    #[sqlx(flatten)]
    event: Event,
}

#[derive(sqlx::FromRow)]
pub struct File {
    path: PathBuf,
    timestamp: NaiveDateTime,
    level: Level,
}

#[derive(sqlx::Type)]
#[repr(i32)]
pub enum Level {
    Info,
    Verbose,
    Debug,
    Warn,
    Error,
}

#[derive(sqlx::FromRow)]
pub struct Object {
    id: i64,
    ty: ObjectType,
    #[sqlx(flatten)]
    lines: Vec<Line>,
    #[sqlx(flatten)]
    events: Vec<Event>,
}

#[derive(sqlx::Type)]
#[repr(i32)]
pub enum ObjectType {
    DB,
    Repl,
    Pusher,
    Puller,
}

#[derive(sqlx::FromRow)]
pub struct Event {
    event_type: EventType,
    timestamp: NaiveDateTime,
    #[sqlx(flatten)]
    objects: Vec<Object>,
}

#[repr(i32)]
pub enum EventType {
    Common(CommonEvent),
    DB(DBEvent),
}

crate::impl_sqlx_type!(<Sqlite> EventType as i64);

#[derive(sqlx::Type)]
#[repr(i32)]
pub enum CommonEvent {
    Created,
    Destroyed,
}

#[derive(sqlx::Type)]
#[repr(i32)]
pub enum DBEvent {
    Opening,
    TxBegin,
    TxCommit,
    TxEnd,
    TxAbort,
}

#[derive(sqlx::Type)]
#[repr(i32)]
enum PgEventType {
    Common,
    DB,
}
