# Lumberjack

## 🚧 🚧 🚧 WIP 🚧 🚧 🚧

### What?

A log parser for CouchbaseLite logs.

Accepts binary logs or text logs in several different CBL formats, extracts important data and
 'events' from the logs, and stores the data in a SQLite database (see `parse/src/data/schema.sql`).

To support the different sets of data required by each type of log, event data is stored in JSON.

### How?

Regex for formats, events, and data are defined by the YAML files found in `parse/src/patterns`.
 The files are ingested by the build script (`parse/build.rs`) which generates parsing code,
 including the JSON schemas for each event type, for each format, for each version.

The parser itself will scan the input file(s) to extract version information, find and verify the
 correct "`Patterns`" for that version and CBL platform, then iterate over each input file, parsing
 each line in parallel to extract the necessary data.

## Build

1. Run `cargo build --release`
2. The binary will be located in `target/release/cbl-lumberjack`

Or, run directly using `cargo run --release`, you can then pass arguments using `--`, like: `cargo run --release -- --xlsx --input info.log`.

## Usage

Run `cbl-lumberjack --help` to see the available options.
```
Usage: cbl-lumberjack [OPTIONS] --input <INPUT>

Options:
  -i, --input <INPUT>    The input path of log file(s) to parse
      --xlsx             If specified, output the parsed data to an xlsx file
  -o, --output <OUTPUT>  The output path for the parsed data. A directory or a file name. If a directory is specified, the file name will be chosen by the program. If no output parameter is specified, the files will be output to the current directory
  -v, --verbose          Enable verbose logging
      --trace            Enable trace logging
      --reduce-lines     Reduce and coalesce similar log lines in trace output. Useful when dealing with a large number of parsing errors. Ignored in release builds
  -h, --help             Print help
  -V, --version          Print version
```
