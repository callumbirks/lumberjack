use crate::LumberjackError;
use std::path::Path;
use std::sync::Arc;

pub async fn read_file(path: &Path) -> crate::Result<Box<str>> {
    tokio::fs::read_to_string(path)
        .await
        .map(String::into_boxed_str)
        .map_err(|err| LumberjackError::Io(err.kind()))
}

pub async fn read_lines(path: &Path) -> crate::Result<Box<[Arc<str>]>> {
    let contents = tokio::fs::read_to_string(path).await?.into_boxed_str();

    Ok(contents.lines().map(|s| Arc::from(s)).collect())
}
