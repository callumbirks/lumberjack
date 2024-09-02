use std::{
    collections::{BTreeMap, BTreeSet},
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

    write_out!(
        out_file_writer,
        "use crate::parser::read_lines;\n",
        "use crate::{Error, Result};\n",
        "use lazy_static::lazy_static;\n",
        "use rangemap::RangeMap;\n",
        "use regex::Regex;\n",
        "use semver::Version;\n",
        "use std::path::Path;\n",
        "use std::collections::HashMap;\n\n",
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
        "                return Ok((pattern_for_version(line, version.clone())?, version));\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "    Err(Error::NotLogs(path.to_path_buf()))\n",
        "}\n\n"
    );

    write_out!(
        out_file_writer,
        "/// Just because a version matched against a pattern, it doesn't mean the pattern is for the correct version.\n",
        "/// We need to fetch the correct pattern for the version, then get the right platform for that version.\n",
        "fn pattern_for_version(line: &str, version: Version) -> Result<Patterns> {\n",
        "    let pattern = PATTERNS_MAP\n",
        "        .get(&version)\n",
            "        .ok_or(Error::UnsupportedVersion(version))?;\n",
        "    for platform in pattern.platforms.iter() {\n",
        "        let version_re = Regex::new(platform.version).unwrap();\n",
        "        let Some(capture) = version_re.captures(line) else {\n",
        "            continue;\n",
        "        };\n",
        "        assert!(\n",
        "            capture.name(\"ver\").is_some(),\n",
        "            \"YAML 'version' spec is missing 'ver' capture!\"\n",
        "        );\n",
        "\n",
        "        return Ok(Patterns::from_strings(pattern, platform));\n",
        "    }\n",
        "    Err(Error::UnsupportedPlatform(line.to_string()))\n",
        "}\n\n",
    );

    write_out!(
        out_file_writer,
        "#[derive(Debug, Clone)]\n",
        "struct PatternStrings {\n",
        "    /// Only used to satisfy RangeMap's requirement that Value implements Eq.\n",
        "    _id: usize,\n",
        "    pub platforms: Vec<&'static PlatformPatternStrings>,\n",
        "    pub object: &'static str,\n",
        "    pub events: HashMap<&'static str, &'static str>,\n",
        "}\n\n",
    );

    write_out!(
        out_file_writer,
        "#[derive(Debug, Clone)]\n",
        "struct PlatformPatternStrings {\n",
        "    pub version: &'static str,\n",
        "    pub timestamp: &'static str,\n",
        "    pub full_timestamp: bool,\n",
        "    pub timestamp_formats: Vec<&'static str>,\n",
        "    pub domain: &'static str,\n",
        "    pub level: Option<&'static str>,\n",
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
        "    pub events: HashMap<&'static str, Regex>,\n",
        "}\n\n",
    );

    write_out!(
        out_file_writer,
        "#[derive(Debug, Clone)]\n",
        "pub struct PlatformPatterns {\n",
        "    pub version: Regex,\n",
        "    pub timestamp: Regex,\n",
        "    pub full_timestamp: bool,\n",
        "    pub timestamp_formats: Vec<&'static str>,\n",
        "    pub domain: Regex,\n",
        "    pub level: Option<Regex>,\n",
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
        "            full_timestamp: patterns.full_timestamp,\n",
        "            timestamp_formats: patterns.timestamp_formats.clone(),\n",
        "            domain: Regex::new(patterns.domain).unwrap(),\n",
        "            level: patterns.level.map(|s| Regex::new(s).unwrap()),\n",
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
        "            events: patterns.events.iter().map(|(k, v)| (*k, Regex::new(v).unwrap())).collect(),\n",
        "        }\n",
        "    }\n",
        "}\n\n",
    );

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
                "        full_timestamp: {},\n",
                "        domain: r#\"{}\"#,\n",
                args!(platform.full_timestamp, platform.domain)
            );

            if let Some(level) = &platform.level {
                write_out!(
                    out_file_writer,
                    "        level: Some(r#\"{}\"#),\n",
                    args!(level)
                );
            } else {
                write_out!(out_file_writer, "        level: None,\n");
            }

            write_out!(
                out_file_writer,
                "        level_names: LevelNames {{\n",
                "            error: r#\"{}\"#,\n",
                "            warn: r#\"{}\"#,\n",
                "            info: r#\"{}\"#,\n",
                "            verbose: r#\"{}\"#,\n",
                "            debug: r#\"{}\"#,\n",
                "        }},\n",
                "    }};\n",
                args!(
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
            "        events: HashMap::from([\n",
            args!(patterns.object)
        );

        for (key, Event { regex, .. }) in &patterns.events {
            write_out!(
                out_file_writer,
                "            (\"{}\", r#\"{}\"#),\n",
                args!(key, regex)
            );
        }

        write_out!(out_file_writer, "      ]),\n", "    };\n");
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
        "use crate::data::util::impl_display_debug;\n",
        "use crate::{Result, Error};\n",
        "use crate::parser::regex_patterns::Patterns;\n",
        "use semver::Version;\n\n",
    );

    write_out!(
        out_file_writer,
        "pub fn parse_event(line: &str, version: &Version, patterns: &Patterns) -> Result<Event> {\n",
    );

    write_out!(out_file_writer, "    match version {\n",);

    for (index, (compatibility, _)) in formats.iter().enumerate() {
        write_out!(
            out_file_writer,
            "        ver if ver >= &Version::new({}, {}, {}) && ver < &Version::new({}, {}, {}) => {{\n",
            "            EventBuilder{}::event_from_line(line, patterns)\n",
            "        }}\n",
            args!(
                compatibility.from_ver.major,
                compatibility.from_ver.minor,
                compatibility.from_ver.patch,
                compatibility.to_ver.major,
                compatibility.to_ver.minor,
                compatibility.to_ver.patch,
                index,
            )
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
        "#[derive(PartialEq, Eq, Debug, Clone)]\n",
        "pub struct Event {\n",
        "    pub event_type: EventType,\n",
        "    /// Optional JSON data. The schema is defined by the event type\n",
        "    pub data: Option<String>\n",
        "}\n\n"
    );

    write_out!(
        out_file_writer,
        "#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, serde::Serialize, enum_iterator::Sequence)]\n",
        "#[repr(u32)]\n",
        "pub enum EventType {\n"
    );

    let all_event_keys = formats
        .iter()
        .flat_map(|(_, patterns)| patterns.events.keys())
        .collect::<BTreeSet<_>>();

    for key in all_event_keys {
        let key = snake_to_pascal_case(key);
        write_out!(out_file_writer, "    {},\n", args!(&key));
    }

    write_out!(out_file_writer, "}\n\n");

    write_out!(out_file_writer, "impl_display_debug!(EventType);\n\n");

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
                        args!(key, capture_type.json_type())
                    );
                }
                write_out!(out_file_writer, "        }\n");
            }
        }

        for (
            event_key,
            Event {
                captures, ignore, ..
            },
        ) in &patterns.events
        {
            if ignore.is_some_and(|i| i) {
                write_out!(
                    out_file_writer,
                    "        if patterns.events[\"{}\"].is_match(line) {{\n",
                    "            return Err(Error::IgnoredEvent);\n",
                    "        }}\n",
                    args!(event_key)
                );
            } else if let Some(captures) = captures {
                write_out!(
                    out_file_writer,
                    "        if let Some(captures) = patterns.events[\"{}\"].captures(line) {{\n",
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
                        CaptureType::HexInt => {
                            write_out!(
                                out_file_writer,
                                "                {{\n",
                                "                    captures\n",
                                "                        .name(\"{}\")\n",
                                "                        .and_then(|m| i64::from_str_radix(m.as_str(), 16).ok())\n",
                                "                        .unwrap()\n",
                                "                }},\n",
                                args!(key)
                            );
                        }
                        CaptureType::OptionalInt => {
                            write_out!(
                                out_file_writer,
                                "                {{\n",
                                "                    captures\n",
                                "                        .name(\"{}\")\n",
                                "                        .and_then(|m| m.as_str().parse::<{}>().ok())\n",
                                "                }},\n",
                                args!(key, capture_type.parse_type())
                            );
                        }
                        CaptureType::OptionalString => {
                            write_out!(
                                out_file_writer,
                                "                {{\n",
                                "                    captures\n",
                                "                        .name(\"{}\")\n",
                                "                        .and_then(|m| m.as_str().parse::<{}>().ok())\n",
                                "                        .and_then(|s| if s.is_empty() {{ None }} else {{ Some(s) }})\n",
                                "                }},\n",
                                args!(key, capture_type.parse_type())
                            );
                        }
                        CaptureType::DefaultedInt(default) => {
                            write_out!(
                                out_file_writer,
                                "                {{\n",
                                "                    captures\n",
                                "                        .name(\"{}\")\n",
                                "                        .and_then(|m| m.as_str().parse::<{}>().ok())\n",
                                "                        .unwrap_or({})\n",
                                "                }},\n",
                                args!(key, capture_type.parse_type(), default)
                            );
                        }
                        CaptureType::DefaultedFloat(default) => {
                            write_out!(
                                out_file_writer,
                                "                {{\n",
                                "                    captures\n",
                                "                        .name(\"{}\")\n",
                                "                        .and_then(|m| m.as_str().parse::<{}>().ok())\n",
                                "                        .unwrap_or({:?})\n",
                                "                }},\n",
                                args!(key, capture_type.parse_type(), default)
                            );
                        }
                        CaptureType::DefaultedString(default) => {
                            write_out!(
                                out_file_writer,
                                "                {{\n",
                                "                    captures\n",
                                "                        .name(\"{}\")\n",
                                "                        .map(|m| m.as_str().parse::<{}>())\n",
                                "                        .unwrap_or_else(|| \"{}\".to_string())\n",
                                "                }},\n",
                                args!(key, capture_type.parse_type(), default)
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
                                args!(key, capture_type.parse_type())
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
                    "        if patterns.events[\"{}\"].is_match(line) {{\n",
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
            "        Err(Error::UnknownEvent)\n",
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
    full_timestamp: bool,
    timestamp_formats: Vec<String>,
    domain: String,
    level: Option<String>,
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
    HexInt,
    Float,
    String,
    OptionalInt,
    OptionalString,
    DefaultedInt(i64),
    DefaultedFloat(f64),
    DefaultedString(String),
}

impl CaptureType {
    fn parse_type(&self) -> &'static str {
        match self {
            CaptureType::Bool => "bool",
            CaptureType::Char => "char",
            CaptureType::Int => "i64",
            CaptureType::HexInt => "i64",
            CaptureType::Float => "f64",
            CaptureType::String => "String",
            CaptureType::OptionalInt => "i64",
            CaptureType::OptionalString => "String",
            CaptureType::DefaultedInt(_) => "i64",
            CaptureType::DefaultedFloat(_) => "f64",
            CaptureType::DefaultedString(_) => "String",
        }
    }

    fn json_type(&self) -> &'static str {
        match self {
            CaptureType::Bool => "bool",
            CaptureType::Char => "char",
            CaptureType::Int => "i64",
            CaptureType::HexInt => "i64",
            CaptureType::Float => "f64",
            CaptureType::String => "String",
            CaptureType::OptionalInt => "Option<i64>",
            CaptureType::OptionalString => "Option<String>",
            CaptureType::DefaultedInt(_) => "i64",
            CaptureType::DefaultedFloat(_) => "f64",
            CaptureType::DefaultedString(_) => "String",
        }
    }
}

#[derive(serde::Deserialize)]
struct Event {
    regex: String,
    captures: Option<BTreeMap<String, CaptureType>>,
    ignore: Option<bool>,
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
