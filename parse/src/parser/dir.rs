use chrono::TimeDelta;
use futures::StreamExt;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use tokio_stream::wrappers::ReadDirStream;

use crate::data::{Database, File, Level, Line, Object};
use crate::parser::model::{DirParserModel, Model};
use crate::util::read_lines;
use crate::{Error, Result};

use super::Parser;

/// Parser for the standard format for CBL logs, a directory containing multiple '.cbllog' files.
pub struct DirParser {
    path: PathBuf,
    files: Vec<(File, Vec<String>)>,
}

impl Parser for DirParser {
    async fn parse(path: impl AsRef<Path>) -> Result<(Vec<File>, Vec<Line>, Vec<Object>)> {
        let model = load_model(&path).await?;

        let dir = tokio::fs::read_dir(&path).await?;
        let mut stream = ReadDirStream::new(dir).enumerate();

        let mut files = vec![];

        while let Some((id, file)) = stream.next().await {
            let file = file?;
            let lines = read_lines(file.path()).await?;
            files.push((
                File {
                    id: id as i32,
                    path: file.path().to_string_lossy().to_string(),
                    // TODO
                    level: Level::Info,
                    // TODO
                    timestamp: Default::default(),
                },
                lines,
            ))
        }

        let mut lines: Vec<Line> = vec![];
        let mut objects: HashSet<Object> = HashSet::default();

        for (file, lines_str) in &files {
            let mut additional_days = TimeDelta::days(0);
            for (i, line) in lines_str.iter().enumerate() {
                let base_date = file.timestamp.date() + additional_days;
                let Ok((mut line, object)) = model.parse_line(line, i + 1, &file, base_date) else {
                    // Don't return early when we fail to parse a line, just skip that line
                    continue;
                };
                // If time_delta is negative, the time on the log line has wrapped to the next day
                let time_delta = line.timestamp - file.timestamp + additional_days;
                if time_delta < TimeDelta::seconds(0) {
                    additional_days += TimeDelta::days(1);
                    line.timestamp += TimeDelta::days(1);
                }
                lines.push(line);
                objects.insert(object);
            }
        }

        let files = files.into_iter().map(|(file, _)| file).collect();

        Ok((files, lines, objects.into_iter().collect()))
    }
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
    DirParserModel::from_version_string(&first_line).map_err(|e| {
        if matches!(e, Error::NoMatches) {
            Error::NotLogs(path.as_ref().to_path_buf())
        } else {
            e
        }
    })
}
