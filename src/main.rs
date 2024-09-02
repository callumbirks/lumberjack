#[cfg(feature = "xlsx")]
mod xlsx;

use clap::Parser;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    /// The input path of log file(s) to parse
    input: PathBuf,
    #[cfg(feature = "xlsx")]
    #[arg(long, default_value_t = false)]
    /// If specified, output the parsed data to an xlsx file
    xlsx: bool,
    #[arg(short, long)]
    /// The output path for the parsed data.
    /// A directory or a file name. If a directory is specified, the file name will be chosen by the program.
    /// If no output parameter is specified, the files will be output to the current directory.
    output: Option<PathBuf>,
    #[arg(short, long)]
    /// Enable verbose logging
    verbose: bool,
    #[arg(long)]
    /// Enable trace logging
    trace: bool,
    #[arg(long)]
    /// Reduce and coalesce similar log lines in trace output. Useful when dealing with a large number of parsing errors.
    /// Ignored in release builds.
    reduce_lines: bool,
}

#[derive(Error, Debug)]
enum Error {
    #[error("SQLite Error {0}")]
    SQLite(#[from] rusqlite::Error),
    #[error("Parse Error {0}")]
    Parse(#[from] lumberjack_parse::Error),
    #[cfg(feature = "xlsx")]
    #[error("Xlsx Error {0}")]
    Xlsx(#[from] rust_xlsxwriter::XlsxError),
}

type Result<T> = std::result::Result<T, Error>;

fn main() -> Result<()> {
    let args = Args::parse();

    let level_filter = if args.trace {
        log::LevelFilter::Trace
    } else if args.verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    env_logger::builder()
        .format_timestamp_millis()
        .filter_level(level_filter)
        .init();

    let Options {
        in_dir,
        out_dir,
        db_file_name,
    } = resolve_args(&args);

    let db_path = out_dir.join(&db_file_name);

    let parser_options = lumberjack_parse::Options {
        reduce_lines: args.reduce_lines,
    };

    lumberjack_parse::parse(&in_dir, &db_path, parser_options)?;

    #[cfg(feature = "xlsx")]
    if args.xlsx {
        let xlsx_filename = Path::new(&db_file_name).with_extension("xlsx");
        let xlsx_path = out_dir.join(xlsx_filename);
        let conn = rusqlite::Connection::open(&db_path)?;
        xlsx::write(xlsx_path, conn)?;
    }

    Ok(())
}

struct Options {
    in_dir: PathBuf,
    out_dir: PathBuf,
    db_file_name: String,
}

fn resolve_args(args: &Args) -> Options {
    let current_dir = std::env::current_dir().unwrap();

    let (out_dir, db_file_name) = if let Some(out_path) = &args.output {
        if out_path.is_dir() {
            (out_path.clone(), "output.sqlite".to_string())
        } else {
            (
                out_path
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or(current_dir.clone()),
                sqlite_file_name(out_path),
            )
        }
    } else {
        (
            std::env::current_dir().unwrap(),
            "output.sqlite".to_string(),
        )
    };

    let out_dir = if out_dir.is_relative() {
        current_dir.join(out_dir)
    } else {
        out_dir
    };

    let in_dir = if args.input.is_relative() {
        current_dir.join(&args.input)
    } else {
        args.input.clone()
    };

    if !out_dir.exists() {
        panic!("Output directory does not exist: {:?}", out_dir)
    }

    Options {
        in_dir,
        out_dir,
        db_file_name,
    }
}

fn sqlite_file_name(path: &Path) -> String {
    path.with_extension("sqlite")
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}
