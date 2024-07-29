use std::path::Path;

use diesel::prelude::*;
use diesel_migrations::*;

use crate::Result;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub fn open_db(path: impl AsRef<Path>, reset: bool) -> Result<SqliteConnection> {
    let path = path.as_ref().to_string_lossy();
    log::debug!(
        "Opening database with {{ path: '{}', reset: {} }}",
        path,
        reset
    );
    let mut conn = SqliteConnection::establish(&path)?;

    if reset {
        conn.revert_all_migrations(MIGRATIONS)?;
    }
    conn.run_pending_migrations(MIGRATIONS)?;

    log::info!("Database opened at \"{}\"", path);

    Ok(conn)
}
