use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    str::FromStr,
};

use chrono::NaiveDateTime;
use rayon::prelude::*;
use regex::Regex;
use regex_patterns::LevelNames;

use crate::{
    data::{parse_event, Domain, File, Level, Line, Object, ObjectType},
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

        let line_count = lines.len();

        let file = File {
            id: index as i32,
            path: path.to_string_lossy().to_string(),
        };

        let (lines, objects): (Vec<Line>, HashSet<Object>) = lines
            .into_par_iter()
            .enumerate()
            .map(|(i, line)| {
                let res = self.parse_line(&line, i as u64, &file);

                let Ok((line, object)) = res else {
                    let err = res.unwrap_err();
                    log::trace!("Line parsing error: {}", &err);
                    return Err(err);
                };

                Ok((line, object))
            })
            .filter_map(|res| res.ok())
            .unzip();

        log::debug!(
            "Parsed {}, skipped {} lines from {}",
            lines.len(),
            line_count - lines.len(),
            &file.path
        );

        Ok(ParserOutput {
            file,
            lines,
            objects,
        })
    }

    fn parse_line(&self, line: &str, line_num: u64, file: &File) -> Result<(Line, Object)> {
        let domain = parse_domain(line, &self.patterns.platform.domain)?;

        let object = parse_object(line, &self.patterns.object)?;

        let timestamp = parse_timestamp(
            line,
            &self.patterns.platform.timestamp,
            self.patterns.platform.timestamp_formats.as_slice(),
        )?;

        let level = parse_level(
            line,
            &self.patterns.platform.level,
            &self.patterns.platform.level_names,
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

        let event = parse_event(line, &self.version, &self.patterns)?;

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
                .filter(|path| match regex_patterns::patterns_for_file(&path) {
                    Err(err) => {
                        log::error!("Error validating file {:?}: {}", path, err);
                        false
                    }
                    Ok((_, version)) => {
                        log::debug!("Found valid log file {:?} with version {}", path, version);
                        true
                    }
                })
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

fn parse_level(line: &str, regex: &Regex, level_names: &LevelNames) -> Result<Level> {
    let Some(caps) = regex.captures(line) else {
        return Err(Error::NoLevel(line.to_string()));
    };

    let level_str = caps
        .name("level")
        .ok_or(Error::NoLevel(line.to_string()))?
        .as_str();

    Level::from_str(level_str, level_names)
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
    line: &str,
    timestamp_regex: &Regex,
    timestamp_formats: &[&str],
) -> Result<NaiveDateTime> {
    let Some(caps) = timestamp_regex.captures(line) else {
        return Err(Error::NoTimestamp(line.to_string()));
    };

    let Some(ts_match) = caps.name("ts") else {
        panic!("Regex has no 'ts' capture group!!");
    };

    let ts_str = ts_match.as_str();

    for timestamp_format in timestamp_formats {
        let Ok(ts) = NaiveDateTime::parse_from_str(ts_str, timestamp_format) else {
            continue;
        };
        return Ok(ts);
    }
    Err(Error::NoTimestamp(line.to_string()))
}

pub(crate) fn read_lines(file_path: &Path) -> Result<Vec<String>> {
    if decoder::is_encoded(file_path)? {
        decoder::decode_lines(file_path)
    } else {
        let contents = std::fs::read_to_string(file_path)?;
        Ok(contents.lines().into_iter().map(str::to_string).collect())
    }
}

pub mod regex_patterns {
    include!(concat!(env!("OUT_DIR"), "/regex_patterns.rs"));
}
