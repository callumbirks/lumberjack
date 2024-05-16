use std::collections::BTreeSet;
use std::sync::Arc;

use regex::Regex;

use crate::data::repl::{Repl, ReplCollection, ReplConfig, ReplMode};
use crate::data::{LogEventType, LogLine, LogObjectType, ReplEvent};
use crate::error::{LumberjackError, Result};
use crate::match_contains;

use super::{LogObjectParse, LogParser};

impl LogObjectParse for Repl {
    const OBJECT_TYPE: LogObjectType = LogObjectType::Repl;
    const PATTERN: &'static str = r"(?i)\w*repl#\d+";

    fn parse_event(line: &str) -> Option<LogEventType> {
        match_contains!(line, {
            [ r#"{"Push":"# ]
                => LogEventType::Created,
            [ "Replicator status", "activityLevel=", "pushStatus=" ]
                => LogEventType::Repl(ReplEvent::StatusUpdate),
            [ "progress +" ]
                => LogEventType::Repl(ReplEvent::DocProgress),
            [ "Saving remote checkpoint", "Read local checkpoint", "Received remote checkpoint",
                "Saved remote checkpoint", "Saved local checkpoint", "No remote checkpoint",
                "No local checkpoint" ]
                => LogEventType::Repl(ReplEvent::Checkpoint),
            [ "now busy", "Connected!" ]
                => LogEventType::Repl(ReplEvent::Started),
            [ "Told to stop", "now stopped" ]
                => LogEventType::Destroyed,
            [ "Scanning for pre-existing conflicts", "conflicted docs in " ]
                => LogEventType::Repl(ReplEvent::ConflictScan),
            [ "Remote-DB ID", "Ignoring local checkpoint" ]
                => LogEventType::Repl(ReplEvent::Config),
            [ "Requesting remote checkpoint" ]
                => LogEventType::Repl(ReplEvent::RequestCheckpoint),
            [ "Replication complete!", "Connection closed" ]
                => LogEventType::Destroyed
        })
    }

    fn parse_details<'a>(
        _parser: &LogParser,
        lines: impl IntoIterator<Item = &'a Arc<LogLine>>,
    ) -> Result<Box<str>> {
        let mut lines = lines.into_iter();
        let Some(first_line) = lines.nth(0) else {
            return Err(LumberjackError::ParseError(
                "Repl is missing expected log lines!".to_string(),
            ));
        };
        let target = {
            let mut target = None;
            for line in lines.take(10) {
                if line.message.contains("Remote-DB ID") {
                    target = Some(Self::parse_target(line.message.as_ref())?);
                    break;
                }
            }
            target
        }
        .unwrap_or_else(|| "None".to_string());

        let config = Self::parse_config(&first_line.message)?;
        let c4id = 0; //Self::parse_c4id(&second_line.message)?;
        Ok(format!(
            "C4Repl: C4Replicator#{}\nConfig: {}\nTarget: {}",
            c4id, config, target
        )
        .into_boxed_str())
    }
}

impl Repl {
    fn parse_config(line: &str) -> Result<ReplConfig> {
        let re = Regex::new(
            r#"\{Coll#[0-9]+} "(?<coll>\w+)": \{"Push": (?<push>disabled|one-shot|continuous|passive), "Pull": (?<pull>disabled|one-shot|continuous|passive)"#,
        )?;

        // A slice of the line which we shrink after each match
        let mut mut_line = line;

        let mut collections: Vec<ReplCollection> = vec![];
        let mut i: usize = 0;

        while let Some(caps) = re.captures(mut_line) {
            let coll = caps.name("coll").map_or("", |m| m.as_str());
            let push_str = caps.name("push").map_or("", |m| m.as_str());
            let pull_str = caps.name("pull").map_or("", |m| m.as_str());
            if coll.is_empty() || push_str.is_empty() || pull_str.is_empty() {
                return Err(LumberjackError::ParseError(format!(
                    "Error parsing repl collection in match {}",
                    &caps[0]
                )));
            }

            let push = ReplMode::try_from(push_str)?;
            let pull = ReplMode::try_from(pull_str)?;

            collections.push(ReplCollection {
                name: coll.to_string(),
                index: i,
                push,
                pull,
            });
            mut_line = &mut_line[caps[0].len()..];
            i += 1;
        }

        if collections.is_empty() {
            return Err(LumberjackError::ParseError(format!(
                "Error parsing repl config from line {}",
                line
            )));
        }

        Ok(ReplConfig {
            collections,
            destination: "".to_string(),
        })
    }

    fn parse_target(line: &str) -> Result<String> {
        let re = Regex::new(r"Remote-DB ID \d found for target <(?P<target>\S+)>")?;
        let Some(caps) = re.captures(line) else {
            return Err(LumberjackError::ParseError(format!(
                "Failed to parse remote target from line {}",
                line
            )));
        };

        if let Some(target) = caps.name("target") {
            Ok(target.as_str().to_string())
        } else {
            Err(LumberjackError::ParseError(format!(
                "Failed to parse remote target from line {}",
                line
            )))
        }
    }

    fn parse_c4id(line: &str) -> Result<u64> {
        let re = Regex::new(r"\w*C4Replicator#(?P<id>\d+)")?;
        let Some(caps) = re.captures(line) else {
            return Err(LumberjackError::ParseError(format!(
                "Failed to parse C4Replicator ID from line {}",
                line
            )));
        };
        if let Some(id) = caps.name("id").and_then(|m| m.as_str().parse::<u64>().ok()) {
            Ok(id)
        } else {
            Err(LumberjackError::ParseError(format!(
                "Failed to parse C4Replicator ID from line {}",
                line
            )))
        }
    }
}
