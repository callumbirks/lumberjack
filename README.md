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
