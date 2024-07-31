use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
    io::Write,
    path::Path,
};

use regex::Regex;
use util::write_out;

const IN_PATH: &'static str = "src/patterns/";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/patterns/");
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let regex_out_path = std::path::Path::new(&out_dir).join("regex_patterns.rs");
    let events_out_path = std::path::Path::new(&out_dir).join("events.rs");

    let formats: BTreeMap<Compatibility, PatternsFile> = parse_yaml();

    create_regex_patterns(regex_out_path.as_path(), &formats);
    create_events(events_out_path.as_path(), &formats);
}

fn create_regex_patterns(out_path: &Path, formats: &BTreeMap<Compatibility, PatternsFile>) {
    let mut out_file_writer = std::fs::OpenOptions::new()
        .truncate(true)
        .create(true)
        .write(true)
        .open(&out_path)
        .unwrap();

    let mut expected_keys: BTreeSet<String> = BTreeSet::new();

    write_out!(
        out_file_writer,
        "use lazy_static::lazy_static;\n",
        "use rangemap::RangeMap;\n",
        "use regex::Regex;\n",
        "use semver::Version;\n",
        "use std::path::Path;\n",
        "use crate::parser::read_lines;\n",
        "use crate::{Error, Result};\n\n",
    );

    write_out!(
        out_file_writer,
        "pub fn patterns_for_version(version: &Version) -> Result<Patterns> {\n",
        "    PATTERNS_MAP.get(version).map(|p| Patterns::from(*p)).ok_or(Error::UnsupportedVersion(version.clone()))\n",
        "}\n\n"
    );

    write_out!(
        out_file_writer,
        "pub fn patterns_for_file(path: &Path) -> Result<(Patterns, Version)> {\n",
        "    let lines = read_lines(path)?;",
        "    for (_, patterns) in PATTERNS_MAP.iter() {\n",
        "        let versions_re = patterns\n",
        "            .version\n",
        "            .iter()\n",
        "            .map(|r| Regex::new(r).unwrap())\n",
        "            .collect::<Vec<_>>();\n",
        "\n",
        "        for line in &lines {\n",
        "            for version_re in &versions_re {\n",
        "                let Some(captures) = version_re.captures(line) else {\n",
        "                    continue;\n",
        "                };\n",
        "\n",
        "                let Some(version) = captures.name(\"ver\") else {\n",
        "                    panic!(\"YAML 'version' spec is missing 'ver' capture!\");\n",
        "                };\n",
        "\n",
        "                // TODO: TEMP FIX FOR CORE CPPTEST LOGS\n",
        "                let version_str = if version.as_str() == \"3.2\" {\n",
        "                    \"3.2.0\"\n",
        "                } else {\n",
        "                    version.as_str()\n",
        "                };\n",
        "\n",
        "                let version = Version::parse(version_str).map_err(|err| Error::Semver(err))?;\n",
        "                return patterns_for_version(&version).map(|p| (p, version));\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "    Err(Error::NoMatches)\n",
        "}\n\n"
    );

    write_out!(
        out_file_writer,
        "#[derive(Debug, Clone)]\n",
        "pub struct PatternStrings {\n",
        "    /// Only used to satisfy RangeMap's requirement that Value implements Eq.\n",
        "    _id: usize,\n",
        "    pub version: Vec<&'static str>,\n",
        "    pub timestamp: Vec<&'static str>,\n",
        "    pub timestamp_formats: Vec<&'static str>,\n",
        "    pub object: &'static str,\n",
        "    pub domain: &'static str,\n",
    );

    for (key, _) in &formats.first_key_value().unwrap().1.patterns.events {
        write_out!(out_file_writer, "    pub {}: &'static str,\n", args!(key));
    }

    write_out!(out_file_writer, "}\n\n");

    write_out!(
        out_file_writer,
        "#[derive(Debug, Clone)]\n",
        "pub struct Patterns {\n",
        "    pub version: Vec<Regex>,\n",
        "    pub timestamp: Vec<Regex>,\n",
        "    pub timestamp_formats: Vec<&'static str>,\n",
        "    pub object: Regex,\n",
        "    pub domain: Regex,\n",
    );

    for (key, _) in &formats.first_key_value().unwrap().1.patterns.events {
        expected_keys.insert(key.clone());
        write_out!(out_file_writer, "    pub {}: Regex,\n", args!(key));
    }

    write_out!(out_file_writer, "}\n\n");

    write_out!(
        out_file_writer,
        "impl PartialEq for PatternStrings {\n",
        "    fn eq(&self, other: &Self) -> bool {\n",
        "        self._id == other._id\n",
        "    }\n",
        "}\n\n",
        "impl Eq for PatternStrings {}\n\n",
    );

    write_out!(
        out_file_writer,
        "impl From<&PatternStrings> for Patterns {\n",
        "    fn from(patterns: &PatternStrings) -> Self {\n",
        "        Patterns {\n",
        "            version: patterns.version.iter().map(|v| Regex::new(v).unwrap()).collect(),\n",
        "            timestamp: patterns.timestamp.iter().map(|t| Regex::new(t).unwrap()).collect(),\n",
        "            timestamp_formats: patterns.timestamp_formats.clone(),\n",
        "            object: Regex::new(patterns.object).unwrap(),\n",
        "            domain: Regex::new(patterns.domain).unwrap(),\n",
    );

    for (key, _) in &formats.first_key_value().unwrap().1.patterns.events {
        write_out!(
            out_file_writer,
            "            {}: Regex::new(patterns.{}).unwrap(),\n",
            args!(key, key)
        );
    }

    write_out!(out_file_writer, "        }\n", "    }\n", "}\n\n");

    for (
        _,
        PatternsFile {
            file_name,
            patterns,
        },
    ) in formats.iter().skip(1)
    {
        // Verify that the keys are the same
        for (key, _) in &patterns.events {
            if !expected_keys.contains(key) {
                panic!("File '{}' has unexpected key '{}'. Make sure all yaml files contain the same keys!", &file_name, key);
            }
        }
        for key in &expected_keys {
            if !patterns.events.contains_key(key) {
                panic!("File '{}' is missing expected key '{}'. Make sure all yaml files contain the same keys!", &file_name, key);
            }
        }
    }

    write_out!(out_file_writer, "lazy_static! {\n",);

    for (
        index,
        (
            _,
            PatternsFile {
                patterns,
                file_name,
            },
        ),
    ) in formats.iter().enumerate()
    {
        write_out!(
            out_file_writer,
            "    /// Generated from '{}{}'\n",
            "    static ref PATTERNS_{}: PatternStrings = PatternStrings {{\n",
            "        _id: {},\n",
            "        version: vec![\n",
            args!(IN_PATH, file_name, index, index)
        );

        for version in &patterns.version {
            write_out!(out_file_writer, "            r#\"{}\"#,\n", args!(version));
        }

        write_out!(
            out_file_writer,
            "        ],\n",
            "        timestamp: vec![\n"
        );

        for timestamp in &patterns.timestamp {
            write_out!(
                out_file_writer,
                "            r#\"{}\"#,\n",
                args!(timestamp)
            );
        }

        write_out!(
            out_file_writer,
            "        ],\n",
            "        timestamp_formats: vec![\n",
        );

        for timestamp_format in &patterns.timestamp_formats {
            write_out!(
                out_file_writer,
                "            r#\"{}\"#,\n",
                args!(timestamp_format)
            );
        }

        write_out!(out_file_writer, "        ],\n");

        write_out!(
            out_file_writer,
            "        object: r#\"{}\"#,\n",
            "        domain: r#\"{}\"#,\n",
            args!(patterns.object, patterns.domain)
        );

        for (key, Event { regex, .. }) in &patterns.events {
            write_out!(
                out_file_writer,
                "        {}: r#\"{}\"#,\n",
                args!(key, regex)
            );
        }

        write_out!(out_file_writer, "    };\n");
    }

    write_out!(out_file_writer, "}\n\n");

    out_file_writer
            .write(
                concat!(
                    "lazy_static! {\n",
                    "    static ref PATTERNS_MAP: RangeMap<Version, &'static PatternStrings> = RangeMap::from([\n"
                )
                .as_bytes(),
            )
            .unwrap();

    for (index, (compatibility, _)) in formats.into_iter().enumerate() {
        write_out!(
            out_file_writer,
            "        (Version::new({}, {}, {})..Version::new({}, {}, {}), &*PATTERNS_{}),\n",
            args!(
                compatibility.from_ver.major,
                compatibility.from_ver.minor,
                compatibility.from_ver.patch,
                compatibility.to_ver.major,
                compatibility.to_ver.minor,
                compatibility.to_ver.patch,
                index
            )
        );
    }

    out_file_writer.write("    ]);\n}\n".as_bytes()).unwrap();
}

