pub mod data;
pub(crate) mod decoder;
mod error;
mod parser;
pub mod schema;
pub mod util;

use crate::data::{open_db, Object};
use crate::parser::Parser;
use diesel::{Connection, RunQueryDsl, SqliteConnection};
pub use error::{Error, Result};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ParserOptions<'a> {
    in_path: &'a Path,
    out_path: Option<&'a Path>,
}

impl<'a> ParserOptions<'a> {
    fn new(in_path: &'a Path) -> ParserOptions<'a> {
        Self {
            in_path,
            out_path: None,
        }
    }

    pub fn out_db(mut self, out_path: &'a Path) -> ParserOptions<'a> {
        self.out_path = Some(out_path);
        self
    }

    pub fn execute(self) -> Result<SqliteConnection> {
        _parse(self)
    }
}

pub fn parse(in_path: &Path) -> ParserOptions {
    ParserOptions::new(in_path)
}

/// Parse logs from the given path (either a file or a directory) and return the populated database.
fn _parse<'a>(options: ParserOptions<'a>) -> Result<SqliteConnection> {
    log::info!("Starting parse with options: {:?}", &options);

    let hash = {
        let mut hasher = DefaultHasher::new();
        options.in_path.hash(&mut hasher);
        hasher.finish()
    };

    // The database path is the hash of the log path in hexadecimal (with 'sqlite' extension)
    let db_path = format!("{hash:x}.sqlite");
    let mut conn = open_db(db_path, true)?;

    let parser = Parser::new(options.in_path)?;

    let mut total_files = 0_u64;
    let mut total_lines = 0_u64;
    let mut total_objects = 0_u64;

    for result in parser.parse() {
        total_files += 1;
        total_lines += result.lines.len() as u64;
        total_objects += result.objects.len() as u64;
        log::debug!(
            "Inserting 1 file, {} lines, {} objects into the database",
            result.lines.len(),
            result.objects.len(),
        );
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

    Ok(conn)
}
