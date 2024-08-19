use std::{
    collections::HashMap,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta};
use lazy_static::lazy_static;
use rayon::{iter::Either, prelude::*};
use regex::Regex;
use regex_patterns::{LevelNames, Patterns};

use crate::{
    data::{parse_event, File, Level, Line},
    decoder, Error, Result,
};

pub struct Parser {
    files: Vec<PathBuf>,
    patterns: regex_patterns::Patterns,
    version: semver::Version,
    options: Options,
}

pub struct ParserOutput {
    pub file: File,
    pub lines: Vec<Line>,
}

#[derive(Default, Clone, Copy)]
pub struct Options {
    /// Reduce and coalesce similar log lines in trace output. Useful when dealing with a large number of parsing errors.
    pub reduce_lines: bool,
}

impl Parser {
    pub fn new(path: &Path, options: Options) -> Result<Self> {
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
            options,
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

        let level = level_from_filename(file_name, &self.patterns.platform.level_names);
        let timestamp = timestamp_from_filename(file_name);

        if level.is_none() && self.patterns.platform.level.is_none() {
            return Err(Error::CannotParse(
                "File has no log level and the log format specifies no level regex!".to_string(),
            ));
        }

        let timestamp = if let Some(timestamp) = timestamp {
            Ok(timestamp)
        } else if self.patterns.platform.full_timestamp {
            match parse_timestamp(
                &lines[0],
                &self.patterns.platform.timestamp,
                self.patterns.platform.full_timestamp,
                &self.patterns.platform.timestamp_formats,
            ) {
                Ok(Timestamp::Full(ts)) => Ok(ts),
                Ok(Timestamp::Partial(_)) => unreachable!(),
                Err(err) => Err(err),
            }
        } else {
            let meta = std::fs::metadata(path)?;
            let created_seconds = meta
                .created()?
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| {
                    Error::CannotParse(format!("Could not get creation time for file {:?}!", path))
                })?
                .as_secs();

            DateTime::from_timestamp(created_seconds as i64, 0)
                .map(|dt| dt.naive_utc())
                .ok_or_else(|| {
                    Error::CannotParse(format!("Could not get creation time for file {:?}!", path))
                })
        }?;

        let line_count = lines.len();

        let file = File {
            id: index as i32,
            path: path.to_string_lossy().to_string(),
            level,
            timestamp,
        };

        let do_log_line_errors = log::log_enabled!(log::Level::Trace);

        // Used for reducing and coalescing lines / errors for debugging and building up formats.
        // Can be expensive so only enable when needed. Disabled in release builds.
        #[cfg(debug_assertions)]
        let do_reduce_line_errors = do_log_line_errors && self.options.reduce_lines;

        let results: Vec<LineResult> =
            // For full timestamp, we can parse all lines in parallel.
            lines
                .into_par_iter()
                .enumerate()
                .map(|(i, line)| {
                    let res = self.parse_line(&line, i as u64, &file, file.timestamp.date());

                    let Ok(line) = res else {
                        let err = res.unwrap_err();
                        #[cfg(debug_assertions)]
                        if do_reduce_line_errors {
                            let reduced_line = reduce_line(&line, &self.patterns);
                            return LineResult::Err((err, Some(reduced_line)));
                        } else if do_log_line_errors {
                            return LineResult::Err((err, Some(line)))
                        } else {
                            return LineResult::Err((err, None))
                        }
                        #[cfg(not(debug_assertions))]
                        if do_log_line_errors {
                            return LineResult::Err((err, Some(line)))
                        } else {
                            return LineResult::Err((err, None))
                        }
                    };

                    if self.patterns.platform.full_timestamp {
                        LineResult::Ok(line)
                    } else if line.timestamp < file.timestamp {
                        LineResult::Rollover(line)
                    } else {
                        LineResult::Ok(line)
                    }
                }).collect();

        let (mut ok_results, results): (Vec<Line>, Vec<LineResult>) =
            results.into_par_iter().partition_map(|lr| match lr {
                LineResult::Ok(line) => Either::Left(line),
                other => Either::Right(other),
            });

        let (mut rollover_results, err_results): (Vec<Line>, Vec<(Error, Option<String>)>) =
            results.into_par_iter().partition_map(|lr| match lr {
                LineResult::Rollover(line) => Either::Left(line),
                LineResult::Err((line, error)) => Either::Right((line, error)),
                _ => unreachable!(),
            });

        // For any line where there was potential date rollover, we need to adjust the timestamp.
        rollover_results.par_sort_unstable_by_key(|line| line.line_num);

        let mut additional_days = TimeDelta::days(0);
        for mut line in rollover_results {
            line.timestamp += additional_days;
            if line.timestamp < file.timestamp {
                additional_days += TimeDelta::days(1);
                line.timestamp += TimeDelta::days(1);
            }
            ok_results.push(line);
        }

