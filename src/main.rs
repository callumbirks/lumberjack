#[cfg(feature = "xlsx")]
mod xlsx;

use clap::Parser;
use std::path::PathBuf;
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
    out_xlsx: bool,
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

    let conn = lumberjack_parse::parse(&args.input).execute()?;

    #[cfg(feature = "xlsx")]
    if args.out_xlsx {
        xlsx::write("test.xlsx", conn)?;
    }

    Ok(())
}
