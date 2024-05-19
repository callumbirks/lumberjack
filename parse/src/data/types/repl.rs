use super::Object;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqliteRow;
use sqlx::Error;

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Repl {
    pub config: sqlx::types::Json<Config>,
    #[sqlx(flatten)]
    #[sqlx(default)]
    pub pusher: Option<Object>,
    #[sqlx(flatten)]
    #[sqlx(default)]
    pub puller: Option<Object>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    collections: Vec<Collection>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Collection {
    scope: String,
    name: String,
    push: Mode,
    pull: Mode,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[repr(u16)]
pub enum Mode {
    Disabled,
    OneShot,
    Continuous,
    Passive,
}

impl PartialEq for Repl {
    fn eq(&self, other: &Self) -> bool {
        self.pusher.id == other.pusher.id
    }
}

impl Eq for Repl {}