        let ignored_err_count = err_results
            .par_iter()
            .filter(|(err, _)| matches!(err, Error::NoDomain | Error::IgnoredEvent))
            .count();

        #[cfg(debug_assertions)]
        if do_reduce_line_errors {
            let mut errors: HashMap<String, (Error, usize)> = HashMap::new();

            for (err, line) in err_results {
                if !matches!(err, Error::NoDomain | Error::IgnoredEvent) {
                    let entry = errors.entry(line.unwrap()).or_insert((err, 0));
                    entry.1 += 1;
                }
            }

            for (line, (err, count)) in errors {
                log::trace!(
                    "Failed to parse line {} times with '{}': '{}'",
                    count,
                    err,
                    line
                );
            }
        } else if do_log_line_errors {
            for (err, line) in err_results {
                if !matches!(err, Error::NoDomain | Error::IgnoredEvent) {
                    log::trace!("Failed to parse line with '{}': '{}'", err, line.unwrap());
                }
            }
        }

        #[cfg(not(debug_assertions))]
        if do_log_line_errors {
            for (err, line) in err_results {
                if !matches!(err, Error::NoDomain | Error::IgnoredEvent) {
                    log::trace!("Failed to parse line with '{}': '{}'", err, line.unwrap());
                }
            }
        }

        log::debug!(
            "Parsed {} lines from '{}' ({} CBL lines skipped due to error, {} insignificant lines ignored)",
            ok_results.len(),
            &file.path,
            line_count - ok_results.len() - ignored_err_count,
            ignored_err_count,
        );

        Ok(ParserOutput {
            file,
            lines: ok_results,
        })
    }

    fn parse_line(
        &self,
        line: &str,
        line_num: u64,
        file: &File,
        base_date: NaiveDate,
    ) -> Result<Line> {
        let domain = parse_domain(line, &self.patterns.platform.domain)?;

        let object_path = parse_object(line, &self.patterns.object);

        let timestamp = parse_timestamp(
            line,
            &self.patterns.platform.timestamp,
            self.patterns.platform.full_timestamp,
            self.patterns.platform.timestamp_formats.as_slice(),
        )?;

        let timestamp = match timestamp {
            Timestamp::Partial(ts) => base_date.and_time(ts),
            Timestamp::Full(ts) => ts,
        };

        let level = if let Some(level) = file.level {
            level
        } else {
            parse_level(
                line,
                self.patterns.platform.level.as_ref().unwrap(),
                &self.patterns.platform.level_names,
            )?
        };

        let event = parse_event(line, &self.version, &self.patterns)?;

        let line = Line {
            file_id: file.id,
            line_num: line_num as i32,
            level,
            timestamp,
            domain,
            event_type: event.event_type,
            event_data: event.data,
            object_path,
        };

        Ok(line)
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
                .filter(|path| match regex_patterns::patterns_for_file(path) {
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
        } else {
            match regex_patterns::patterns_for_file(path) {
                Err(err) => {
                    log::error!("Error validating file {:?}: {}", path, err);
                    vec![]
                }
                Ok((_, version)) => {
                    log::debug!("Found valid log file {:?} with version {}", path, version);
                    vec![path.to_path_buf()]
                }
            }
        };
        Ok(files)
    }
}

