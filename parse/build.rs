use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
};

use regex::Regex;
use util::write_out;

const IN_PATH: &'static str = "src/patterns/";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/patterns/");

    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let out_path = std::path::Path::new(&out_dir).join("regex_patterns.rs");
    let mut out_file_writer = std::fs::OpenOptions::new()
        .truncate(true)
        .create(true)
        .write(true)
        .open(&out_path)
        .unwrap();

    write_out!(
        out_file_writer,
        "use lazy_static::lazy_static;\n",
        "use rangemap::RangeMap;\n",
        "use regex::Regex;\n",
        "use semver::Version;\n",
        "use std::path::Path;\n\n"
    );

    write_out!(
        out_file_writer,
        "#[derive(Debug)]\n",
        "pub enum Error {\n",
        "    Io(std::io::Error),\n",
        "    Semver(semver::Error),\n",
        "    UnsupportedVersion(Version),\n",
        "    NoMatches,\n",
        "}\n\n",
    );

    write_out!(
        out_file_writer,
        "pub fn patterns_for_version(version: &Version) -> Result<Patterns, Error> {\n",
        "    PATTERNS_MAP.get(version).map(|p| Patterns::from(*p)).ok_or(Error::UnsupportedVersion(version.clone()))\n",
        "}\n\n"
    );

    write_out!(
        out_file_writer,
        "pub fn patterns_for_file(path: &Path) -> Result<(Patterns, Version), Error> {\n",
        "    let file_contents = std::fs::read_to_string(path).map_err(|err| Error::Io(err))?;\n",
        "    let lines = file_contents.lines().collect::<Vec<_>>();\n",
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
        "                let version = Version::parse(version.as_str()).map_err(|err| Error::Semver(err))?;\n",
        "                return patterns_for_version(&version).map(|p| (p, version));\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "    Err(Error::NoMatches)\n",
        "}\n\n"
    );

    let in_dir = std::path::Path::new(IN_PATH);

    let filename_regex = Regex::new("(?<from_major>\\d+)-(?<from_minor>\\d+)-(?<from_patch>\\d+)_(?<to_major>\\d+)-(?<to_minor>\\d+)-(?<to_patch>\\d+)").unwrap();

    let mut expected_keys: BTreeSet<String> = BTreeSet::new();

    struct PatternsFile {
        file_name: String,
        patterns: Patterns,
    }

    let mut formats: BTreeMap<Compatibility, PatternsFile> = BTreeMap::new();

    for (_index, dir_entry) in std::fs::read_dir(&in_dir)
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

    write_out!(
        out_file_writer,
        "#[derive(Debug, Clone)]\n",
        "pub struct PatternStrings {\n",
        "    /// Only used to satisfy RangeMap's requirement that Value implements Eq.\n",
        "    _id: usize,\n",
        "    pub version: Vec<&'static str>,\n",
        "    pub timestamp: Vec<&'static str>,\n",
        "    pub timestamp_formats: Vec<&'static str>,\n",
    );

    for (key, _) in &formats.first_key_value().unwrap().1.patterns.patterns {
        write_out!(out_file_writer, "    pub {}: &'static str,\n", args!(key));
    }

    write_out!(out_file_writer, "}\n\n");

    write_out!(
        out_file_writer,
        "#[derive(Debug, Clone)]\n",
        "pub struct Patterns {\n",
        "    pub version: Vec<Regex>,\n",
        "    pub timestamp: Vec<Regex>,\n",
        "    pub timestamp_formats: Vec<&'static str>,\n"
    );

    for (key, _) in &formats.first_key_value().unwrap().1.patterns.patterns {
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
    );

    for (key, _) in &formats.first_key_value().unwrap().1.patterns.patterns {
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
        for (key, _) in &patterns.patterns {
            if !expected_keys.contains(key) {
                panic!("File '{}' has unexpected key '{}'. Make sure all yaml files contain the same keys!", &file_name, key);
            }
        }
        for key in &expected_keys {
            if !patterns.patterns.contains_key(key) {
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

        for (key, value) in &patterns.patterns {
            write_out!(
                out_file_writer,
                "        {}: r#\"{}\"#,\n",
                args!(key, value)
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

#[derive(serde::Deserialize)]
struct Patterns {
    version: Vec<String>,
    timestamp: Vec<String>,
    timestamp_formats: Vec<String>,
    patterns: BTreeMap<String, String>,
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
