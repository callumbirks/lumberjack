use std::path::{Path, PathBuf};

use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt;

use crate::data::Database;
use crate::parser::model::{DirParserModel, Model};
use crate::util::read_lines;
use crate::{Error, Result};

use super::Parser;

/// Parser for the standard format for CBL logs, a directory containing multiple '.cbllog' files.
pub struct DirParser {
    path: PathBuf,
    files: Vec<File>,
}

impl Parser for DirParser {
    async fn parse(path: impl AsRef<Path>, database: &Database) -> Result<()> {
        let model = load_model(&path).await?;

        let dir = tokio::fs::read_dir(&path).await?;
        let mut stream = ReadDirStream::new(dir);

        let mut files = vec![];

        while let Some(file) = stream.next().await {
            let file = file?;
            let lines = read_lines(file.path()).await?;
            files.push(File {
                path: file.path().to_path_buf(),
                lines,
            })
        }

        let parser = Self {
            path: path.as_ref().to_path_buf(),
            files,
        };

        Ok(())
    }
}

struct File {
    path: PathBuf,
    lines: Vec<String>,
}

async fn load_model(path: impl AsRef<Path>) -> Result<Box<DirParserModel>> {
    let mut dir = tokio::fs::read_dir(&path).await?;
    // The first file in the dir
    let Some(file) = dir.next_entry().await? else {
        return Err(Error::NotLogs(path.as_ref().to_path_buf()));
    };

    let contents = tokio::fs::read_to_string(file.path()).await?;
    let Some(first_line) = contents.lines().into_iter().nth(0).map(ToString::to_string) else {
        return Err(Error::NotLogs(path.as_ref().to_path_buf()));
    };

    // In the standard format, the version string is always output at the top of every file.
    DirParserModel::from_version_string(first_line).map_err(|e| {
        if matches!(e, Error::NoMatches) {
            Error::NotLogs(path.as_ref().to_path_buf())
        } else {
            e
        }
    })
}
