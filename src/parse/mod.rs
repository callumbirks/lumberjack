use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs::File;
use std::path::Path;
use std::slice::SliceIndex;
use std::sync::{Arc, OnceLock};

use chrono::{NaiveTime, TimeDelta};
use enum_iterator::all;
use grep::matcher::Matcher;
use grep::regex::RegexMatcher;
use grep::searcher::sinks::UTF8;
use grep::searcher::Searcher;
use iced::widget::shader::wgpu::naga::{FastHashMap, FastHashSet};
use tokio::fs::read_dir;
use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt;

use crate::data::{LogEventType, LogFile, LogLine, LogObject, LogObjectType};
use crate::error::{LumberjackError, Result};

pub mod db;
pub mod puller;
pub mod pusher;
pub mod repl;

#[derive(Debug, Clone)]
pub struct LogParser {
    files: Box<[Arc<LogFile>]>,
    // Map from (file, line_num) -> line string
    log_lines: BTreeSet<Arc<LogLine>>,
    objects: FastHashMap<LogObjectType, FastHashSet<Arc<LogObject>>>,
    // Used during parsing. Stored separately from LogFile because LogFiles are kept after parsing,
    // but we don't need to cache the log lines after parsing.
    cached_lines: FastHashMap<Arc<LogFile>, Arc<[String]>>,
}

#[derive(Debug, Clone)]
pub struct LogHolder {
    pub log_lines: Vec<Arc<LogLine>>,
    pub objects: FastHashMap<LogObjectType, Vec<Arc<LogObject>>>,
}

