use log::LevelFilter;
use std::path::PathBuf;
use std::str::FromStr;

use super::types::*;
use super::*;
use crate::{Error, Result};

#[tokio::test]
async fn insert_get_lines() -> Result<()> {
    env_logger::builder()
        .filter_module("lumberjack_parse", LevelFilter::Debug)
        .filter_module("sqlx", LevelFilter::Info)
        .init();
    let path = PathBuf::from_str("lumberjack_test.sqlite").unwrap();
    let db = Database::open(&path, true).await?;
    let lines = vec![
        Line {
            level: Level::Info,
            line_num: 0,
            timestamp: chrono::Local::now().naive_utc(),
            message: "line1".to_string(),
            event_type: EventType::Common(CommonEvent::Created),
            object: Object {
                id: 1,
                type_: ObjectType::DB,
            },
            file: File {
                path: PathBuf::default(),
            },
        },
        Line {
            level: Level::Info,
            line_num: 1,
            timestamp: chrono::Local::now().naive_utc(),
            message: "line2".to_string(),
            event_type: EventType::Common(CommonEvent::Destroyed),
            object: Object {
                id: 1,
                type_: ObjectType::DB,
            },
            file: File {
                path: PathBuf::default(),
            },
        },
    ];

    db.insert_lines(lines.clone()).await?;

    let fetched = db.get_line(Level::Info, 0).await?;
    assert_eq!(fetched, lines[0]);

    let fetched = db.get_line(Level::Info, 1).await?;
    assert_eq!(fetched, lines[1]);

    let fetched = db.get_line(Level::Info, 2).await;
    assert!(matches!(
        fetched.unwrap_err(),
        Error::Sqlx(sqlx::Error::RowNotFound)
    ));

    tokio::fs::remove_file(&path).await?;
    Ok(())
}
