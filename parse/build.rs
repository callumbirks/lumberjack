use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
    io::Write,
    path::Path,
};

use regex::Regex;
use util::write_out;

const IN_PATH: &str = "src/patterns/";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/patterns/");
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let regex_out_path = std::path::Path::new(&out_dir).join("regex_patterns.rs");
    let events_out_path = std::path::Path::new(&out_dir).join("events.rs");

    let formats: BTreeMap<Compatibility, Patterns> = parse_yaml();

    create_regex_patterns(regex_out_path.as_path(), &formats);
    create_events(events_out_path.as_path(), &formats);
}

fn create_regex_patterns(out_path: &Path, formats: &BTreeMap<Compatibility, Patterns>) {
    let mut out_file_writer = std::fs::OpenOptions::new()
        .truncate(true)
        .create(true)
        .write(true)
        .open(out_path)
        .unwrap();

    let mut expected_keys: BTreeSet<String> = BTreeSet::new();

    write_out!(
        out_file_writer,
        "use crate::parser::read_lines;\n",
        "use crate::{Error, Result};\n",
        "use lazy_static::lazy_static;\n",
        "use rangemap::RangeMap;\n",
        "use regex::Regex;\n",
        "use semver::Version;\n",
        "use std::path::Path;\n\n",
    );

    write_out!(
        out_file_writer,
        "/// Loop over every line of a file and attempt to match against 'version' regex for all known formats and platforms,\n",
        "/// returning the matching pattern, and the version, if found.\n",
        "pub fn patterns_for_file(path: &Path) -> Result<(Patterns, Version)> {\n",
        "    let lines = read_lines(path)?;\n",
        "    for (_, patterns) in PATTERNS_MAP.iter() {\n",
        "        let mut version_re_cache: Vec<Regex> = vec![];\n",
        "        for line in &lines {\n",
        "            for (index, platform) in patterns.platforms.iter().enumerate() {\n",
        "                let version_re = if index < version_re_cache.len() {\n",
        "                    &version_re_cache[index]\n",
        "                } else {\n",
        "                    let vr = Regex::new(platform.version).unwrap();\n",
        "                    version_re_cache.push(vr);\n",
        "                    &version_re_cache[index]\n",
        "                };\n\n",
        "                let Some(captures) = version_re.captures(line) else {\n",
        "                    continue;\n",
        "                };\n",
        "\n",
        "                let Some(version) = captures.name(\"ver\") else {\n",
        "                    panic!(\"YAML 'version' spec is missing 'ver' capture!\");\n",
        "                };\n",
        "\n",
        "                // TODO: REMOVE TEMP FIX FOR CORE CPPTEST LOGS\n",
        "                let version_str = if version.as_str() == \"3.2\" {\n",
        "                    \"3.2.0\"\n",
        "                } else {\n",
        "                    version.as_str()\n",
        "                };\n",
        "\n",
        "                let version = Version::parse(version_str).map_err(Error::Semver)?;\n",
        "                return PATTERNS_MAP\n",
        "                    .get(&version)\n",
        "                    .map(|patterns| (Patterns::from_strings(patterns, platform), version.clone()))\n",
        "                    .ok_or(Error::UnsupportedVersion(version));\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "    Err(Error::NoMatches)\n",
        "}\n\n"
    );

    write_out!(
        out_file_writer,
        "#[derive(Debug, Clone)]\n",
        "struct PatternStrings {\n",
        "    /// Only used to satisfy RangeMap's requirement that Value implements Eq.\n",
        "    _id: usize,\n",
        "    pub platforms: Vec<&'static PlatformPatternStrings>,\n",
        "    pub object: &'static str,\n",
    );

    for key in formats.first_key_value().unwrap().1.events.keys() {
        write_out!(out_file_writer, "    pub {}: &'static str,\n", args!(key));
    }

    write_out!(out_file_writer, "}\n\n");

    write_out!(
        out_file_writer,
        "#[derive(Debug, Clone)]\n",
        "struct PlatformPatternStrings {\n",
        "    pub version: &'static str,\n",
        "    pub timestamp: &'static str,\n",
        "    pub timestamp_formats: Vec<&'static str>,\n",
        "    pub domain: &'static str,\n",
        "    pub level: &'static str,\n",
        "    pub level_names: LevelNames,\n",
        "}\n\n"
    );

    write_out!(
        out_file_writer,
        "#[derive(Debug, Clone)]\n",
        "pub struct LevelNames {\n",
        "    pub error: &'static str,\n",
        "    pub warn: &'static str,\n",
        "    pub info: &'static str,\n",
        "    pub verbose: &'static str,\n",
        "    pub debug: &'static str,\n",
        "}\n\n"
    );

    write_out!(
        out_file_writer,
        "#[derive(Debug, Clone)]\n",
        "pub struct Patterns {\n",
        "    pub platform: PlatformPatterns,\n",
        "    pub object: Regex,\n",
    );

    for key in formats.first_key_value().unwrap().1.events.keys() {
        expected_keys.insert(key.clone());
        write_out!(out_file_writer, "    pub {}: Regex,\n", args!(key));
    }

    write_out!(out_file_writer, "}\n\n");

    write_out!(
        out_file_writer,
        "#[derive(Debug, Clone)]\n",
        "pub struct PlatformPatterns {\n",
        "    pub version: Regex,\n",
        "    pub timestamp: Regex,\n",
        "    pub timestamp_formats: Vec<&'static str>,\n",
        "    pub domain: Regex,\n",
        "    pub level: Regex,\n",
        "    pub level_names: LevelNames,\n",
        "}\n\n"
    );

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
        "impl From<&PlatformPatternStrings> for PlatformPatterns {\n",
        "    fn from(patterns: &PlatformPatternStrings) -> Self {\n",
        "        PlatformPatterns {\n",
        "            version: Regex::new(patterns.version).unwrap(),\n",
        "            timestamp: Regex::new(patterns.timestamp).unwrap(),\n",
        "            timestamp_formats: patterns.timestamp_formats.clone(),\n",
        "            domain: Regex::new(patterns.domain).unwrap(),\n",
        "            level: Regex::new(patterns.level).unwrap(),\n",
        "            level_names: patterns.level_names.clone(),\n",
        "        }\n",
        "    }\n",
        "}\n\n"
    );

    write_out!(
        out_file_writer,
        "impl Patterns {\n",
        "    fn from_strings(patterns: &PatternStrings, platform: &PlatformPatternStrings) -> Self {\n",
        "        Patterns {\n",
        "            platform: PlatformPatterns::from(platform),\n",
        "            object: Regex::new(patterns.object).unwrap(),\n",
    );

    for key in formats.first_key_value().unwrap().1.events.keys() {
        write_out!(
            out_file_writer,
            "            {}: Regex::new(patterns.{}).unwrap(),\n",
            args!(key, key)
        );
    }

    write_out!(out_file_writer, "        }\n", "    }\n", "}\n\n");

    // TODO: Remove this. We don't need to check for key consistency anymore, because each format can have unique events
    //for (_, patterns) in formats.iter().skip(1) {
    //    // Verify that the keys are the same
    //    for (key, _) in &patterns.events {
    //        if !expected_keys.contains(key) {
    //            panic!("File '{}' has unexpected key '{}'. Make sure all yaml files contain the same keys!", &file_name, key);
    //        }
    //    }
    //    for key in &expected_keys {
    //        if !patterns.events.contains_key(key) {
    //            panic!("File '{}' is missing expected key '{}'. Make sure all yaml files contain the same keys!", &file_name, key);
    //        }
    //    }
    //}

    write_out!(out_file_writer, "lazy_static! {\n",);

    for (pattern_index, (_, Patterns { platforms, .. })) in formats.iter().enumerate() {
        for (platform_index, platform) in platforms.iter().enumerate() {
            write_out!(
                out_file_writer,
                "    static ref PLATFORM_{}_{}: PlatformPatternStrings = PlatformPatternStrings {{\n",
                "        version: r#\"{}\"#,\n",
                "        timestamp: r#\"{}\"#,\n",
                "        timestamp_formats: vec![\n",
                args!(pattern_index, platform_index, platform.version, platform.timestamp)
            );

            for timestamp_format in &platform.timestamp_formats {
                write_out!(
                    out_file_writer,
                    "            r#\"{}\"#,\n",
                    args!(timestamp_format)
                );
            }

            write_out!(
                out_file_writer,
                "        ],\n",
                "        domain: r#\"{}\"#,\n",
                "        level: r#\"{}\"#,\n",
                "        level_names: LevelNames {{\n",
                "            error: r#\"{}\"#,\n",
                "            warn: r#\"{}\"#,\n",
                "            info: r#\"{}\"#,\n",
                "            verbose: r#\"{}\"#,\n",
                "            debug: r#\"{}\"#,\n",
                "        }},\n",
                "    }};\n",
                args!(
                    platform.domain,
                    platform.level,
                    platform.level_names.error,
                    platform.level_names.warn,
                    platform.level_names.info,
                    platform.level_names.verbose,
                    platform.level_names.debug
                )
            );
        }
    }

    write_out!(out_file_writer, "\n");

    for (index, (_, patterns)) in formats.iter().enumerate() {
        write_out!(
            out_file_writer,
            "    static ref PATTERNS_{}: PatternStrings = PatternStrings {{\n",
            "        _id: {},\n",
            "        platforms: vec![\n",
            args!(index, index)
        );

        for platform_index in 0..patterns.platforms.len() {
            write_out!(
                out_file_writer,
                "            &*PLATFORM_{}_{},\n",
                args!(index, platform_index)
            );
        }

        write_out!(
            out_file_writer,
            "        ],\n",
            "        object: r#\"{}\"#,\n",
            args!(patterns.object)
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
            .write_all(
                concat!(
                    "lazy_static! {\n",
                    "    static ref PATTERNS_MAP: RangeMap<Version, &'static PatternStrings> = RangeMap::from([\n"
                )
                .as_bytes(),
            )
            .unwrap();

    for (index, (compatibility, _)) in formats.iter().enumerate() {
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

    write_out!(out_file_writer, "    ]);\n}\n",);
}

fn create_events(out_path: &Path, formats: &BTreeMap<Compatibility, Patterns>) {
    let mut out_file_writer = std::fs::OpenOptions::new()
        .truncate(true)
        .create(true)
        .write(true)
        .open(out_path)
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
        "        _ => Err(Error::UnsupportedVersion(version.clone())),\n",
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

    for key in formats.first_key_value().unwrap().1.events.keys() {
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

    for (index, (_, patterns)) in formats.iter().enumerate() {
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
                for key in captures.keys() {
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
                for key in captures.keys() {
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

fn parse_yaml() -> BTreeMap<Compatibility, Patterns> {
    let in_dir = std::path::Path::new(IN_PATH);

    let filename_regex = Regex::new("(?<from_major>\\d+)-(?<from_minor>\\d+)-(?<from_patch>\\d+)_(?<to_major>\\d+)-(?<to_minor>\\d+)-(?<to_patch>\\d+)").unwrap();

    let mut formats: BTreeMap<Compatibility, Patterns> = BTreeMap::new();

    for dir_entry in std::fs::read_dir(in_dir).unwrap().map(Result::unwrap) {
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

        formats.insert(compatibility, patterns);
    }

    formats
}

#[derive(serde::Deserialize)]
struct Patterns {
    platforms: Vec<PlatformPatterns>,
    object: String,
    events: BTreeMap<String, Event>,
}

#[derive(serde::Deserialize)]
struct PlatformPatterns {
    version: String,
    timestamp: String,
    timestamp_formats: Vec<String>,
    domain: String,
    level: String,
    level_names: LevelNames,
}

#[derive(serde::Deserialize)]
struct LevelNames {
    error: String,
    warn: String,
    info: String,
    verbose: String,
    debug: String,
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
        let Some(captures) = regex.captures(file_name) else {
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
            $writer.write_all(concat!($($string),+).as_bytes()).unwrap();
        };
        ($writer:expr, $($string:literal),+, args!($($args:expr),+$(,)?)$(,)?) => {
            $writer.write_all(format!(concat!($($string),+), $($args),+).as_bytes()).unwrap();
        };
    }

    pub(super) use write_out;
}
