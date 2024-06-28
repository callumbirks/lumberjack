use chrono::format::ParseErrorKind;
use chrono::{NaiveDateTime, TimeDelta};
use futures::StreamExt;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use tokio_stream::wrappers::ReadDirStream;

use crate::data::{File, Level, Line, Object};
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
        log::debug!(
            "Opening directory at \"{}\" for parsing...",
            path.as_ref().to_string_lossy()
        );
        let model = load_model(&path).await?;

        log::info!("Loaded model with compatibility {}", model.compatibility);

        let dir = tokio::fs::read_dir(&path).await?;
        let mut stream = ReadDirStream::new(dir).enumerate();

        // Each file paired with its contents as lines.
        let mut files: Vec<(File, Vec<String>)> = vec![];

        // Iterate over each file in the directory, parsing the log level, timestamp, and creating
        // `File` objects. Also read the lines from each file and pair them with the file they
        // belong to.
        while let Some((id, file)) = stream.next().await {
            let file = file?;
            let file_path = file.path();
            let Ok(lines) = read_lines(&file_path).await else {
                log::error!(
                    "Failed to read lines of \"{}\", skipping...",
                    file_path.to_string_lossy()
                );
                continue;
            };
            // file_stem gives filename without extension
            let Some(file_name) = file_path.file_stem().and_then(OsStr::to_str) else {
                log::error!(
                    "Failed to read filename of \"{}\", skipping...",
                    file_path.to_string_lossy()
                );
                continue;
            };
            let Ok(level) = level_from_filename(file_name) else {
                log::error!(
                    "Failed to parse log level from \"{}\", skipping...",
                    &file_name
                );
                continue;
            };
            let Ok(timestamp) = timestamp_from_filename(file_name) else {
                log::error!(
                    "Failed to parse timestamp from \"{}\", skipping...",
                    &file_name
                );
                continue;
            };
            let entry = (
                File {
                    id: id as i32,
                    path: file_path.to_string_lossy().to_string(),
                    level,
                    timestamp,
                },
                lines,
            );
            log::debug!("Parsed {:?} with {} lines", entry.0, entry.1.len());
            files.push(entry);
        }

        log::info!(
            "Found {} files, {} lines to parse.",
            files.len(),
            files.iter().fold(0_i64, |mut x, (_, lines)| x
                .saturating_add(lines.len() as i64))
        );

        let mut lines: Vec<Line> = vec![];
        // We use a `HashSet` for objects because each object will appear across many different
        // lines.
        let mut objects: HashSet<Object> = HashSet::default();

        let mut skipped: i64 = 0;

        // Iterate over each file, and the lines it contains, to parse `Object`s and `Line`s.
        for (file, lines_str) in &files {
            // CBL 3.1 (on most platforms) does not log a full timestamp on each line, just the
            // time. So we need to roll over the day to account for logs spanning more than 1 day.
            let mut additional_days = TimeDelta::days(0);
            let mut unknown_objects: HashSet<String> = HashSet::new();
            for (i, line) in lines_str.into_iter().enumerate() {
                let base_date = file.timestamp.date() + additional_days;

                let res = model.parse_line(line, i + 1, &file, base_date);
                let Ok((mut line, object)) = res else {
                    // It's okay to not be able to parse lines, not every line is relevant.
                    let err = res.unwrap_err();
                    log::trace!("Line parsing error: \"{}\"", &err);
                    if let Error::UnknownObject(object_str) = err {
                        unknown_objects.insert(object_str);
                    }
                    skipped = skipped.saturating_add(1);
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
            if !unknown_objects.is_empty() {
                log::warn!("Found unknown objects: {:?}", unknown_objects);
            }
        }

        // Enable trace logs (`RUST_LOG=trace`) to see why each line was skipped.
        log::info!("Parsed {} lines, skipped {} lines", lines.len(), skipped);
        log::info!("Found {} unique objects", objects.len());

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

fn level_from_filename(name: &str) -> Result<Level> {
    let Some(level_str) = name.split('_').nth(1) else {
        log::error!("Failed to parse log level from filename '{name}'");
        return Err(Error::InvalidFilename(name.to_string()));
    };
    Level::from_str(level_str)
}

fn timestamp_from_filename(name: &str) -> Result<NaiveDateTime> {
    let Some(ts_str) = name.split('_').last() else {
        log::error!("Failed to parse timestamp from filename '{name}'");
        return Err(Error::InvalidFilename(name.to_string()));
    };

    let Some(dt) = ts_str
        .parse()
        .ok()
        .and_then(|int| chrono::DateTime::from_timestamp_millis(int))
    else {
        log::error!("Failed to parse timestamp from filename '{name}'");
        return Err(Error::InvalidFilename(name.to_string()));
    };
    Ok(dt.naive_utc())
}