enum LineResult {
    Ok(Line),
    Rollover(Line),
    /// Error and reduced line
    Err((Error, Option<String>)),
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

fn parse_domain(line: &str, regex: &Regex) -> Result<String> {
    let Some(caps) = regex.captures(line) else {
        return Err(Error::NoDomain);
    };

    let domain_str = caps.name("domain").ok_or(Error::NoDomain)?.as_str();

    Ok(domain_str.to_string())
}

fn parse_level(line: &str, regex: &Regex, level_names: &LevelNames) -> Result<Level> {
    let Some(caps) = regex.captures(line) else {
        return Err(Error::NoLevel);
    };

    let level_str = caps.name("level").ok_or(Error::NoLevel)?.as_str();

    Level::from_str(level_str, level_names)
}

lazy_static! {
    /// Used to strip the actual object class name from the JNI mangled name.
    static ref JNI_OBJECT_REGEX: Regex =
        Regex::new(r#"N\d+litecore\d+(\w+\d)?(?<object>\w+)E"#).unwrap();
}

fn parse_object(line: &str, regex: &Regex) -> Option<String> {
    let caps = regex.captures(line)?;

    let obj_str = caps.name("obj")?.as_str();

    let obj_str = if let Some(obj_caps) = JNI_OBJECT_REGEX.captures(obj_str) {
        obj_caps.name("object").unwrap().as_str()
    } else {
        obj_str
    };

    Some(obj_str.to_string())
}

#[derive(Debug, Clone)]
enum Timestamp {
    Partial(NaiveTime),
    Full(NaiveDateTime),
}

fn parse_timestamp(
    line: &str,
    timestamp_regex: &Regex,
    full_timestamp: bool,
    timestamp_formats: &[&str],
) -> Result<Timestamp> {
    let Some(caps) = timestamp_regex.captures(line) else {
        return Err(Error::NoTimestamp(line.to_string()));
    };

    let Some(ts_match) = caps.name("ts") else {
        panic!("Regex has no 'ts' capture group!!");
    };

    let ts_str = ts_match.as_str();

    for timestamp_format in timestamp_formats {
        if full_timestamp {
            if let Ok(ts) = NaiveDateTime::parse_from_str(ts_str, timestamp_format) {
                return Ok(Timestamp::Full(ts));
            }
        } else if let Ok(ts) = NaiveTime::parse_from_str(ts_str, timestamp_format) {
            return Ok(Timestamp::Partial(ts));
        }
    }
    Err(Error::NoTimestamp(line.to_string()))
}

lazy_static! {
    static ref DOCID_REGEX: Regex = Regex::new(r#"\w+::\w{8}-\w{4}-\w{4}-\w{4}-\w{12}"#).unwrap();
    static ref REVID_REGEX: Regex = Regex::new("(#)?\\d+-\\w{32}").unwrap();
    static ref DICT_REGEX: Regex = Regex::new(r#"\{(\W\w+\W:.*,)*\W\w+\W:.*"#).unwrap();
    static ref QUERY_REGEX: Regex = Regex::new(r#"SELECT fl_result\(.*FROM.*"#).unwrap();
    static ref DIGIT_REGEX: Regex = Regex::new("\\d+").unwrap();
    static ref QUOTE_REGEX: Regex = Regex::new(r#"^'.*'"#).unwrap();
}

fn reduce_line(line: &str, patterns: &Patterns) -> String {
    let domain_end = patterns
        .platform
        .domain
        .find(line)
        .map_or(0, |mat| mat.end());

    let level_end = patterns
        .platform
        .level
        .as_ref()
        .and_then(|re| re.find(line))
        .map_or(0, |mat| mat.end());

    // Strip domain and level from the line
    let line = if domain_end > level_end {
        &line[domain_end..]
    } else {
        &line[level_end..]
    };

    // Strip any dictionaries from the line
    let dict_mat = DICT_REGEX.find(line);
    let line = if let Some(mat) = dict_mat {
        let start = &line[..mat.start()];
        format!("{}{{DICT}}", start)
    } else {
        line.to_string()
    };

    let query_mat = QUERY_REGEX.find(&line);
    let line = if let Some(mat) = query_mat {
        let start = &line[..mat.start()];
        format!("{}{{QUERY}}", start)
    } else {
        line.to_string()
    };

    let is_doc_id = |word: &str| DOCID_REGEX.is_match(word);
    let is_rev_id = |word: &str| REVID_REGEX.is_match(word);
    let is_quoted = |word: &str| QUOTE_REGEX.is_match(word);

    line.split_whitespace()
        .map(|word| {
            if is_doc_id(word) {
                "{DOCID}".to_string()
            } else if is_rev_id(word) {
                "{REVID}".to_string()
            } else if is_quoted(word) {
                "{QUOTED}".to_string()
            } else if word.chars().all(|c| c.is_ascii_hexdigit())
                && !word.chars().all(|c| c.is_ascii_digit())
            {
                "{HEX}".to_string()
            } else if word.chars().any(|c| c.is_ascii_digit()) {
                DIGIT_REGEX.replace_all(word, "{NUMBER}").to_string()
            } else {
                word.to_string()
            }
        })
        .fold(String::new(), |mut acc, word| {
            acc.push_str(&word);
            acc.push(' ');
            acc
        })
}

fn level_from_filename(file_name: &str, level_names: &LevelNames) -> Option<Level> {
    let level_str = file_name.split('_').nth(1)?;
    Level::from_str(level_str, level_names).ok()
}

fn timestamp_from_filename(file_name: &str) -> Option<NaiveDateTime> {
    let ts_str = file_name.split('_').last()?;

    let dt = ts_str
        .parse()
        .ok()
        .and_then(DateTime::from_timestamp_millis)?;

    Some(dt.naive_utc())
}

pub(crate) fn read_lines(file_path: &Path) -> Result<Vec<String>> {
    if decoder::is_encoded(file_path)? {
        decoder::decode_lines(file_path)
    } else {
        let contents = std::fs::read_to_string(file_path)?;
        Ok(contents.lines().map(str::to_string).collect())
    }
}

pub mod regex_patterns {
    include!(concat!(env!("OUT_DIR"), "/regex_patterns.rs"));
}
