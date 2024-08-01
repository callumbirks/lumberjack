pub mod data;
pub(crate) mod decoder;
mod error;
mod parser;
#[rustfmt::skip]
pub mod schema;
pub mod util;

use crate::data::{open_db, Object};
use crate::parser::Parser;
use diesel::{Connection, RunQueryDsl};
pub use error::{Error, Result};
use std::path::Path;

/// Parse logs from the given `in_path` into a SQLite database at the given `out_path`.
pub fn parse(in_path: &Path, out_path: &Path) -> Result<()> {
    log::info!("Parsing logs at {:?}", in_path);

    let mut conn = open_db(out_path, true)?;

    let parser = Parser::new(in_path)?;

    let mut total_files = 0_u64;
    let mut total_lines = 0_u64;
    let mut total_objects = 0_u64;

    for result in parser.parse() {
        total_files += 1;
        total_lines += result.lines.len() as u64;
        total_objects += result.objects.len() as u64;
        let objects: Vec<Object> = result.objects.into_iter().collect();
        conn.transaction(|tx| {
            diesel::insert_or_ignore_into(schema::files::table)
                .values(result.file)
                .execute(tx)?;
            diesel::insert_or_ignore_into(schema::objects::table)
                .values(objects)
                .execute(tx)?;
            diesel::insert_into(schema::lines::table)
                .values(result.lines)
                .execute(tx)
        })?;
    }

    log::info!(
        "Parsing complete. Parsed {} files, {} lines, {} objects",
        total_files,
        total_lines,
        total_objects
    );

    log::info!("Wrote parsed data to {:?}", out_path);

    Ok(())
}
