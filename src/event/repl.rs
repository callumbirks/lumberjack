use std::collections::HashMap;
use std::io::BufRead;

use grep::matcher::Matcher;
use regex::Regex;

use crate::data::{ReplCollection, ReplConfig, ReplMode};
use crate::event::EventGroup;
use crate::lumberjack::{LogLine, Lumberjack};
use crate::LumberjackError::ParseError;
use crate::Result;

#[derive(Debug, Clone)]
pub struct Repl {
    pub id: u64,
    pub c4id: u64,
    pub config: ReplConfig,
    pub lines: Vec<LogLine>,
}

impl EventGroup for Repl {
    fn from_lumberjack(lumberjack: &Lumberjack) -> Result<Vec<Self>> {
        let pattern = r"(?i)\w*repl#\d+";

        let mut replicators: HashMap<i64, Repl> = HashMap::new();

        let matches = lumberjack.find(pattern)?;

        for lmatch in matches {
            let Some(repl_id) = lmatch.snippet.split('#').last()
                .and_then(|n| n.parse::<i64>().ok())
                else {
                    return Err(ParseError(
                        format!("Couldn't parse Repl ID in snippet {:?}", lmatch.snippet)));
                };

            if let Some(entry) = replicators.get_mut(&repl_id) {
                entry.lines.push(lmatch.log_line)
            } else {
                let repl = Repl {
                    id: repl_id as u64,
                    c4id: 0,
                    config: ReplConfig { collections: vec![], destination: "".to_string() },
                    lines: vec![lmatch.log_line],
                };
                replicators.insert(repl_id, repl);
            }
        }

        if replicators.is_empty() {
            return Err(crate::LumberjackError::NoMatches(pattern.to_string()));
        }

        let mut replicators: Vec<Repl> = replicators.values().map(Clone::clone).collect();

        for repl in &mut replicators {
            let log_line = repl.lines.first().unwrap();
            let Some(second_line) = repl.lines.get(1) else {
                return Err(ParseError(format!("Repl#{} is missing expected log lines!", repl.id)));
            };

            let line_str = log_line.read()?;
            let second_line_str = second_line.read()?;
            repl.config = Self::parse_config(&line_str)?;
            repl.c4id = Self::parse_c4id(&second_line_str)?;
        }

        Ok(replicators)
    }
}

impl Repl {
    fn parse_config(line: &str) -> Result<ReplConfig> {
        let re = Regex::new(r#"\{Coll#[0-9]+} "(?<coll>\w+)": \{"Push": (?<push>disabled|one-shot|continuous|passive), "Pull": (?<pull>disabled|one-shot|continuous|passive)"#)?;

        // A slice of the line which we shrink after each match
        let mut mut_line = line;

        let mut collections: Vec<ReplCollection> = vec![];
        let mut i: usize = 0;

        while let Some(caps) = re.captures(mut_line) {
            let coll = caps.name("coll").map_or("", |m| m.as_str());
            let push_str = caps.name("push").map_or("", |m| m.as_str());
            let pull_str = caps.name("pull").map_or("", |m| m.as_str());
            if coll.is_empty() || push_str.is_empty() || pull_str.is_empty() {
                return Err(
                    ParseError(format!("Error parsing repl collection in match {}", &caps[0]))
                );
            }

            let Some(push) = ReplMode::from_str(push_str) else {
                return Err(
                    ParseError(format!("Unkown repl mode {}", push_str))
                );
            };

            let Some(pull) = ReplMode::from_str(pull_str) else {
                return Err(
                    ParseError(format!("Unkown repl mode {}", pull_str))
                );
            };

            collections.push(ReplCollection { name: coll.to_string(), index: i, push, pull });
            mut_line = &mut_line[caps[0].len()..];
            i += 1;
        }

        if collections.is_empty() {
            return Err(
                ParseError(format!("Error parsing repl config from line {}", line))
            );
        }

        Ok(ReplConfig {
            collections,
            destination: "".to_string(),
        })
    }

    fn parse_c4id(line: &str) -> Result<u64> {
        let re = Regex::new(r"\w*C4Replicator#(?P<id>\d+)")?;
        let Some(caps) = re.captures(line) else {
            return Err(ParseError(format!("Failed to parse C4Replicator ID from line {}", line)));
        };
        if let Some(id) = caps.name("id").and_then(|m| m.as_str().parse::<u64>().ok()) {
            Ok(id)
        } else {
            Err(ParseError(format!("Failed to parse C4Replicator ID from line {}", line)))
        }
    }
}
