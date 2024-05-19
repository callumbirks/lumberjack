use crate::{Error, Result};
use futures::TryStreamExt;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncBufReadExt;
use tokio_stream::wrappers::LinesStream;
use tokio_stream::StreamExt;
use tokio_util::io::{ReaderStream, StreamReader};

/// Used to neatly match across `str::contains`
/// # Example
/// ```
/// use crate::lumberjack_parse::match_contains;
/// enum Foo {
///     Foo,
///     Bar,
///     FooBar,
/// }
///
/// let foo_mat = |input| { match_contains!(input, {
///     [ "foo", "Foo" ] => Foo::Foo, // "foo" or "Foo"
///     [ "bar", "Bar" ] => Foo::Bar, // "bar" or "Bar"
///     // ("foo" and "bar") or ("Foo" and "Bar")
///     [ "foo" && "bar", "Foo" && "Bar" ] => Foo::FooBar,
/// })};
///
/// assert_eq!(foo_mat("foo"), Some(Foo::Foo));
/// assert_eq!(foo_mat("fooBar"), None);
/// ```
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
