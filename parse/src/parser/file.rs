use crate::data::{File, Line, Object};
use crate::parser::Parser;
use std::path::{Path, PathBuf};

pub struct FileParser {
    path: PathBuf,
}

impl Parser for FileParser {
    async fn parse(path: impl AsRef<Path>) -> crate::Result<(Vec<File>, Vec<Line>, Vec<Object>)> {
        let contents = tokio::fs::read_to_string(path).await?;
        for line in contents.lines() {
            todo!()
        }
        unimplemented!();
    }
}