fn create_events(out_path: &Path, formats: &BTreeMap<Compatibility, PatternsFile>) {
    let mut out_file_writer = std::fs::OpenOptions::new()
        .truncate(true)
        .create(true)
        .write(true)
        .open(&out_path)
        .unwrap();

    write_out!(
        out_file_writer,
        "use crate::data::util::{diesel_tosql_transmute, impl_display_debug};\n",
        "use crate::{Result, Error};\n",
        "use crate::parser::regex_patterns::Patterns;\n",
        "use diesel::{sql_types, AsExpression, FromSqlRow};\n",
        "use semver::Version;\n\n",
    );

    write_out!(
        out_file_writer,
        "pub fn parse_event(line: &str, version: &Version, patterns: &Patterns) -> Result<Event> {\n",
    );

    for (index, (compatibility, _)) in formats.iter().enumerate() {
        write_out!(
            out_file_writer,
            "    let ver_from_{} = Version::new({}, {}, {});\n",
            "    let ver_to_{} = Version::new({}, {}, {});\n",
            args!(
                index,
                compatibility.from_ver.major,
                compatibility.from_ver.minor,
                compatibility.from_ver.patch,
                index,
                compatibility.to_ver.major,
                compatibility.to_ver.minor,
                compatibility.to_ver.patch
            )
        );
    }

    write_out!(out_file_writer, "    match version {\n",);

    for index in 0..formats.len() {
        write_out!(
            out_file_writer,
            "        ver if ver >= &ver_from_{} && ver < &ver_to_{} => {{\n",
            "            EventBuilder{}::event_from_line(line, patterns)\n",
            "        }}\n",
            args!(index, index, index,)
        );
    }

    write_out!(
        out_file_writer,
        "        _ => return Err(Error::UnsupportedVersion(version.clone())),\n",
        "    }\n",
        "}\n\n"
    );

    write_out!(
        out_file_writer,
        "#[derive(serde::Serialize, PartialEq, Eq, Debug, Clone)]\n",
        "pub struct Event {\n",
        "    pub event_type: EventType,\n",
        "    /// Optional JSON data. The schema is defined by the event type\n",
        "    pub data: Option<String>\n",
        "}\n\n"
    );

    write_out!(
        out_file_writer,
        "#[derive(AsExpression, FromSqlRow, serde::Serialize, Debug, Copy, Clone, Eq, PartialEq, Hash)]\n",
        "#[repr(i32)]\n",
        "#[diesel(sql_type = sql_types::Integer)]\n",
        "pub enum EventType {\n"
    );

    for (key, _) in &formats.first_key_value().unwrap().1.patterns.events {
        let key = snake_to_pascal_case(key);
        write_out!(out_file_writer, "    {},\n", args!(&key));
    }

    write_out!(out_file_writer, "}\n\n");

    write_out!(
        out_file_writer,
        "diesel_tosql_transmute!(EventType, i32, sql_types::Integer);\n",
        "impl_display_debug!(EventType);\n\n"
    );

    write_out!(
        out_file_writer,
        "trait EventBuilder {\n",
        "    fn event_from_line(line: &str, patterns: &Patterns) -> Result<Event>;\n",
        "}\n\n"
    );

    for (index, (_, PatternsFile { patterns, .. })) in formats.iter().enumerate() {
        write_out!(out_file_writer, "struct EventBuilder{};\n\n", args!(index));
        write_out!(
            out_file_writer,
            "impl EventBuilder for EventBuilder{} {{\n",
            "    fn event_from_line(line: &str, patterns: &Patterns) -> Result<Event> {{\n",
            args!(index)
        );
        for (event_key, Event { captures, .. }) in &patterns.events {
            if let Some(captures) = captures {
                write_out!(
                    out_file_writer,
                    "        #[derive(serde::Serialize)]\n",
                    "        struct {} {{\n",
                    args!(snake_to_pascal_case(event_key))
                );
                for (key, capture_type) in captures {
                    write_out!(
                        out_file_writer,
                        "            {}: {},\n",
                        args!(key, capture_type)
                    );
                }
                write_out!(out_file_writer, "        }\n");
            }
        }

        for (event_key, Event { captures, .. }) in &patterns.events {
            if let Some(captures) = captures {
                write_out!(
                    out_file_writer,
                    "        if let Some(captures) = patterns.{}.captures(line) {{\n",
                    "            let (",
                    args!(event_key)
                );
                for (key, _) in captures {
                    write_out!(out_file_writer, "{}, ", args!(key));
                }
                write_out!(out_file_writer, ") = (\n");
                for (key, capture_type) in captures {
                    match capture_type {
                        CaptureType::Bool => {
                            write_out!(
                                out_file_writer,
                                "                {{\n",
                                "                    captures\n",
                                "                        .name(\"{}\")\n",
                                "                        .and_then(|m| m.as_str().parse::<i16>().ok())\n",
                                "                        .unwrap()\n",
                                "                        != 0\n",
                                "                }},\n",
                                args!(key)
                            );
                        }
                        _ => {
                            write_out!(
                                out_file_writer,
                                "                {{\n",
                                "                    captures\n",
                                "                        .name(\"{}\")\n",
                                "                        .and_then(|m| m.as_str().parse::<{}>().ok())\n",
                                "                        .unwrap()\n",
                                "                }},\n",
                                args!(key, capture_type)
                            );
                        }
                    }
                }
                write_out!(
                    out_file_writer,
                    "            );\n",
                    "            let data = {} {{\n",
                    args!(snake_to_pascal_case(event_key))
                );
                for (key, _) in captures {
                    write_out!(out_file_writer, "                {},\n", args!(key));
                }
                write_out!(
                    out_file_writer,
                    "            }};\n",
                    "            return Ok(Event {{\n",
                    "                event_type: EventType::{},\n",
                    "                data: Some(serde_json::to_string(&data).unwrap())\n",
                    "            }});\n",
                    args!(snake_to_pascal_case(event_key))
                );
                write_out!(out_file_writer, "        }\n");
            } else {
                write_out!(
                    out_file_writer,
                    "        if patterns.{}.is_match(line) {{\n",
                    "            return Ok(Event {{\n",
                    "                event_type: EventType::{},\n",
                    "                data: None\n",
                    "            }});\n",
                    "        }}\n",
                    args!(event_key, snake_to_pascal_case(event_key))
                );
            }
        }

        write_out!(
            out_file_writer,
            "        Err(Error::NoEvent(line.to_string()))\n",
            "    }\n",
            "}\n"
        );
    }
}

