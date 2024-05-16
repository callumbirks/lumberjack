use crate::data::{LogEventType, LogLine, LogObjectType, PusherEvent};
use crate::match_contains;
use crate::parse::{LogObjectParse, LogParser};
use std::sync::Arc;

pub struct Pusher;

impl LogObjectParse for Pusher {
    const OBJECT_TYPE: LogObjectType = LogObjectType::Pusher;
    const PATTERN: &'static str = r"\w*Pusher#\d+";

    fn parse_event(line: &str) -> Option<LogEventType> {
        match_contains!(line, {
            [ "Starting push from local seq" ]
                => LogEventType::Created,
            [ "Found " && " changes up to", "Read " && "local changes" ]
                => LogEventType::Pusher(PusherEvent::FoundChanges),
            [ "Got response for " ]
                => LogEventType::Pusher(PusherEvent::ChangesResponse),
            [ "Caught up" ]
                => LogEventType::Pusher(PusherEvent::CaughtUp),
            [ "activityLevel=" ]
                => LogEventType::Pusher(PusherEvent::ActivityUpdate),
            [ "now busy" ]
                => LogEventType::Pusher(PusherEvent::Started),
            [ ") progress +" ]
                => LogEventType::Pusher(PusherEvent::Progress),
            [ "now stopped" ]
                => LogEventType::Destroyed,
            [ "Sending rev '", "Transmitting 'rev'" ]
                => LogEventType::Pusher(PusherEvent::SendRev),
            [ "Queueing rev '" ]
                => LogEventType::Pusher(PusherEvent::QueueRev),
            [ "Completed rev" ]
                => LogEventType::Pusher(PusherEvent::CompletedRev),
            [ "Checkpoint now" ]
                => LogEventType::Pusher(PusherEvent::CheckpointUpdate),
        })
    }

    fn parse_details<'a>(
        parser: &LogParser,
        lines: impl IntoIterator<Item = &'a Arc<LogLine>>,
    ) -> crate::error::Result<Box<str>> {
        Ok(Box::default())
    }
}
