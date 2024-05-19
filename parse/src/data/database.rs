use std::path::Path;

use futures::future::BoxFuture;
use tokio::sync::RwLock;

use crate::data::{Object, ObjectExtra, ObjectType, Repl};
use crate::{Error, Result};

use super::types::{Level, Line};

pub struct Database(RwLock<Internal>);

struct Internal {
    conn: SqliteConnection,
}

impl Database {
    /// Pass `reset: true` to re-create the database if it already exists at the given
    /// path.
    /// ## Errors
    /// If Sqlite cannot open or create the database at the path provided, or if some other
    /// Sqlite error occurs.
    pub async fn open(path: impl AsRef<Path>, reset: bool) -> Result<Self> {
        log::debug!(
            "Opening database with {{ path: \"{}\", reset: {} }}",
            path.as_ref().to_str().unwrap_or("INVALID UNICODE"),
            reset
        );
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true);
        let mut conn = SqliteConnection::connect_with(&options).await?;

        if reset {
            conn.transaction(|tx| Box::pin(drop_tables(tx))).await?;
        }
        conn.transaction(|tx| Box::pin(create_tables(tx))).await?;

        log::info!("Database opened");

        Ok(Self(RwLock::new(Internal { conn })))
    }

    /// ## Errors
    /// If any of the lines to insert already exist in the database, or for any Sqlite failures.
    pub async fn insert_lines<I, O>(&self, lines: I) -> Result<()>
    where
        for<'a> I: IntoIterator<IntoIter = O> + Sync + Send + 'a,
        for<'a> O: Iterator<Item = Line> + Send + 'a,
    {
        self.transaction(|tx| Box::pin(insert_lines(tx, lines)))
            .await
    }

    /// ## Errors
    /// If a line with the given `level` and `line_num` could not be found, or for any Sqlite
    /// failures.
    pub async fn get_line(&self, level: Level, line_num: u32) -> Result<Line> {
        let res = self
            .transaction(|tx| Box::pin(get_line(tx, level, line_num)))
            .await;

        match &res {
            Ok(line) => {
                log::debug!(
                    "Fetched line {{ object: {}, level: {}, line_num: {}, ... }}",
                    line.object.name(),
                    line.level,
                    line.line_num
                )
            }
            Err(err) => {
                log::error!(
                    "Error {} fetching line with {{ level: {}, line_num: {}, ... }}",
                    err,
                    level,
                    line_num
                )
            }
        }

        res
    }

    async fn transaction<'a, T, F>(&self, f: F) -> Result<T>
    where
        for<'c> F:
            FnOnce(&'c mut Transaction<Sqlite>) -> BoxFuture<'c, Result<T>> + 'a + Send + Sync,
        T: Send,
    {
        self.0.write().await.conn.transaction(f).await
    }
}

async fn get_line(tx: &mut Transaction<'_, Sqlite>, level: Level, line_num: u32) -> Result<Line> {
    let line: Line = sqlx::query_as(
        "SELECT
                        l.*,
                        o.*,
                        f.path
                    FROM lines as l
                    INNER JOIN objects as o ON l.object_id = o.id
                    INNER JOIN files as f ON l.object_id = f.id
                    WHERE level = ? AND line_num = ?",
    )
    .bind(level)
    .bind(line_num)
    .fetch_one(&mut **tx)
    .await
    .map_err(Error::Sqlx)?;

    match line.object.type_ {
        ObjectType::Repl => {
            let repl: Repl = sqlx::query_as(
                "\
            SELECT 
                repl.config,
                o1.*,
                o2.*,
            FROM repl \
            LEFT JOIN objects as o1 ON o1.id = repl.pusher_id
            LEFT JOIN objects as o2 ON o2.id = repl.puller_id
            WHERE repl.id = ?",
            )
            .bind(line.object.id)
            .fetch_one(&mut **tx)
            .await
            // TODO: Map the error so we know the Repl didn't exist, but the line did
            .map_err(Error::Sqlx)?;
            Ok(Line {
                object: Object {
                    extra: ObjectExtra::Repl(Box::new(repl)),
                    ..line.object
                },
                ..line
            })
        }
        _ => Ok(line),
    }
}

async fn insert_lines<I, O>(tx: &mut Transaction<'_, Sqlite>, lines: I) -> Result<()>
where
    I: IntoIterator<IntoIter = O> + Sync + Send,
    O: Iterator<Item = Line> + Send,
{
    let lines = lines.into_iter();
    log::info!("Inserting ~{} lines...", lines.size_hint().0);
    for line in lines {
        sqlx::query(
            "INSERT OR IGNORE INTO objects
                    (id, type_)
                    VALUES (?, ?)",
        )
        .bind(line.object.id)
        .bind(line.object.type_)
        .execute(&mut **tx)
        .await?;

        sqlx::query(
            "INSERT OR IGNORE INTO files
                (id, path)
                VALUES (?, ?)",
        )
        .bind(1)
        .bind("")
        .execute(&mut **tx)
        .await?;

        match line.object.extra {
            ObjectExtra::Repl(r) => {
                sqlx::query(
                    "INSERT INTO repl\
                        (config, pusher_id, puller_id)\
                        VALUES (?, ?, ?)",
                )
                .bind(r.config)
                .bind(r.pusher.id)
                .bind(r.puller.id)
                .execute(&mut **tx)
                .await?;
            }
            _ => {}
        }

        sqlx::query(
            "INSERT INTO lines
                    (level, line_num, timestamp, message, event_type, object_id, file_id)
                    VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(line.level)
        .bind(line.line_num)
        .bind(line.timestamp)
        .bind(&line.message)
        .bind(line.event_type)
        .bind(line.object.id)
        .bind(1)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn create_tables(tx: &mut Transaction<'_, Sqlite>) -> Result<()> {
    log::debug!("Creating tables 'lines', 'files', 'objects'");
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS lines(   \
            level      INTEGER unsigned NOT NULL,\
            line_num   INTEGER unsigned NOT NULL,\
            timestamp  INTEGER          NOT NULL,\
            message    TEXT             NOT NULL,\
            event_type INTEGER          NOT NULL,\
            object_id  INTEGER          NOT NULL,\
            file_id    INTEGER          NOT NULL,\
            PRIMARY KEY (level, line_num),       \
            FOREIGN KEY (object_id)              \
                REFERENCES objects(id),          \
            FOREIGN KEY (file_id)                \
                REFERENCES files(id)             \
        )",
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS files(\
            id   INTEGER PRIMARY KEY,         \
            path TEXT                         \
        )",
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS objects(\
            id    INTEGER PRIMARY KEY,          \
            type_ INTEGER NOT NULL              \
        )",
    )
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS repl(\
            id        INTEGER PRIMARY KEY,   \
            config    BLOB    NOT NULL,      \
            pusher_id INTEGER,               \
            puller_id INTEGER,               \
            FOREIGN KEY (pusher_id)          \
                REFERENCES objects(id),      \
            FOREIGN KEY (puller_id)          \
                REFERENCES objects(id)       \
        )",
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn drop_tables(tx: &mut Transaction<'_, Sqlite>) -> Result<()> {
    log::debug!("Dropping existing tables...");
    sqlx::query("DROP TABLE IF EXISTS lines")
        .execute(&mut **tx)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS files")
        .execute(&mut **tx)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS objects")
        .execute(&mut **tx)
        .await?;
    Ok(())
}
