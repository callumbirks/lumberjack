use crate::data::{CommonEvent, EventType, File, Line, Object, ObjectExtra, ObjectType};
use crate::parser::model::{Model, Timestamp};
use crate::parser::{CBLVersion, Platform};
use crate::{parser, Error, Result};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use lazy_static::lazy_static;
use rangemap::RangeMap;
use regex::{Match, Regex};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::ops::{Deref, Range};
use std::str::FromStr;

pub struct DirParserModel {
    patterns: Patterns,
}

impl Model for DirParserModel {
    /// Parse a CBL version string and return `Self` with the correct patterns loaded for that
    /// version.
    fn from_version_string(line: &str) -> crate::Result<Box<Self>> {
        // Iterate over the available sets of patterns. PATTERNS_MAP is a map from a range
        // of version numbers to a preloaded YAML file.
        for (version_range, patterns) in PATTERNS_MAP.iter() {
            let patterns: Patterns = serde_yaml::from_str(patterns)?;
            if let Ok(version) = parse_version(&line, &patterns) {
                return if version_range.contains(&version.version) {
                    Ok(Box::new(Self { patterns }))
                } else {
                    let Some(patterns) = PATTERNS_MAP.get(&version.version) else {
                        return Err(Error::UnsupportedVersion(version.version));
                    };
                    let patterns: Patterns = serde_yaml::from_str(patterns)?;
                    Ok(Box::new(Self { patterns }))
                };
            }
        }
        Err(Error::NoMatches)
    }

    fn parse_timestamp(&self, line: &str) -> crate::Result<Timestamp> {
        let re = Regex::new(&self.patterns.timestamp)?;

        let Some(caps) = re.captures(line) else {
            return Err(Error::NoMatches);
        };

        let Some(ts_match) = caps.name("ts") else {
            return Err(Error::NoMatches);
        };

        let ts_str = ts_match.as_str();

        if self.patterns.full_datetime {
            Ok(Timestamp::DateTime(NaiveDateTime::parse_from_str(
                ts_str,
                &self.patterns.timestamp_format,
            )?))
        } else {
            Ok(Timestamp::Time(NaiveTime::parse_from_str(
                ts_str,
                &self.patterns.timestamp_format,
            )?))
        }
    }

    fn parse_line(
        &self,
        line: &str,
        line_num: usize,
        file: &File,
        base_date: NaiveDate,
    ) -> Result<(Line, Object)> {
        let re = Regex::new(&self.patterns.object)?;
        let Some(caps) = re.captures(line) else {
            return Err(Error::NoMatches);
        };

        let caps = (caps.name("obj"), caps.name("id"));

        let Some((obj_str, id_str)) = (match caps {
            (Some(obj), Some(id)) => Some((obj.as_str(), id.as_str())),
            _ => None,
        }) else {
            return Err(Error::NoMatches);
        };

        let Some(object_type) = (match obj_str {
            "DB" => Some(ObjectType::DB),
            "Repl" | "repl" => Some(ObjectType::Repl),
            "Pusher" => Some(ObjectType::Pusher),
            "Puller" => Some(ObjectType::Puller),
            _ => None,
        }) else {
            return Err(Error::NoMatches);
        };

        let object_id: i32 = id_str.parse()?;

        let timestamp = match self.parse_timestamp(line)? {
            Timestamp::DateTime(dt) => dt,
            Timestamp::Time(t) => base_date.and_time(t),
        };

        Ok((
            Line {
                level: file.level,
                line_num: line_num as i64,
                timestamp,
                message: line.to_string(),
                event_type: EventType::None, // TODO Event parsing
                object_id,
                file_id: file.id,
            },
            Object {
                id: object_id,
                ty: object_type,
            },
        ))
    }
}

fn parse_version(line: &str, patterns: &Patterns) -> crate::Result<CBLVersion> {
    let re = Regex::new(&patterns.version)?;

    let Some(caps) = re.captures(line) else {
        return Err(Error::NoMatches);
    };

    let caps = (
        caps.name("ver"),
        caps.name("plat"),
        caps.name("os"),
        caps.name("build"),
        caps.name("commit"),
    );

    let Some((version, platform, os, build, commit)) = (match caps {
        (Some(v), Some(p), Some(o), Some(b), Some(c)) => Some((v, p, o, b, c)),
        _ => None,
    }) else {
        return Err(Error::NoMatches);
    };

    let (version, platform, os, build, commit) = (
        version.as_str(),
        platform.as_str(),
        os.as_str(),
        build.as_str(),
        commit.as_str(),
    );

    let platform = Platform::from_str(platform)?;
    let version = Version::from_str(version)?;
    let build: u16 = build.parse()?;

    Ok(CBLVersion {
        platform,
        version,
        system: os.to_string(),
        build,
        commit: commit.to_string(),
    })
}

lazy_static! {
    /// A `RangeMap` from a `Range` of `Version` to a string containing a preloaded YAML file
    static ref PATTERNS_MAP: RangeMap<Version, &'static str> = RangeMap::from([(
        Version::new(3, 1, 0)..Version::new(3, 1, 7),
        include_str!("patterns/dir/3-1-0_3-1-7.yaml")
    )]);
}

#[derive(Serialize, Deserialize, Debug)]
struct Patterns {
    version: String,
    full_datetime: bool,
    timestamp: String,
    timestamp_format: String,
    object: String,
    objects: ObjectPatterns,
}

#[derive(Serialize, Deserialize, Debug)]
struct ObjectPatterns {
    db: String,
    repl: String,
    pusher: String,
    puller: String,
}
