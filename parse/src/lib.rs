pub mod data;
pub(crate) mod decoder;
mod error;
mod parser;
#[rustfmt::skip]
pub mod schema;
pub mod util;

use crate::data::open_db;
use crate::data::EventType;
use crate::parser::Parser;
use diesel::prelude::*;
use diesel::{Connection, RunQueryDsl};
pub use error::{Error, Result};
use serde::Serialize;
use std::path::Path;

/// Parse logs from the given `in_path` into a SQLite database at the given `out_path`.
pub fn parse(in_path: &Path, out_path: &Path) -> Result<()> {
    log::info!("Parsing logs at {:?}", in_path);

    let mut conn = open_db(out_path, true)?;

    let parser = Parser::new(in_path)?;

    let mut total_files = 0_u64;
    let mut total_lines = 0_u64;

    let event_types = enum_iterator::all::<EventType>()
        .map(InsertableEventType::from)
        .collect::<Vec<_>>();

    diesel::insert_into(schema::event_types::table)
        .values(event_types)
        .execute(&mut conn)?;

    for result in parser.parse() {
        total_files += 1;
        total_lines += result.lines.len() as u64;
        conn.transaction(|tx| {
            diesel::insert_or_ignore_into(schema::files::table)
                .values(result.file)
                .execute(tx)?;
            diesel::insert_into(schema::lines::table)
                .values(result.lines)
                .execute(tx)
        })?;
    }

    log::info!(
        "Parsing complete. Parsed {} files, {} lines",
        total_files,
        total_lines,
    );

    log::info!("Wrote parsed data to {:?}", out_path);

    Ok(())
}

#[derive(Insertable, Serialize, Identifiable, Debug, Clone)]
#[diesel(table_name = schema::event_types)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
struct InsertableEventType {
    pub id: i32,
    pub name: String,
}

impl From<EventType> for InsertableEventType {
    fn from(value: EventType) -> Self {
        Self {
            id: unsafe { std::mem::transmute::<EventType, i32>(value) },
            name: value.to_string(),
        }
    }
}
