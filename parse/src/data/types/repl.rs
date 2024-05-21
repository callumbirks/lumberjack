use crate::data::util::diesel_tosql_json;
use crate::schema::repls;
use diesel::deserialize::FromSql;
use diesel::prelude::*;
use diesel::serialize::IsNull;
use diesel::serialize::{Output, ToSql};
use diesel::sqlite::Sqlite;
use diesel::AsExpression;
use diesel::{sql_types, FromSqlRow};
use serde::{Deserialize, Serialize};

#[derive(Insertable, Identifiable, Queryable, Selectable, Associations, Debug, Clone)]
#[diesel(primary_key(object_id))]
#[diesel(belongs_to(crate::data::Object))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Repl {
    pub object_id: i32,
    pub config: Config,
}

#[derive(AsExpression, FromSqlRow, Serialize, Deserialize, Debug, Clone)]
#[diesel(sql_type = sql_types::Text)]
pub struct Config {
    collections: Vec<Collection>,
}

diesel_tosql_json!(Config);

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
        self.object_id == other.object_id
    }
}

impl Eq for Repl {}
