use std::cmp::Ordering;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, NaiveTime, TimeDelta, Utc};
use grep::matcher::Matcher;
use grep::regex::RegexMatcher;
use grep::searcher::Searcher;
use grep::searcher::sinks::UTF8;

use crate::{LumberjackError, Result};

#[derive(Debug)]
pub struct Lumberjack {
    pub(crate) files: Arc<[LogFile]>,
}

#[derive(Debug, Clone)]
pub struct LogFile {
    pub path: PathBuf,
    pub start_dt: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct LogLine {
    pub file: Arc<LogFile>,
    // Starts at 1
    pub line_num: u64,
    // Recorded as a TimeDelta compared to file.start_dt
    pub time_delta: TimeDelta,
}

#[derive(Debug)]
pub struct LogObject {
    lines: Vec<LogLine>,
}

#[derive(Debug, Clone)]
pub struct LumberjackMatch {
    pub log_line: LogLine,
    pub snippet: String,
    pub line_str: String,
}

impl LogFile {
    pub fn with_path(path: PathBuf) -> Result<LogFile> {
        // cbllog file names look like "cbl_info_1702053563433.cbllog"
        // First we strip the ".cbllog" extension
        let Some(file_name) = path.file_name()
            .and_then(OsStr::to_str)
            .and_then(|f| f.strip_suffix(".cbllog"))
            else {
                return Err(crate::LumberjackError::FileNotLog(path.to_string_lossy().to_string()));
            };

        // Then we parse the timestamp from the end of the file name
        let Some(timestamp) = file_name.split('_').last()
            .and_then(|t| t.parse::<i64>().ok())
            .and_then(DateTime::from_timestamp_millis)
            else {
                return Err(crate::LumberjackError::ParseTimestampError { line: path.to_string_lossy().to_string() });
            };

        let file = File::open(&path)?;
        Ok(LogFile {
            path,
            start_dt: timestamp,
        })
    }
}

impl Lumberjack {
    pub fn with_dir(dir: &Path) -> Result<Lumberjack> {
        if !dir.is_dir() {
            return Err(LumberjackError::DirectoryInvalid);
        }

        let mut log_files: Vec<LogFile> = vec![];

        for file in dir.read_dir()? {
            let file = file?;
            if matches!(file.path().extension().and_then(OsStr::to_str), Some("cbllog")) {
                log_files.push(LogFile::with_path(file.path())?);
            }
        }

        if log_files.is_empty() {
            return Err(LumberjackError::DirectoryNoLogs);
        }

        Ok(Lumberjack { files: Arc::from(log_files) })
    }

    pub fn find(&self, pattern: &str) -> Result<Vec<LumberjackMatch>> {
        let matcher = RegexMatcher::new(pattern)?;

        let mut lmatches: Vec<LumberjackMatch> = vec![];

        for log_file in self.files.iter() {
            let mut matches: Vec<(u64, String, String)> = vec![];
            let fd = File::open(&log_file.path)?;
            Searcher::new().search_file(&matcher, &fd, UTF8(|lnum, line| {
                if let Some(found) = matcher.find(line.as_bytes())? {
                    matches.push((
                        lnum,
                        line.to_string(),
                        // The matching snippet
                        line[found].to_string(),
                    ));
                    Ok(true)
                } else {
                    Ok(false)
                }
            }))?;

            let file_time = log_file.start_dt.time();

            for (line_num, line_str, snippet) in matches {
                let Ok(line_time) = NaiveTime::parse_from_str(&line_str[..=14], "%H:%M:%S%.6f") else {
                    return Err(LumberjackError::ParseTimestampError { line: line_str.clone() });
                };

                let mut additional_days = TimeDelta::days(0);
                let mut time_delta = line_time - file_time + additional_days;
                // If time_delta is negative, the difference between file_time and line_time is greater than 24 hours
                if time_delta < TimeDelta::seconds(0) {
                    additional_days += TimeDelta::days(1);
                    time_delta += TimeDelta::days(1);
                }

                lmatches.push(
                    LumberjackMatch {
                        log_line: LogLine { file: Arc::new(log_file.clone()), line_num, time_delta },
                        snippet,
                        line_str,
                    });
            }
        }
        if lmatches.is_empty() {
            return Err(LumberjackError::NoMatches(pattern.to_string()));
        }

        lmatches.sort_unstable_by(|m, m_other| m.log_line.cmp(&m_other.log_line));

        Ok(lmatches)
    }
}

impl LogLine {
    pub fn read(&self) -> Result<String> {
        let file = File::open(&self.file.path)?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        // -1 because line_num is 1-indexed, but nth() expects 0-indexed
        let line_str = lines.nth(self.line_num as usize - 1).expect("No line");
        Ok(line_str?)
    }
}

impl Eq for LogLine {}

impl PartialEq<Self> for LogLine {
    fn eq(&self, other: &Self) -> bool {
        self.file.path == other.file.path && self.line_num == other.line_num
    }
}

impl PartialOrd<Self> for LogLine {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LogLine {
    fn cmp(&self, other: &Self) -> Ordering {
        let dt = self.file.start_dt + self.time_delta;
        let other_dt = other.file.start_dt + other.time_delta;
        dt.cmp(&other_dt)
    }
}