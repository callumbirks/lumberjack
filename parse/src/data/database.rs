use chrono::NaiveDateTime;
use diesel::associations::HasTable;
use diesel::dsl::AsExprOf;
use diesel::expression::{AsExpression, TypedExpressionType};
use diesel::prelude::*;
use diesel::query_builder::QueryBuilder;
use diesel::sql_types::SqlType;
use diesel::sqlite::SqliteQueryBuilder;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::fmt::{Debug, Formatter};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use futures::future::BoxFuture;
use tokio::sync::RwLock;

use crate::data::{ObjectExtra, ObjectType};
use crate::schema::{files, lines, objects, repls};
use crate::{data, schema, Error, Result};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub async fn open_db(path: impl AsRef<Path>, reset: bool) -> Result<SqliteConnection> {
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

    log::info!("Database opened");

    Ok(conn)
}
