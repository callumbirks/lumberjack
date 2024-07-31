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
