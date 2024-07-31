#[cfg(feature = "xlsx")]
mod xlsx;

use clap::Parser;
use diesel::{Connection, SqliteConnection};
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
    #[arg(short, long)]
    /// Enable extra verbose logging
    extra_verbose: bool,
}

#[derive(Error, Debug)]
enum Error {
    #[error("Parse Error {0}")]
    Parse(#[from] lumberjack_parse::Error),
    #[error("Diesel Error {0}")]
    Diesel(#[from] diesel::result::Error),
    #[cfg(feature = "xlsx")]
    #[error("Xlsx Error {0}")]
    Xlsx(#[from] rust_xlsxwriter::XlsxError),
    #[error("SQLite Connection Error {0}")]
    SqliteConnection(#[from] diesel::ConnectionError),
}

type Result<T> = std::result::Result<T, Error>;

fn main() -> Result<()> {
    let args = Args::parse();

    let level_filter = if args.extra_verbose {
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

    let current_dir = std::env::current_dir().unwrap();

    let (out_dir, db_file_name) = if let Some(out_path) = args.output {
        if out_path.is_dir() {
            (out_path, "output.sqlite".to_string())
        } else {
            (
                out_path
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or(current_dir),
                sqlite_file_name(&out_path),
            )
        }
    } else {
        (
            std::env::current_dir().unwrap(),
            "output.sqlite".to_string(),
        )
    };

    let db_path = out_dir.with_file_name(&db_file_name);

    lumberjack_parse::parse(&args.input, &db_path)?;

    #[cfg(feature = "xlsx")]
    if args.xlsx {
        let xlsx_filename = Path::new(&db_file_name).with_extension("xlsx");
        let xlsx_path = out_dir.join(xlsx_filename);
        let conn = SqliteConnection::establish(db_path.to_str().unwrap())?;
        xlsx::write(xlsx_path, conn)?;
    }

    Ok(())
}

fn sqlite_file_name(path: &Path) -> String {
    path.with_extension("sqlite")
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}
