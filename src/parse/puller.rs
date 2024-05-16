use crate::data::repl::Repl;
use crate::data::{LogDomain, LogEventType, LogLine, LogObjectType, PullerEvent};
use crate::error::LumberjackError;
use crate::match_contains;
use crate::parse::{LogObjectParse, LogParser};
use crate::util::ContainsWithCase;
use std::collections::BTreeSet;
use std::sync::Arc;

pub struct Puller;

impl LogObjectParse for Puller {
    const OBJECT_TYPE: LogObjectType = LogObjectType::Puller;
    const PATTERN: &'static str = r"\w*Puller#\d+";

    fn parse_event(line: &str) -> Option<LogEventType> {
        match_contains!(line, {
            [ "Checkpoint now at '" ]
                => LogEventType::Puller(PullerEvent::Checkpoint),
            [ ") progress +" ]
                => LogEventType::Puller(PullerEvent::Progress),
            [ "Now waiting for " ]
                => LogEventType::Puller(PullerEvent::WaitingRevs),
            [ "revs were provisionally handled" ]
                => LogEventType::Puller(PullerEvent::HandledRevs),
            [ "Starting pull from remote seq" ]
                => LogEventType::Created,
            [ "activityLevel=" ]
                => LogEventType::Puller(PullerEvent::ActivityUpdate),
            [ r#"msg["revocations"]="# ]
                => LogEventType::Created,
            [ "now busy" ]
                => LogEventType::Puller(PullerEvent::Started),
            [ "Back pressure" ]
                => LogEventType::Puller(PullerEvent::BackPressure),
            [ "now stopped" ]
                => LogEventType::Destroyed,
        })
    }

    fn parse_details<'a>(
        parser: &LogParser,
        lines: impl IntoIterator<Item = &'a Arc<LogLine>>,
    ) -> crate::error::Result<Box<str>> {
        let mut lines = lines.into_iter();
        let first = lines.nth(0).expect("parse_details called with empty lines");

        let repl_id: Option<u64> = {
            // Search for the nearest "Repl now busy" to the first line. This is probably the repl we
            // belong to.
            let search_range: std::ops::Range<usize> =
                ((first.line_num as usize).saturating_sub(20))..(first.line_num as usize);
            parser
                .get_lines(&first.file, search_range)
                .and_then(|lines| {
                    lines
                        .iter()
                        .rev()
                        .find(|s| s.contains_with_case("{repl#") && s.contains("now busy"))
                        .and_then(|s| LogParser::parse_id::<Repl>(s))
                })
        };

        let parent_str = if let Some(repl_id) = repl_id {
            format!("Repl#{}", repl_id)
        } else {
            "None found".to_string()
        };

        Ok(format!("Parent: {}", parent_str).into_boxed_str())
    }
}
