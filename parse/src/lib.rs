pub mod data;
mod error;
mod parser;
mod schema;
pub mod util;

use crate::data::open_db;
use crate::parser::{DirParser, FileParser, Parser};
use crate::schema::{files, lines, objects};
use diesel::{Insertable, RunQueryDsl, SqliteConnection};
pub use error::{Error, Result};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;

/// Parse logs from the given path (either a file or a directory) and return the populated database.
pub async fn parse(path: impl AsRef<Path>) -> Result<SqliteConnection> {
    let path = path.as_ref();

    let hash = {
        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        hasher.finish()
    };

    // The database path is the hash of the log path in hexadecimal (with 'sqlite' extension)
    let db_path = format!("{hash:x}.sqlite");
    let mut conn = open_db(db_path, false).await?;

    let (files, lines, objects) = if path.is_dir() {
        DirParser::parse(path).await?
    } else {
        FileParser::parse(path).await?
    };

    diesel::insert_into(files::table)
        .values(&files)
        .execute(&mut conn)?;
    diesel::insert_into(objects::table)
        .values(&objects)
        .execute(&mut conn)?;
    diesel::insert_into(lines::table)
        .values(&lines)
        .execute(&mut conn)?;

    Ok(conn)
}