fn parse_yaml() -> BTreeMap<Compatibility, PatternsFile> {
    let in_dir = std::path::Path::new(IN_PATH);

    let filename_regex = Regex::new("(?<from_major>\\d+)-(?<from_minor>\\d+)-(?<from_patch>\\d+)_(?<to_major>\\d+)-(?<to_minor>\\d+)-(?<to_patch>\\d+)").unwrap();

    let mut formats: BTreeMap<Compatibility, PatternsFile> = BTreeMap::new();

    for (_, dir_entry) in std::fs::read_dir(&in_dir)
        .unwrap()
        .map(Result::unwrap)
        .enumerate()
    {
        if dir_entry.file_type().unwrap().is_dir() {
            continue;
        }

        match dir_entry
            .path()
            .extension()
            .map(|ext| ext.to_str().unwrap())
        {
            Some("yaml") | Some("yml") => (),
            _ => continue,
        };

        let file_name = dir_entry.file_name().into_string().unwrap();
        let compatibility = Compatibility::from_file_name(&filename_regex, &file_name);

        let file_contents = std::fs::read_to_string(dir_entry.path()).unwrap();
        let patterns: Patterns = serde_yaml::from_str(&file_contents).unwrap();

        formats.insert(
            compatibility,
            PatternsFile {
                file_name,
                patterns,
            },
        );
    }

    formats
}

