pub struct Collection {
    pub name: String,
}

pub struct Scope {
    pub name: String,
    pub collections: Vec<Collection>,
}

#[derive(Debug, Copy, Clone)]
pub enum ReplMode {
    Disabled,
    Passive,
    OneShot,
    Continuous,
}

impl ReplMode {
    pub fn from_str(s: &str) -> Option<ReplMode> {
        match s {
            "disabled" => Some(ReplMode::Disabled),
            "passive" => Some(ReplMode::Passive),
            "one-shot" => Some(ReplMode::OneShot),
            "continuous" => Some(ReplMode::Continuous),
            _ => None
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReplCollection {
    pub name: String,
    pub index: usize,
    pub push: ReplMode,
    pub pull: ReplMode,
}

#[derive(Debug, Clone)]
pub struct ReplConfig {
    pub collections: Vec<ReplCollection>,
    pub destination: String,
}