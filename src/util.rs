use crate::error::LumberjackError;
use crate::parse::LogParser;
use enum_iterator::Sequence;
use std::fmt::{Display, Formatter};
use std::path::Path;
use std::sync::Arc;

pub async fn open_folder() -> crate::Result<LogParser> {
    let picked = rfd::AsyncFileDialog::new()
        .set_title("Open a cbllog directory...")
        .pick_folder()
        .await
        .ok_or(LumberjackError::DirectoryInvalid)?;

    LogParser::with_dir(picked.path()).await
}

pub async fn read_file(path: &Path) -> crate::Result<Box<str>> {
    tokio::fs::read_to_string(path)
        .await
        .map(String::into_boxed_str)
        .map_err(|err| LumberjackError::Io(err.kind()))
}

pub async fn read_lines(path: &Path) -> crate::Result<Box<[Arc<str>]>> {
    let contents = tokio::fs::read_to_string(path).await?.into_boxed_str();

    Ok(contents.lines().map(|s| Arc::from(s)).collect())
}

pub trait Truncate {
    fn truncate(&self, max_len: usize) -> &Self;
}

impl Truncate for str {
    fn truncate(&self, max_len: usize) -> &Self {
        match self.char_indices().nth(max_len) {
            None => self,
            Some((idx, _)) => &self[..idx],
        }
    }
}

pub trait ContainsWithCase {
    /// If the pattern is lowercase, ignore case for matching.
    /// Otherwise, match is case-sensitive.
    fn contains_with_case(&self, pattern: &str) -> bool;
}

impl ContainsWithCase for str {
    fn contains_with_case(&self, pattern: &str) -> bool {
        if pattern
            .chars()
            .all(|x| x.is_lowercase() || !x.is_alphabetic())
        {
            self.to_lowercase().contains(pattern)
        } else {
            self.contains(pattern)
        }
    }
}

/// Used to neatly match across `str::contains`
/// # Example
/// ```
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

#[macro_export]
macro_rules! enum_impl_display {
    {$(#[derive($($derive:ident),+)])?
        pub enum $enum_name:ident {
        $($variant:ident => $str:literal),+
        $(;$($nested:ident($inner:ty) => $nest_str:literal),*)?
    }} => (
        $(#[derive($($derive),+)])?
        pub enum $enum_name {
            $($variant),+,
            $($($nested($inner)),*)?
        }

        impl Display for $enum_name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                match *self {
                    $($enum_name::$variant => write!(f, $str)),+,
                    $($($enum_name::$nested(x) => write!(f, $nest_str, x)),*)?
                }
            }
        }
    );
}