#[derive(serde::Deserialize)]
struct Patterns {
    version: Vec<String>,
    timestamp: Vec<String>,
    timestamp_formats: Vec<String>,
    object: String,
    domain: String,
    level: Option<String>,
    events: BTreeMap<String, Event>,
}

struct PatternsFile {
    file_name: String,
    patterns: Patterns,
}

#[derive(serde::Deserialize)]
enum CaptureType {
    Bool,
    Char,
    Int,
    String,
}

impl Display for CaptureType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CaptureType::Bool => write!(f, "bool"),
            CaptureType::Char => write!(f, "char"),
            CaptureType::Int => write!(f, "i64"),
            CaptureType::String => write!(f, "String"),
        }
    }
}

#[derive(serde::Deserialize)]
struct Event {
    regex: String,
    captures: Option<BTreeMap<String, CaptureType>>,
}

fn snake_to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|s| {
            let mut c = s.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().chain(c).collect(),
            }
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Compatibility {
    from_ver: semver::Version,
    to_ver: semver::Version,
}

impl Compatibility {
    fn from_file_name(regex: &Regex, file_name: &str) -> Compatibility {
        let Some(captures) = regex.captures(&file_name) else {
            panic!("Invalid file name: '{}'. File name should match the pattern '<major>-<minor>-<patch>_<major>-<minor>-<patch>'", &file_name);
        };

        let from_ver = {
            let major = captures
                .name("from_major")
                .unwrap()
                .as_str()
                .parse()
                .unwrap();
            let minor = captures
                .name("from_minor")
                .unwrap()
                .as_str()
                .parse()
                .unwrap();
            let patch = captures
                .name("from_patch")
                .unwrap()
                .as_str()
                .parse()
                .unwrap();
            semver::Version::new(major, minor, patch)
        };

        let to_ver = {
            let major = captures.name("to_major").unwrap().as_str().parse().unwrap();
            let minor = captures.name("to_minor").unwrap().as_str().parse().unwrap();
            let patch = captures.name("to_patch").unwrap().as_str().parse().unwrap();
            semver::Version::new(major, minor, patch)
        };

        Compatibility { from_ver, to_ver }
    }
}

mod util {
    macro_rules! write_out {
        ($writer:expr, $($string:literal),+$(,)?) => {
            $writer.write(concat!($($string),+).as_bytes()).unwrap();
        };
        ($writer:expr, $($string:literal),+, args!($($args:expr),+$(,)?)$(,)?) => {
            $writer.write(format!(concat!($($string),+), $($args),+).as_bytes()).unwrap();
        };
    }

    pub(super) use write_out;
}