impl LogHolder {
    pub fn new() -> Self {
        let objects: FastHashMap<LogObjectType, Vec<Arc<LogObject>>> =
            all::<LogObjectType>().map(|ot| (ot, vec![])).collect();
        LogHolder {
            log_lines: vec![],
            objects,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogMatch {
    pub log_line: LogLine,
    // The snippet which matched the regex
    pub snippet: Box<str>,
}

pub trait LogObjectParse {
    const OBJECT_TYPE: LogObjectType;
    const PATTERN: &'static str;
    fn parse_event(line: &str) -> Option<LogEventType>;
    fn parse_details<'a>(
        parser: &LogParser,
        lines: impl IntoIterator<Item = &'a Arc<LogLine>>,
    ) -> Result<Box<str>>;
}

impl LogParser {
    pub async fn with_dir(dir_path: &Path) -> Result<Self> {
        let dir = read_dir(dir_path).await?;
        let mut dir_stream = ReadDirStream::new(dir);

        let mut log_files: Vec<Arc<LogFile>> = vec![];

        let mut cached_lines = FastHashMap::default();

        while let Some(Ok(file)) = dir_stream.next().await {
            let path = file.path();
            if matches!(path.extension().and_then(OsStr::to_str), Some("cbllog")) {
                // Read and cache the file's lines
                let lines = {
                    let content = tokio::fs::read_to_string(&path).await?;
                    content.lines().map(str::to_string).collect::<Arc<_>>()
                };

                let file = Arc::new(LogFile::with_path(file.path())?);
                log_files.push(file.clone());
                cached_lines.insert(file, lines);
            }
        }

        let objects = all::<LogObjectType>()
            .map(|ot| (ot, FastHashSet::default()))
            .collect();

        if log_files.is_empty() {
            Err(LumberjackError::DirectoryInvalid)
        } else {
            Ok(LogParser {
                files: log_files.into_boxed_slice(),
                // These start empty and will be built up during parse() calls
                log_lines: BTreeSet::new(),
                objects,
                cached_lines,
            })
        }
    }

    pub async fn parse<T>(mut self) -> Result<Self>
    where
        T: LogObjectParse,
    {
        self.parse_objects::<T>().await.map(|mut lines| {
            for line in &lines {
                if let Some(object) = &line.object {
                    // The mutable key type (LogObject) is fine. LogObject's Hash uses only the ID,
                    // and the mutable part is the `details` string.
                    #[allow(clippy::mutable_key_type)]
                    let entry = self
                        .objects
                        .get_mut(&object.object_type)
                        .expect("Unhandled object type");
                    if !entry.contains(object) {
                        entry.insert(Arc::clone(object));
                    }
                }
            }
            self.log_lines.append(&mut lines);
            self
        })
    }

    pub fn finish(self) -> Result<LogHolder> {
        Ok(LogHolder {
            log_lines: self.log_lines.into_iter().collect(),
            objects: self
                .objects
                .into_iter()
                .map(|(ot, set)| (ot, set.into_iter().collect()))
                .collect(),
        })
    }

    pub async fn find(&self, pattern: &str) -> Result<Vec<LogMatch>> {
        let matcher = RegexMatcher::new(pattern)?;

        let pattern = pattern.to_string();

        let mut matches: Vec<LogMatch> = vec![];
        for log_file in self.files.iter() {
            let mut file_matches: Vec<(u64, Box<str>, Box<str>)> = vec![];
            let fd = File::open(&log_file.path)?;
            Searcher::new().search_file(
                &matcher,
                &fd,
                UTF8(|line_num, line| {
                    if let Some(found) = matcher.find(line.as_bytes())? {
                        let check = &line[(found.start() - 3)..found.start()];
                        if check == ": {" {
                            file_matches.push((
                                line_num,
                                line.to_string().into_boxed_str(),
                                // The matching snippet
                                line[found].to_string().into_boxed_str(),
                            ));
                        }
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                }),
            )?;

            let file_datetime = log_file.timestamp;
            let file_time = file_datetime.time();

            for (line_num, line_str, snippet) in file_matches {
                let Ok(line_time) = NaiveTime::parse_from_str(&line_str[..=14], "%H:%M:%S%.6f")
                else {
                    return Err(LumberjackError::ParseTimestampError {
                        line: line_str.clone(),
                    });
                };

                let (_, line_str) = line_str.split_once(&*snippet).unwrap();
                let line_str = &line_str[2..];

                let mut additional_days = TimeDelta::days(0);
                let mut time_delta = line_time - file_time + additional_days;
                // If time_delta is negative, the difference between file_time and line_time is greater than 24 hours
                if time_delta < TimeDelta::seconds(0) {
                    additional_days += TimeDelta::days(1);
                    time_delta += TimeDelta::days(1);
                }

                matches.push(LogMatch {
                    log_line: LogLine {
                        file: Arc::clone(log_file),
                        message: line_str.to_string().into_boxed_str(),
                        event: None,
                        line_num,
                        timestamp: file_datetime + time_delta,
                        object: None,
                    },
                    snippet,
                });
            }
        }
        if matches.is_empty() {
            return Err(LumberjackError::NoMatches(
                pattern.to_string().into_boxed_str(),
            ));
        }
        matches.sort_unstable_by(|m, m_other| m.log_line.cmp(&m_other.log_line));
        Ok(matches)
    }

    async fn parse_objects<T>(&self) -> Result<BTreeSet<Arc<LogLine>>>
    where
        T: LogObjectParse,
    {
        let matches = self.find(T::PATTERN).await?;

        let mut objects: FastHashMap<usize, (Arc<LogObject>, BTreeSet<Arc<LogLine>>)> =
            FastHashMap::default();

        for mat in matches {
            let Some(id) = mat
                .snippet
                .split('#')
                .last()
                .and_then(|n| n.parse::<u64>().ok())
            else {
                return Err(LumberjackError::ParseError(format!(
                    "Couldn't parse Repl ID in snippet {:?}",
                    mat.snippet
                )));
            };

            let (object, lines) = objects.entry(id as usize).or_insert_with(|| {
                let object = Arc::new(LogObject {
                    object_type: T::OBJECT_TYPE,
                    id,
                    details: OnceLock::new(),
                });
                (object, BTreeSet::new())
            });

            let event = T::parse_event(&mat.log_line.message);

            lines.insert(Arc::new(LogLine {
                object: Some(Arc::clone(object)),
                event,
                ..mat.log_line
            }));
        }

        if objects.is_empty() {
            return Err(LumberjackError::NoMatches(
                T::PATTERN.to_string().into_boxed_str(),
            ));
        }

        objects
            .into_values()
            .map(|(object, lines)| {
                let details = T::parse_details(&self, lines.iter())?;
                object.details.set(details).ok();
                Ok(lines)
            })
            .try_fold(
                BTreeSet::new(),
                |mut res, lines: Result<BTreeSet<Arc<LogLine>>>| {
                    res.append(&mut lines?);
                    Ok(res)
                },
            )
    }

    pub fn parse_id<T>(line: &str) -> Option<u64>
    where
        T: LogObjectParse,
    {
        let matcher = RegexMatcher::new(T::PATTERN).ok()?;
        let mut result = None;
        Searcher::new()
            .search_slice(
                &matcher,
                line.as_bytes(),
                UTF8(|line_num, line| {
                    if let Some(Some(found)) = matcher.find(line.as_bytes()).ok() {
                        let check = &line[(found.start() - 3)..found.start()];
                        if check == ": {" {
                            let snippet = &line[found];
                            result = snippet
                                .split('#')
                                .last()
                                .and_then(|n| n.parse::<u64>().ok());
                            Ok(true)
                        } else {
                            Ok(false)
                        }
                    } else {
                        Ok(false)
                    }
                }),
            )
            .ok()?;
        result
    }

    pub fn get_lines<I>(&self, file: &Arc<LogFile>, index: I) -> Option<&I::Output>
    where
        I: SliceIndex<[String]>,
    {
        self.cached_lines.get(file).and_then(|s| s.get(index))
    }
}
