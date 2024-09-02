use std::path::Path;

use crate::{Error, Result};

const MIGRATIONS: &str = include_str!("./schema.sql");

pub fn open_db(path: impl AsRef<Path>, reset: bool) -> Result<rusqlite::Connection> {
    let path = path.as_ref();

    if reset && path.exists() {
        std::fs::remove_file(path)
            .map_err(|err| Error::CannotParse(format!("Failed to remove database: {}", err)))?;
    }

    log::debug!(
        "Opening database with {{ path: {:?}, reset: {} }}",
        path,
        reset
    );

    let flags =
        rusqlite::OpenFlags::SQLITE_OPEN_CREATE | rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE;
    let conn = rusqlite::Connection::open_with_flags(path, flags)?;

    if reset {
        // Optimization for fast bulk inserts
        conn.execute_batch(
            "
                PRAGMA journal_mode=OFF;
                PRAGMA synchronous=0;
                PRAGMA cache_size=500000;
                PRAGMA locking_mode=EXCLUSIVE;
                PRAGMA temp_store=MEMORY;
            ",
        )?;
        // Create the schema
        conn.execute_batch(MIGRATIONS)?;
    }

    log::debug!("Database opened at {:?}", path);

    Ok(conn)
}
