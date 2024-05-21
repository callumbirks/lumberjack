use chrono::NaiveDateTime;
use diesel::associations::HasTable;
use diesel::prelude::*;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::fmt::{Debug, Formatter};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use futures::future::BoxFuture;
use tokio::sync::RwLock;

use crate::data::{ObjectExtra, ObjectType};
use crate::schema::{files, lines, objects, repls};
use crate::{data, schema, Error, Result};

pub struct Database(RwLock<Internal>);

struct Internal {
    conn: SqliteConnection,
}

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

impl Database {
    /// Pass `reset: true` to re-create the database if it already exists at the given
    /// path.
    /// ## Errors
    /// If Sqlite cannot open or create the database at the path provided, or if some other
    /// Sqlite error occurs.
    pub async fn open(path: impl AsRef<Path>, reset: bool) -> Result<Self> {
        let path = path.as_ref().to_string_lossy();
        log::debug!(
            "Opening database with {{ path: \"{}\", reset: {} }}",
            path,
            reset
        );
        let mut conn = SqliteConnection::establish(&path)?;

        if reset {
            conn.revert_all_migrations(MIGRATIONS)?;
        }
        conn.run_pending_migrations(MIGRATIONS)?;

        log::info!("Database opened");

        Ok(Self(RwLock::new(Internal { conn })))
    }

    /// ## Errors
    /// If any of the lines to insert already exist in the database, or for any Sqlite failures.
    pub async fn insert_lines(&self, lines: &[data::Line]) -> Result<()> {
        let mut internal = self.0.write().await;
        let conn = &mut internal.conn;

        diesel::insert_into(lines::table)
            .values(lines)
            .execute(conn)?;

        Ok(())
    }

    pub async fn insert_files(&self, files: &[data::File]) -> Result<()> {
        let mut internal = self.0.write().await;
        let conn = &mut internal.conn;

        diesel::insert_into(files::table)
            .values(files)
            .execute(conn)?;

        Ok(())
    }

    pub async fn insert_objects(&self, objects: &[data::Object]) -> Result<()> {
        let mut internal = self.0.write().await;
        let conn = &mut internal.conn;

        diesel::insert_into(objects::table)
            .values(objects)
            .execute(conn)?;

        Ok(())
    }

    pub async fn all_lines(&self) -> Result<Vec<data::Line>> {
        let mut internal = self.0.write().await;
        let conn = &mut internal.conn;

        lines::dsl::lines
            .select(data::Line::as_select())
            .load(conn)
            .map_err(|e| Error::Diesel(e))
    }

    pub async fn all_objects(&self) -> Result<Vec<data::Object>> {
        let mut internal = self.0.write().await;
        let conn = &mut internal.conn;

        objects::dsl::objects
            .select(data::Object::as_select())
            .load(conn)
            .map_err(|e| Error::Diesel(e))
    }

    /// ## Errors
    /// If a line with the given `level` and `line_num` could not be found, or for any Sqlite
    /// failures.
    pub async fn get_line(&self, level: data::Level, line_num: i64) -> Result<data::Line> {
        let mut internal = self.0.write().await;
        let conn = &mut internal.conn;

        let (line, object, file) = lines::dsl::lines
            .find((level, line_num))
            .inner_join(objects::table)
            .inner_join(files::table)
            .select((
                data::Line::as_select(),
                data::Object::as_select(),
                data::File::as_select(),
            ))
            .first(conn)?;

        let mut extra: ObjectExtra = ObjectExtra::None;

        match object.ty {
            ObjectType::Repl => {
                let repl = data::repl::Repl::belonging_to(&object)
                    .select(data::repl::Repl::as_select())
                    .first(conn)?;
                extra = ObjectExtra::Repl(Box::new(repl))
            }
            _ => {}
        };

        Ok(line)
    }
}

impl Debug for Database {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Database").finish_non_exhaustive()
    }
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Database(RwLock::default())
    }
}
