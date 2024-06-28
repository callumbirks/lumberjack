#[cfg(feature = "xlsx")]
mod xlsx;

use clap::Parser;
use log::LevelFilter;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input: PathBuf,
    #[cfg(feature = "xlsx")]
    #[arg(long, default_value_t = false)]
    out_xlsx: bool,
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

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder().format_timestamp_millis().init();
    let args = Args::parse();

    let conn = lumberjack_parse::parse(&args.input).execute().await?;

    #[cfg(feature = "xlsx")]
    if args.out_xlsx {
        xlsx::write("test.xlsx", conn)?;
    }

    Ok(())
}
