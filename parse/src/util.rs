use crate::{Error, Result};
use futures::TryStreamExt;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncBufReadExt;
use tokio_stream::wrappers::LinesStream;
use tokio_stream::StreamExt;
use tokio_util::io::{ReaderStream, StreamReader};

/// Used to neatly match across `str::contains`
#[cfg(test)]
#[macro_export]
macro_rules! match_contains {
    ($input:expr, {
        $([$($($comp:literal)&&+),+] => $mat:expr),+$(,)*
    }) => {
        match $input {
            $(x if $(($(x.contains($comp))&&+))||+ => Some($mat)),+,
            _ => None
        }
    }
}

pub(crate) async fn read_lines(file_path: impl AsRef<Path>) -> Result<Vec<String>> {
    let file = File::open(file_path).await?;
    let stream = ReaderStream::new(file);
    let read = StreamReader::new(stream);
    let lines_stream = LinesStream::new(read.lines());
    lines_stream.map_err(Error::Io).collect().await
}
