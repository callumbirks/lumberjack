pub mod data;
mod error;
mod parser;
mod schema;
pub mod util;

use crate::data::Database;
use crate::parser::{DirParser, FileParser, Parser};
pub use error::{Error, Result};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;

/// Parse logs from the given path (either a file or a directory) and return the populated database.
pub async fn parse(path: impl AsRef<Path>) -> Result<Database> {
    let path = path.as_ref();

    let hash = {
        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        hasher.finish()
    };

    // The database path is the hash of the log path in hexadecimal (with 'sqlite' extension)
    let db_path = format!("{hash:x}.sqlite");
    let database = Database::open(db_path, true).await?;

    let (files, lines, objects) = if path.is_dir() {
        DirParser::parse(path).await?
    } else {
        FileParser::parse(path).await?
    };

    database.insert_files(&files).await?;
    database.insert_objects(&objects).await?;
    database.insert_lines(&lines).await?;

    Ok(database)
}
