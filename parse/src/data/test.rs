use chrono::{Local, NaiveDateTime};
use diesel::result::DatabaseErrorKind;
use log::LevelFilter;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

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
    let object = Object {
        id: 1,
        ty: ObjectType::DB,
    };
    let file = File {
        id: 0,
        path: "".to_string(),
        level: Level::Info,
        timestamp: Local::now().naive_utc(),
    };
    let lines = vec![
        Line {
            level: Level::Info,
            line_num: 0,
            timestamp: chrono::Local::now().naive_utc(),
            message: "line1".to_string(),
            event_type: EventType::Common(CommonEvent::Created),
            object_id: 1,
            file_id: 0,
        },
        Line {
            level: Level::Info,
            line_num: 1,
            timestamp: chrono::Local::now().naive_utc(),
            message: "line2".to_string(),
            event_type: EventType::Common(CommonEvent::Destroyed),
            object_id: 1,
            file_id: 0,
        },
    ];

    db.insert_files(&[file]).await?;
    db.insert_objects(&[object]).await?;
    db.insert_lines(&lines).await?;

    let fetched = db.get_lines(Level::Info, 0).await?;
    assert_eq!(fetched, lines[0]);

    let fetched = db.get_lines(Level::Info, 1).await?;
    assert_eq!(fetched, lines[1]);

    let fetched = db.get_lines(Level::Info, 2).await;
    assert!(matches!(
        fetched.unwrap_err(),
        Error::Diesel(diesel::result::Error::NotFound)
    ));

    tokio::fs::remove_file(&path).await?;
    Ok(())
}
