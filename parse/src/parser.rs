use std::{
    collections::HashSet,
    ffi::OsStr,
    io::Read,
    path::{Path, PathBuf},
    str::FromStr,
};

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeDelta};
use regex::Regex;
use util::match_event;

use crate::{
    data::{event::*, Domain, Event, File, Level, Line, Object, ObjectType},
    decoder, Error, Result,
};

pub struct Parser {
    files: Vec<PathBuf>,
    patterns: regex_patterns::Patterns,
    version: semver::Version,
}

pub struct ParserOutput {
    pub file: File,
    pub lines: Vec<Line>,
    pub objects: HashSet<Object>,
}

impl Parser {
    pub fn new(path: &Path) -> Result<Self> {
        let files = Self::find_log_files(path)?;
        if files.is_empty() {
            log::error!("No valid log files found at path {:?}!", path);
            return Err(Error::NotLogs(path.to_path_buf()));
        }
        let (patterns, version) = regex_patterns::patterns_for_file(&files[0])?;
        Ok(Self {
            files,
            patterns,
            version,
        })
    }

    pub fn parse(&self) -> impl Iterator<Item = ParserOutput> + '_ {
        ParserIter {
            parser: self,
            index: 0,
        }
    }

    fn parse_file(&self, index: usize) -> Result<ParserOutput> {
        let path = self.files[index].as_path();
        let lines = read_lines(path)?;
        let Some(file_name) = path.file_stem().and_then(OsStr::to_str) else {
            return Err(Error::InvalidFilename(path.to_string_lossy().to_string()));
        };
        let level = level_from_filename(file_name);
        let Some(timestamp) = timestamp_from_filename(file_name) else {
            return Err(Error::InvalidFilename(path.to_string_lossy().to_string()));
        };

        let file = File {
            id: index as i32,
            path: path.to_string_lossy().to_string(),
            level,
            timestamp,
        };

        let mut additional_days = TimeDelta::days(0);
        let (lines, objects): (Vec<Line>, HashSet<Object>) = lines
            .into_iter()
            .enumerate()
            .map(|(i, line)| {
                let res = self.parse_line(
                    &line,
                    i as u64,
                    &file,
                    file.timestamp.date() + additional_days,
                );

                let Ok((mut line, object)) = res else {
                    let err = res.unwrap_err();
                    log::trace!("Line parsing error: {}", &err);
                    return Err(err);
                };

                if line.timestamp - file.timestamp < additional_days {
                    additional_days += TimeDelta::days(1);
                    line.timestamp += TimeDelta::days(1);
                }

                Ok((line, object))
            })
            .filter_map(|res| res.ok())
            .unzip();

        Ok(ParserOutput {
            file,
            lines,
            objects,
        })
    }

    fn parse_line(
        &self,
        line: &str,
        line_num: u64,
        file: &File,
        base_date: NaiveDate,
    ) -> Result<(Line, Object)> {
        let domain = parse_domain(line, &self.patterns.domain)?;

        let object = parse_object(line, &self.patterns.object)?;

        let timestamp = parse_timestamp(
            base_date,
            line,
            self.patterns.timestamp.as_slice(),
            self.patterns.timestamp_formats.as_slice(),
        )?;

        let line = {
            let object_end = self
                .patterns
                .object
                .captures(line)
                .unwrap()
                .name("id")
                .unwrap()
                .end();
            line.split_at(object_end + 2).1
        };

        let level = if let Some(level) = file.level {
            level
        } else {
            unimplemented!("Parse line level")
        };

        let event = parse_event(line, object.object_type, &self.patterns)?;

        let line = Line {
            file_id: file.id,
            line_num: line_num as i64,
            level,
            timestamp,
            domain,
            event_type: event.event_type,
            event_data: event.data,
            object_id: object.id,
        };

        Ok((line, object))
    }

    fn find_log_files(path: &Path) -> Result<Vec<PathBuf>> {
        log::debug!(
            "Searching for valid log files in file or directory {:?}",
            path
        );
        let files = if path.is_dir() {
            let dir = std::fs::read_dir(path)?;
            dir.into_iter()
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| path.is_file())
                .filter(|path| regex_patterns::patterns_for_file(&path).is_ok())
                .collect()
        } else if regex_patterns::patterns_for_file(&path).is_ok() {
            vec![path.to_path_buf()]
        } else {
            vec![]
        };
        Ok(files)
    }
}

struct ParserIter<'a> {
    parser: &'a Parser,
    index: usize,
}

impl<'a> Iterator for ParserIter<'a> {
    type Item = ParserOutput;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.parser.files.len() {
            return None;
        }
        self.index += 1;
        match self.parser.parse_file(self.index - 1) {
            Ok(output) => Some(output),
            Err(err) => {
                log::error!(
                    "Error parsing file '{}': {}",
                    self.parser.files[self.index - 1].to_string_lossy(),
                    err
                );
                None
            }
        }
    }
}

