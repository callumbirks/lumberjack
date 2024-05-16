use crate::enum_impl_display;
use std::fmt::{Display, Formatter, Write};

use crate::error::LumberjackError;

pub struct Repl;

pub struct Collection {
    pub name: String,
}

pub struct Scope {
    pub name: String,
    pub collections: Vec<Collection>,
}

enum_impl_display! {
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub enum ReplMode {
        Disabled => "Disabled",
        Passive => "Passive",
        OneShot => "One-Shot",
        Continuous => "Continuous"
    }
}

impl TryFrom<&str> for ReplMode {
    type Error = LumberjackError;
    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value {
            "disabled" => Ok(ReplMode::Disabled),
            "passive" => Ok(ReplMode::Passive),
            "one-shot" => Ok(ReplMode::OneShot),
            "continuous" => Ok(ReplMode::Continuous),
            _ => Err(LumberjackError::ParseError(format!(
                "Unknown repl mode {}",
                value
            ))),
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

impl Display for ReplCollection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Name: {},\nIndex: {}\nPush: {}\nPull: {}",
            self.name, self.index, self.push, self.pull
        )
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
