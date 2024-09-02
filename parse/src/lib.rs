pub mod data;
pub(crate) mod decoder;
mod error;
mod parser;
pub mod util;

use crate::data::open_db;
use crate::data::{EventType, Insertable};
use crate::parser::Parser;
pub use error::{Error, Result};
use std::path::Path;

pub use crate::parser::Options;

/// Parse logs from the given `in_path` into a SQLite database at the given `out_path`.
/// Return the number of lines which failed to parse.
pub fn parse(in_path: &Path, out_path: &Path, options: Options) -> Result<u64> {
    log::info!("Parsing logs at {:?}", in_path);

    let mut conn = open_db(out_path, true)?;

    let parser = Parser::new(in_path, options)?;

    let mut total_files = 0_u64;
    let mut total_lines = 0_u64;

    {
        let mut tx = conn.transaction()?;
        enum_iterator::all::<EventType>().db_insert(&mut tx)?;
        tx.commit()?;
    }

    let mut error_count = 0_u64;

    for result in parser.parse() {
        error_count += result.error_count;
        total_files += 1;
        total_lines += result.lines.len() as u64;
        let mut tx = conn.transaction()?;
        result.file.db_insert(&mut tx)?;
        result.lines.into_iter().db_insert(&mut tx)?;
        tx.commit()?;
    }

    log::info!(
        "Parsing complete. Parsed {} files, {} lines",
        total_files,
        total_lines,
    );

    log::info!("Wrote parsed data to {:?}", out_path);

    Ok(error_count)
}
