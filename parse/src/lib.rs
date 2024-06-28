pub mod data;
mod error;
mod parser;
pub mod schema;
pub mod util;

use crate::data::open_db;
use crate::parser::{DirParser, FileParser, Parser};
use diesel::{Connection, Insertable, RunQueryDsl, SqliteConnection};
pub use error::{Error, Result};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

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

    pub async fn execute(mut self) -> Result<SqliteConnection> {
        _parse(self).await
    }
}

pub fn parse(in_path: &Path) -> ParserOptions {
    ParserOptions::new(in_path)
}

/// Parse logs from the given path (either a file or a directory) and return the populated database.
async fn _parse<'a>(options: ParserOptions<'a>) -> Result<SqliteConnection> {
    log::debug!("Starting parse with options: {:?}", &options);

    let hash = {
        let mut hasher = DefaultHasher::new();
        options.in_path.hash(&mut hasher);
        hasher.finish()
    };

    // The database path is the hash of the log path in hexadecimal (with 'sqlite' extension)
    let db_path = format!("{hash:x}.sqlite");
    let mut conn = open_db(db_path, true).await?;

    let (files, lines, objects) = if options.in_path.is_dir() {
        log::info!("Input path is a directory, using directory parser...");
        DirParser::parse(options.in_path).await?
    } else {
        log::info!("Input path is a file, using file parser...");
        FileParser::parse(options.in_path).await?
    };

    conn.transaction(|tx| {
        diesel::insert_into(schema::files::table)
            .values(&files)
            .execute(tx)?;
        diesel::insert_into(schema::objects::table)
            .values(&objects)
            .execute(tx)?;
        diesel::insert_into(schema::lines::table)
            .values(&lines)
            .execute(tx)
    })?;

    log::info!(
        "Inserted {} files, {} lines, {} objects into the database.",
        files.len(),
        lines.len(),
        objects.len()
    );

    Ok(conn)
}
