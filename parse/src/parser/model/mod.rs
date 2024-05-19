pub use dir::DirParserModel;

use crate::Result;

mod dir;

pub trait Model {
    fn from_version_string(line: String) -> Result<Box<Self>>;
}
