use std::fmt::Debug;
use std::slice::Iter;
use std::sync::Arc;

use crate::lumberjack::{LogLine, Lumberjack};
use crate::Result;

pub mod repl;

pub trait LogObject: Debug + Send + Sync {
    fn name(&self) -> String;
    fn info(&self) -> String;
    fn lines(&self) -> Iter<'_, Arc<LogLine>>;
}

struct Test {
    item: Box<dyn LogObject>,
}

pub trait LogObjectGroup: Debug + Sized {
    async fn from_lumberjack(lumberjack: &Lumberjack) -> Result<Vec<Arc<Self>>>;
}
