use crate::parser::model::Model;
use crate::parser::{CBLVersion, Platform};
use crate::{parser, Error};
use lazy_static::lazy_static;
use rangemap::RangeMap;
use regex::Regex;
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
    fn from_version_string(line: String) -> crate::Result<Box<Self>> {
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

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Patterns {
    version: String,
}
