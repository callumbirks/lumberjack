use std::str::FromStr;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use lazy_static::lazy_static;
use rangemap::RangeMap;
use regex::Regex;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::data::{EventType, File, Line, Object, ObjectType};
use crate::parser::model::{Compatibility, Model, Timestamp};
use crate::parser::{CBLVersion, Platform};
use crate::{Error, Result};

pub struct DirParserModel {
    pub compatibility: Compatibility,
    patterns: Patterns,
    regex_cache: RegexCache,
}

impl Model for DirParserModel {
    /// Parse a CBL version string and return `Self` with the correct patterns loaded for that
    /// version.
    fn from_version_string(line: &str) -> Result<Box<Self>> {
        // Iterate over the available sets of patterns. PATTERNS_MAP is a map from a range
        // of version numbers to a preloaded YAML file.
        for (version_range, patterns) in PATTERNS_MAP.iter() {
            let patterns: Patterns = serde_yaml::from_str(patterns)?;
            if let Ok(version) = parse_version(&line, &patterns) {
                return if version_range.contains(&version.version) {
                    let regex_cache = RegexCache {
                        version: Regex::new(&patterns.version)?,
                        timestamp: Regex::new(&patterns.timestamp)?,
                        object: Regex::new(&patterns.object)?,
                    };
                    Ok(Box::new(Self {
                        compatibility: Compatibility::with_versions(version_range.clone()),
                        patterns,
                        regex_cache,
                    }))
                } else {
                    let Some(patterns) = PATTERNS_MAP.get(&version.version) else {
                        return Err(Error::UnsupportedVersion(version.version));
                    };
                    let patterns: Patterns = serde_yaml::from_str(patterns)?;
                    let regex_cache = RegexCache {
                        version: Regex::new(&patterns.version)?,
                        timestamp: Regex::new(&patterns.timestamp)?,
                        object: Regex::new(&patterns.object)?,
                    };
                    Ok(Box::new(Self {
                        compatibility: Compatibility::with_versions(version_range.clone()),
                        patterns,
                        regex_cache,
                    }))
                };
            }
        }
        Err(Error::NoMatches)
    }

    fn parse_timestamp(&self, line: &str) -> crate::Result<Timestamp> {
        let re = &self.regex_cache.timestamp;

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
        let Some(caps) = &self.regex_cache.object.captures(line) else {
            return Err(Error::NoObject(line.to_string()));
        };

        let caps = (caps.name("obj"), caps.name("id"));

        let Some((obj_str, id_str)) = (match caps {
            (Some(obj), Some(id)) => Some((obj.as_str(), id.as_str())),
            _ => None,
        }) else {
            return Err(Error::NoObject(line.to_string()));
        };

        let Some(object_type) = (match obj_str {
            "DB" => Some(ObjectType::DB),
            "Repl" | "repl" => Some(ObjectType::Repl),
            "Pusher" => Some(ObjectType::Pusher),
            "Puller" => Some(ObjectType::Puller),
            "Inserter" => Some(ObjectType::Inserter),
            "BLIPIO" => Some(ObjectType::BLIPIO),
            "IncomingRev" => Some(ObjectType::IncomingRev),
            "Connection" => Some(ObjectType::Connection),
            "C4SocketImpl" => Some(ObjectType::C4SocketImpl),
            "RevFinder" => Some(ObjectType::RevFinder),
            "ReplicatorChangesFeed" => Some(ObjectType::ReplicatorChangesFeed),
            "QueryEnum" => Some(ObjectType::QueryEnum),
            "C4Replicator" => Some(ObjectType::C4Replicator),
            "Housekeeper" => Some(ObjectType::Housekeeper),
            "Shared" => Some(ObjectType::Shared),
            "CollectionImpl" => Some(ObjectType::CollectionImpl),
            "Query" => Some(ObjectType::Query),
            "DBAccess" => Some(ObjectType::DBAccess),
            _ => None,
        }) else {
            return Err(Error::UnknownObject(obj_str.to_string()));
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

fn parse_version(line: &str, patterns: &Patterns) -> Result<CBLVersion> {
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
}

struct RegexCache {
    version: Regex,
    timestamp: Regex,
    object: Regex,
}
