use crate::Result;
use std::path::Path;

/// Used to neatly match across `str::contains`
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

pub(crate) fn read_lines(file_path: impl AsRef<Path>) -> Result<Vec<String>> {
    let contents = std::fs::read_to_string(file_path.as_ref())?;
    Ok(contents.lines().into_iter().map(str::to_string).collect())
}
