use std::fmt::Debug;

use crate::lumberjack::Lumberjack;
use crate::Result;

pub mod repl;

pub trait EventGroup: Debug + Sized {
    fn from_lumberjack(lumberjack: &Lumberjack) -> Result<Vec<Self>>;
}