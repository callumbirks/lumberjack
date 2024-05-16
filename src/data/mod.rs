use enum_iterator::Sequence;
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock};

use chrono::NaiveDateTime;

use crate::enum_impl_display;
use crate::error::LumberjackError;
pub use file::LogFile;

mod file;
pub mod repl;

#[derive(Debug, Clone)]
pub struct LogLine {
    pub file: Arc<LogFile>,
    pub message: Box<str>,
    pub event: Option<LogEventType>,
    // Starts at 1
    pub line_num: u64,
    pub timestamp: NaiveDateTime,
    pub object: Option<Arc<LogObject>>,
}

/// # Example
/// ```
/// let repl = LogObject {
///     object_type: LogObjectType::Repl,
///     id: parsed_id, // i.e. 76, parsed from "Repl#76"
///     details: format!(
///         "ID: {} \nConfig: {}",
///         parsed_id, parsed_config
///     )
///     .into_boxed_str()
/// }
/// ```
#[derive(Debug, Clone)]
pub struct LogObject {
    pub object_type: LogObjectType,
    pub id: u64,
    pub details: OnceLock<Box<str>>,
}

enum_impl_display! {
    #[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Sequence)]
    pub enum LogDomain {
        None => "All",
        DB => "DB",
        Sync => "Sync",
        Query => "Query"
    }
}

enum_impl_display! {
    #[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Sequence)]
    pub enum LogLevel {
        None => "All",
        Info => "Info",
        Verbose => "Verbose",
        Debug => "Debug",
        Warn => "Warn",
        Error => "Error"
    }
}

enum_impl_display! {
    #[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Sequence)]
    pub enum LogEventType {
        None => "All",
        Created => "Created",
        Destroyed => "Destroyed";
        Repl(ReplEvent) => "Repl {}",
        QueryEnum(QueryEnumEvent) => "QueryEnum {}",
        DB(DBEvent) => "DB {}",
        Puller(PullerEvent) => "Puller {}",
        Pusher(PusherEvent) => "Pusher {}"
    }
}

enum_impl_display! {
    #[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Sequence)]
    pub enum DBEvent {
        Opening => "Opening",
        TransactionBegin => "Transaction Begin",
        TransactionCommit => "Transaction Commit",
        TransactionAbort => "Transaction Abort",
        DocSaved => "Doc Saved",
        ExpirationUpdate => "Expiration Update",
        ExpirationSet => "Expiration Set",
        DocDeleted => "Doc Deleted",
        CheckpointSet => "Checkpoint Set",
        HousekeepingUpdate => "Housekeeping"
    }
}

enum_impl_display! {
    #[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Sequence)]
    pub enum ReplEvent {
        Started => "Started",
        Replicated => "Replicated",
        StatusUpdate => "Status Update",
        Checkpoint => "Checkpoint Update",
        DocProgress => "Doc Progress",
        ConflictScan => "Conflict Scan",
        Config => "Config",
        RequestCheckpoint => "Request Checkpoint"
    }
}

enum_impl_display! {
    #[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Sequence)]
    pub enum PullerEvent {
        Started => "Started",
        HandledRevs => "Handled Revs",
        Checkpoint => "Checkpoint Update",
        Progress => "Progress",
        ActivityUpdate => "Activity Update",
        WaitingRevs => "Waiting Revs",
        BackPressure => "Back Pressure"
    }
}

enum_impl_display! {
    #[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Sequence)]
    pub enum PusherEvent {
        Started => "Started",
        FoundChanges => "Found Changes",
        CaughtUp => "Caught Up",
        Progress => "Progress",
        ActivityUpdate => "Activity Update",
        ChangesResponse => "Changes Response",
        QueueRev => "Queueing Rev",
        SendRev => "Sending Rev",
        CheckpointUpdate => "Checkpoint Update",
        CompletedRev => "Completed Rev"
    }
}

enum_impl_display! {
    #[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Sequence)]
    pub enum QueryEnumEvent {
        ResultEnumerated => "Result Enumerated"
    }
}

impl TryFrom<&str> for LogLevel {
    type Error = LumberjackError;
    fn try_from(value: &str) -> Result<Self, <Self as TryFrom<&str>>::Error> {
        match value {
            "info" => Ok(LogLevel::Info),
            "verbose" => Ok(LogLevel::Verbose),
            "debug" => Ok(LogLevel::Debug),
            "warning" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(LumberjackError::ParseError(format!(
                "Unknown log level {}",
                value
            ))),
        }
    }
}

impl LogLevel {
    pub fn from_str(str: &str) -> Option<LogLevel> {
        match str {
            "info" => Some(LogLevel::Info),
            "verbose" => Some(LogLevel::Verbose),
            "debug" => Some(LogLevel::Debug),
            "warning" => Some(LogLevel::Warn),
            "error" => Some(LogLevel::Error),
            _ => None,
        }
    }
}

enum_impl_display! {
    #[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Sequence)]
    pub enum LogObjectType {
        None => "None",
        DB => "DB",
        Repl => "Repl",
        Puller => "Puller",
        Pusher => "Pusher",
        Query => "Query",
        QueryEnum => "QueryEnum"
    }
}

impl Display for LogObject {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}#{}", self.object_type, self.id)
    }
}

impl LogObject {
    pub fn name(&self) -> String {
        format!("{}#{}", self.object_type, self.id)
    }

    pub fn details(&self) -> String {
        let details = self.details.get().map(AsRef::as_ref).unwrap_or("");
        format!("{}#{}\n{}", self.object_type, self.id, details)
    }
}

impl PartialEq<Self> for LogObject {
    fn eq(&self, other: &Self) -> bool {
        // Object IDs are all unique
        self.id.eq(&other.id)
    }
}

impl Eq for LogObject {}

impl Hash for LogObject {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Object IDs are all unique
        self.id.hash(state)
    }
}

impl PartialEq<Self> for LogLine {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp.eq(&other.timestamp) && self.file.eq(&other.file)
    }
}

impl Eq for LogLine {}

impl PartialOrd<Self> for LogLine {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LogLine {
    fn cmp(&self, other: &Self) -> Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}

impl Hash for LogLine {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.timestamp.hash(state);
    }
}
