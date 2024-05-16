use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Database Connection Error")]
    DBConn(#[from] sqlx::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
