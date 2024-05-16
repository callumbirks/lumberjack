use chrono::NaiveDateTime;
use sqlx::database::{HasArguments, HasValueRef};
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::postgres::{PgRow, PgTypeInfo};
use sqlx::{Error, FromRow, Postgres, Row};
use std::path::PathBuf;

#[derive(sqlx::FromRow)]
pub struct Line {
    #[sqlx(flatten)]
    file: File,
    line_num: u64,
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
pub struct Event {
    event_type: EventType,
    timestamp: NaiveDateTime,
}

#[repr(i32)]
pub enum EventType {
    Common(CommonEvent),
    DB(DBEvent),
}

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

impl sqlx::Type<Postgres> for EventType {
    fn type_info() -> <Postgres as sqlx::Database>::TypeInfo {
        <i64 as sqlx::Type<Postgres>>::type_info()
    }
}

impl sqlx::Encode<'_, Postgres> for EventType {
    fn encode_by_ref(&self, buf: &mut <Postgres as HasArguments<'_>>::ArgumentBuffer) -> IsNull {
        let int: i64 = unsafe { std::mem::transmute(self) };
        <i64 as sqlx::Encode<Postgres>>::encode_by_ref(&int, buf)
    }
}

impl sqlx::Decode<'_, Postgres> for EventType {
    fn decode(value: <Postgres as HasValueRef<'_>>::ValueRef) -> Result<Self, BoxDynError> {
        <i64 as sqlx::Decode<Postgres>>::decode(value).map(|i| unsafe { std::mem::transmute(&i) })
    }
}
