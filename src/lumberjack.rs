use std::cmp::Ordering;
use std::ffi::OsStr;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::Arc;

use chrono::{DateTime, NaiveTime, TimeDelta, Utc};
use grep::matcher::Matcher;
use grep::regex::RegexMatcher;
use grep::searcher::sinks::UTF8;
use grep::searcher::Searcher;
use tokio::fs::read_dir;
use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt;

use crate::data::repl::Repl;
use crate::data::LogObjectGroup;
use crate::util::read_file;
use crate::{LumberjackError, Result};

#[derive(Debug, Clone)]
pub struct Lumberjack {
    pub files: Arc<[Arc<LogFile>]>,
    pub repl_objects: Vec<Arc<Repl>>,
}

#[derive(Debug, Clone)]
pub struct LogFile(Pin<Box<LogFileInner>>);

// Wrapped in a PinBox because it is self-referential; `lines` contains pointers to substrings
// of `content`.
#[derive(Debug, Clone)]
struct LogFileInner {
    pub path: PathBuf,
    pub start_dt: DateTime<Utc>,
    pub content: Box<str>,
    lines: Box<[NonNull<str>]>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct LogLine {
    pub file: Arc<LogFile>,
    // Starts at 1
    pub line_num: u64,
    // Recorded as a TimeDelta compared to file.start_dt
    pub time_delta: TimeDelta,
}

#[derive(Debug, Clone)]
pub struct LumberjackMatch {
    pub log_line: LogLine,
    pub snippet: String,
}

impl LogFile {
    pub async fn with_path(path: PathBuf) -> Result<LogFile> {
        // cbllog file names look like "cbl_info_1702053563433.cbllog"
        // First we strip the ".cbllog" extension
        let Some(file_name) = path
            .file_name()
            .and_then(OsStr::to_str)
            .and_then(|f| f.strip_suffix(".cbllog"))
        else {
            return Err(LumberjackError::FileNotLog(
                path.to_string_lossy().to_string(),
            ));
        };

        // Then we parse the timestamp from the end of the file name
        let Some(timestamp) = file_name
            .split('_')
            .last()
            .and_then(|t| t.parse::<i64>().ok())
            .and_then(DateTime::from_timestamp_millis)
        else {
            return Err(LumberjackError::ParseTimestampError {
                line: path.to_string_lossy().to_string(),
            });
        };

        let content = read_file(&path).await?;
        let lines = Self::line_pointers(&content);

        Ok(LogFile(Box::pin(LogFileInner {
            path,
            start_dt: timestamp,
            content,
            lines,
        })))
    }

    pub fn path(&self) -> &Path {
        &self.0.path
    }

    pub fn start_dt(&self) -> &DateTime<Utc> {
        &self.0.start_dt
    }

    pub fn content(&self) -> &str {
        &self.0.content
    }

    pub fn lines(&self) -> &[NonNull<str>] {
        &self.0.lines
    }

    fn line_pointers(string: &str) -> Box<[NonNull<str>]> {
        string.lines().map(NonNull::from).collect()
    }
}

impl Lumberjack {
    pub async fn with_dir(dir_path: &Path) -> Result<Lumberjack> {
        if !dir_path.is_dir() {
            return Err(LumberjackError::DirectoryInvalid);
        }

        let mut log_files: Vec<Arc<LogFile>> = vec![];

        let mut dir_stream = ReadDirStream::new(read_dir(dir_path).await?);

        while let Some(Ok(file)) = dir_stream.next().await {
            if matches!(
                file.path().extension().and_then(OsStr::to_str),
                Some("cbllog")
            ) {
                log_files.push(Arc::new(LogFile::with_path(file.path()).await?));
            }
        }

        if log_files.is_empty() {
            return Err(LumberjackError::DirectoryNoLogs);
        }

        let mut lumberjack = Lumberjack {
            files: Arc::from(log_files),
            repl_objects: vec![],
        };

        lumberjack.repl_objects = Repl::from_lumberjack(&lumberjack).await?;

        Ok(lumberjack)
    }

    /**
     * Finds and returns all log lines which match the given regex pattern.
     * The results will be sorted by timestamp (oldest first).
     */
    pub async fn find(&self, pattern: &str) -> Result<Vec<LumberjackMatch>> {
        let matcher = RegexMatcher::new(pattern)?;

        let log_files: Box<[Arc<LogFile>]> = self.files.iter().map(Arc::clone).collect();
        let pattern = pattern.to_string();

        tokio::task::spawn_blocking(move || {
            let mut matches: Vec<LumberjackMatch> = vec![];
            for log_file in log_files.iter() {
                let mut file_matches: Vec<(u64, String, String)> = vec![];
                let fd = File::open(log_file.path())?;
                Searcher::new().search_file(
                    &matcher,
                    &fd,
                    UTF8(|line_num, line| {
                        if let Some(found) = matcher.find(line.as_bytes())? {
                            file_matches.push((
                                line_num,
                                line.to_string(),
                                // The matching snippet
                                line[found].to_string(),
                            ));
                            Ok(true)
                        } else {
                            Ok(false)
                        }
                    }),
                )?;

                let file_time = log_file.start_dt().time();

                for (line_num, line_str, snippet) in file_matches {
                    let Ok(line_time) = NaiveTime::parse_from_str(&line_str[..=14], "%H:%M:%S%.6f")
                    else {
                        return Err(LumberjackError::ParseTimestampError {
                            line: line_str.clone(),
                        });
                    };

                    let mut additional_days = TimeDelta::days(0);
                    let mut time_delta = line_time - file_time + additional_days;
                    // If time_delta is negative, the difference between file_time and line_time is greater than 24 hours
                    if time_delta < TimeDelta::seconds(0) {
                        additional_days += TimeDelta::days(1);
                        time_delta += TimeDelta::days(1);
                    }

                    matches.push(LumberjackMatch {
                        log_line: LogLine {
                            file: log_file.clone(),
                            line_num,
                            time_delta,
                        },
                        snippet,
                    });
                }
            }
            if matches.is_empty() {
                return Err(LumberjackError::NoMatches(pattern.to_string()));
            }
            matches.sort_unstable_by(|m, m_other| m.log_line.cmp(&m_other.log_line));
            Ok(matches)
        })
        .await
        .map_err(|_| LumberjackError::TokioJoin)?
    }
}

impl LogLine {
    pub fn read(&self) -> &str {
        unsafe { self.file.lines()[self.line_num as usize - 1].as_ref() }
    }
}

impl Hash for LogFile {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.path().hash(state);
        self.start_dt().hash(state);
    }
}

impl PartialEq for LogFile {
    fn eq(&self, other: &Self) -> bool {
        self.path().eq(other.path())
    }
}
impl Eq for LogFile {}

impl Display for LogLine {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.read().fmt(f)
    }
}

impl PartialOrd<Self> for LogLine {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LogLine {
    fn cmp(&self, other: &Self) -> Ordering {
        let dt = *self.file.start_dt() + self.time_delta;
        let other_dt = *other.file.start_dt() + other.time_delta;
        dt.cmp(&other_dt)
    }
}

// This is fine because LogFile is never mutated
unsafe impl Send for LogFile {}
unsafe impl Sync for LogFile {}
