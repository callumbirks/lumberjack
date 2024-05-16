use std::collections::BTreeSet;
use std::sync::Arc;

use crate::data::{DBEvent, LogDomain, LogEventType, LogLine, LogObjectType};
use crate::error::Result;
use crate::match_contains;
use crate::parse::{LogObjectParse, LogParser};

pub struct DB;

impl LogObjectParse for DB {
    const OBJECT_TYPE: LogObjectType = LogObjectType::DB;
    const PATTERN: &'static str = r"\w*DB#\d+";

    fn parse_event(line: &str) -> Option<LogEventType> {
        match_contains!(line, {
            [ "Opening database" ]
                => LogEventType::Created,
            [ "Closed SQLite", "Closing database" ]
                => LogEventType::Destroyed,
            [ "Adding the `expiration`", "KeyStore(info) set" ]
                => LogEventType::DB(DBEvent::Opening),
            [ "begin transaction" ]
                => LogEventType::DB(DBEvent::TransactionBegin),
            [ "commit transaction" ]
                => LogEventType::DB(DBEvent::TransactionCommit),
            [ "Transaction exiting scope without explicit", "abort transaction" ]
                => LogEventType::DB(DBEvent::TransactionAbort),
            [ "Saved '" ]
                => LogEventType::DB(DBEvent::DocSaved),
            [ "Deleted '", "KeyStore(del" && ") insert" ]
                => LogEventType::DB(DBEvent::DocDeleted),
            [ "Next expiration time" ]
                => LogEventType::DB(DBEvent::ExpirationUpdate),
            [ "set expiration of" ]
                => LogEventType::DB(DBEvent::ExpirationSet),
            [ "KeyStore(checkpoints) set" ]
                => LogEventType::DB(DBEvent::CheckpointSet),
            [ "Housekeeping: " ]
                => LogEventType::DB(DBEvent::HousekeepingUpdate)
        })
    }

    fn parse_details<'a>(
        parser: &LogParser,
        lines: impl IntoIterator<Item = &'a Arc<LogLine>>,
    ) -> Result<Box<str>> {
        Ok("TODO".to_string().into_boxed_str())
    }
}