// TODO: Remove once we are using 3.2 logs
fn level_from_filename(name: &str) -> Option<Level> {
    let Some(level_str) = name.split('_').nth(1) else {
        return None;
    };
    Level::from_str(level_str).ok()
}

// TODO: Remove once we are using 3.2 logs
fn timestamp_from_filename(name: &str) -> Option<NaiveDateTime> {
    let Some(ts_str) = name.split('_').last() else {
        return None;
    };

    let Some(dt) = ts_str
        .parse()
        .ok()
        .and_then(|int| chrono::DateTime::from_timestamp_millis(int))
    else {
        return None;
    };

    Some(dt.naive_utc())
}

fn parse_domain(line: &str, regex: &Regex) -> Result<Domain> {
    let Some(caps) = regex.captures(line) else {
        return Err(Error::NoDomain(line.to_string()));
    };

    let domain_str = caps
        .name("domain")
        .ok_or(Error::NoDomain(line.to_string()))?
        .as_str();

    Domain::from_str(domain_str)
}

fn parse_object(line: &str, regex: &Regex) -> Result<Object> {
    let Some(caps) = regex.captures(line) else {
        return Err(Error::NoObject(line.to_string()));
    };

    let (obj_match, id_match) = (
        caps.name("obj").ok_or(Error::NoObject(line.to_string()))?,
        caps.name("id").ok_or(Error::NoObject(line.to_string()))?,
    );

    let (obj_str, id_str) = (obj_match.as_str(), id_match.as_str());

    let object_type = ObjectType::from_str(obj_str)?;
    let object_id: i32 = id_str.parse()?;

    Ok(Object {
        id: object_id,
        object_type,
        data: None,
    })
}

fn parse_timestamp(
    base_date: NaiveDate,
    line: &str,
    timestamp_regex: &[Regex],
    timestamp_formats: &[&str],
) -> Result<NaiveDateTime> {
    for timestamp_re in timestamp_regex {
        let Some(caps) = timestamp_re.captures(line) else {
            continue;
        };

        let Some(ts_match) = caps.name("ts") else {
            panic!("Regex has no 'ts' capture group!!");
        };

        let ts_str = ts_match.as_str();

        for timestamp_format in timestamp_formats {
            let Ok(ts) = NaiveTime::parse_from_str(ts_str, timestamp_format) else {
                continue;
            };
            return Ok(base_date.and_time(ts));
        }
    }
    Err(Error::NoTimestamp(line.to_string()))
}

fn parse_event(
    line: &str,
    object_type: ObjectType,
    patterns: &regex_patterns::Patterns,
) -> Result<Event> {
    match_event! { line, object_type, patterns,
        ObjectType::DB => [
            DBOpenEvent,
            DBUpgradeEvent,
            DBTxBeginEvent,
            DBTxCommitEvent,
            DBTxAbortEvent,
            DBSavedRevEvent,
        ],
        ObjectType::Repl => [
            ReplConflictScanEvent,
            ReplConnectedEvent,
            ReplActivityUpdateEvent,
            ReplStatusUpdateEvent,
        ],
        //ObjectType::Query => [
        //    // TODO: QueryCreateIndexEvent,
        //],
        ObjectType::Housekeeper => [
            HousekeeperMonitorEvent,
        ],
        ObjectType::BLIPIO => [
            BLIPSendRequestStartEvent,
            BLIPQueueRequestEvent,
            BLIPWSWriteStartEvent,
            BLIPSendFrameEvent,
            BLIPSendRequestEndEvent,
            BLIPWSWriteEndEvent,
            BLIPReceiveFrameEvent,
        ]
    }
}

pub(crate) fn read_lines(file_path: &Path) -> Result<Vec<String>> {
    if decoder::is_encoded(file_path)? {
        decoder::decode_lines(file_path)
    } else {
        let contents = std::fs::read_to_string(file_path)?;
        Ok(contents.lines().into_iter().map(str::to_string).collect())
    }
}

mod util {
    macro_rules! match_event {
        ($line:expr, $object_ty:expr, $regex_cache:expr, $($mat_object_ty:pat => [
            $($event_ty:ty),+$(,)?
        ]),+$(,)?) => {
            match $object_ty {
                $($mat_object_ty => {
                    $(if let Ok(event) = <$event_ty>::from_line($line, $regex_cache) {
                        return Ok(Event::from(event));
                    })+
                    return Err(Error::NoEvent($line.to_string()));
                }),+,
                _ => Err(Error::NoEvent($line.to_string()))
            }
        };
    }

    pub(super) use match_event;
}

pub mod regex_patterns {
    include!(concat!(env!("OUT_DIR"), "/regex_patterns.rs"));
}
