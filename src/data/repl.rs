use std::collections::HashMap;
use std::fmt::{Display, Formatter, Write};
use std::hash::{Hash, Hasher};
use std::slice::Iter;
use std::sync::Arc;

use regex::Regex;

use crate::data::{LogObject, LogObjectGroup};
use crate::lumberjack::{LogLine, Lumberjack};
use crate::widget::log_table;
use crate::widget::log_table::Row;
use crate::LumberjackError::ParseError;
use crate::{LumberjackError, Result};

#[derive(Debug, Clone)]
pub struct Repl {
    pub id: u64,
    pub c4id: u64,
    pub config: ReplConfig,
    pub lines: Vec<Arc<LogLine>>,
}

pub struct Collection {
    pub name: String,
}

pub struct Scope {
    pub name: String,
    pub collections: Vec<Collection>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ReplMode {
    Disabled,
    Passive,
    OneShot,
    Continuous,
}

impl TryFrom<&str> for ReplMode {
    type Error = LumberjackError;
    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value {
            "disabled" => Ok(ReplMode::Disabled),
            "passive" => Ok(ReplMode::Passive),
            "one-shot" => Ok(ReplMode::OneShot),
            "continuous" => Ok(ReplMode::Continuous),
            _ => Err(ParseError(format!("Unknown repl mode {}", value))),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ReplCollection {
    pub name: String,
    pub index: usize,
    pub push: ReplMode,
    pub pull: ReplMode,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ReplConfig {
    pub collections: Vec<ReplCollection>,
    pub destination: String,
}

impl LogObjectGroup for Repl {
    async fn from_lumberjack(lumberjack: &Lumberjack) -> Result<Vec<Arc<Self>>> {
        let pattern = r"(?i)\w*repl#\d+";

        let matches = lumberjack.find(pattern).await?;

        let replicators = tokio::task::spawn_blocking(move || {
            let mut replicators: HashMap<i64, Repl> = HashMap::new();

            for lmatch in matches {
                let Some(repl_id) = lmatch
                    .snippet
                    .split('#')
                    .last()
                    .and_then(|n| n.parse::<i64>().ok())
                else {
                    return Err(ParseError(format!(
                        "Couldn't parse Repl ID in snippet {:?}",
                        lmatch.snippet
                    )));
                };

                if let Some(entry) = replicators.get_mut(&repl_id) {
                    entry.lines.push(Arc::from(lmatch.log_line))
                } else {
                    let repl = Repl {
                        id: repl_id as u64,
                        c4id: 0,
                        config: ReplConfig {
                            collections: vec![],
                            destination: "".to_string(),
                        },
                        lines: vec![Arc::from(lmatch.log_line)],
                    };
                    replicators.insert(repl_id, repl);
                }
            }

            if replicators.is_empty() {
                return Err(LumberjackError::NoMatches(pattern.to_string()));
            }

            replicators
                .into_values()
                .map(|mut r| {
                    let log_line = r.lines.first().unwrap();
                    let Some(second_line) = r.lines.get(1) else {
                        return Err(ParseError(format!(
                            "Repl#{} is missing expected log lines!",
                            r.id
                        )));
                    };
                    let line_str = log_line.read();
                    let second_line_str = second_line.read();
                    r.config = Self::parse_config(line_str)?;
                    r.c4id = Self::parse_c4id(second_line_str)?;
                    Ok(Arc::from(r))
                })
                .collect()
        });

        replicators.await.map_err(|_| LumberjackError::TokioJoin)?
    }
}

impl LogObject for Repl {
    fn name(&self) -> String {
        format!("Repl#{}", self.id)
    }

    fn info(&self) -> String {
        format!("Config: {}", self.config)
    }

    fn lines(&self) -> Iter<'_, Arc<LogLine>> {
        self.lines.iter()
    }
}

impl PartialEq<Self> for Repl {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Repl {}

impl Hash for Repl {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.id);
    }
}

impl Display for Repl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name())
    }
}

impl Display for ReplMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ReplMode::Disabled => "Disabled",
            ReplMode::Passive => "Passive",
            ReplMode::OneShot => "OneShot",
            ReplMode::Continuous => "Continuous",
        })
    }
}

impl Display for ReplCollection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Name: {}, Index: {}, Push: {}, Pull: {}",
            self.name, self.index, self.push, self.pull
        ))
    }
}

impl Display for ReplConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Collections: [")?;
        for (i, coll) in self.collections.iter().enumerate() {
            coll.fmt(f)?;
            if i < self.collections.len() - 1 {
                f.write_char(',')?;
            }
        }
        f.write_char(']')
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
                return Err(ParseError(format!(
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
            return Err(ParseError(format!(
                "Error parsing repl config from line {}",
                line
            )));
        }

        Ok(ReplConfig {
            collections,
            destination: "".to_string(),
        })
    }

    fn parse_c4id(line: &str) -> Result<u64> {
        let re = Regex::new(r"\w*C4Replicator#(?P<id>\d+)")?;
        let Some(caps) = re.captures(line) else {
            return Err(ParseError(format!(
                "Failed to parse C4Replicator ID from line {}",
                line
            )));
        };
        if let Some(id) = caps.name("id").and_then(|m| m.as_str().parse::<u64>().ok()) {
            Ok(id)
        } else {
            Err(ParseError(format!(
                "Failed to parse C4Replicator ID from line {}",
                line
            )))
        }
    }
}
