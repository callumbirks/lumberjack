use crate::data::Database;
use crate::parser::Parser;
use std::path::{Path, PathBuf};

pub struct FileParser {
    path: PathBuf,
}

impl Parser for FileParser {
    async fn parse(path: impl AsRef<Path>, database: &Database) -> crate::Result<()> {
        let contents = tokio::fs::read_to_string(path).await?;
        for line in contents.lines() {
            todo!()
        }

        Ok(())
    }
}
