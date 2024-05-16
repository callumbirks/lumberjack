use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, SqliteConnection, SqlitePool};

mod data;
mod error;
mod util;

pub use error::Error;
pub use error::Result;

// pub async fn fetch<T>(&self, pred: Predicate) -> Result<T> {
//      query_as!(T, &self.conn, translate!(pred))
// }

pub async fn open() -> Result<()> {
    let options = SqliteConnectOptions::new()
        .filename("lumberjack_test")
        .create_if_missing(true);
    let conn = SqliteConnection::connect_with(&options).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
