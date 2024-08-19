use diesel::internal::derives::multiconnection::chrono::NaiveDateTime;
use serde::{Serialize, Serializer};

// Some helper types / functions for writing Lumberjack types to XLSX.
// Specifically, write enums as strings, because `rust_xlsxwriter` will not write enums.

fn serialize_to_string<S, T>(t: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: ToString,
{
    serializer.serialize_str(&t.to_string())
}

#[derive(Serialize)]
pub struct File {
    id: i32,
    path: String,
}

#[derive(Serialize)]
pub struct Line {
    file_id: i32,
    line_num: i32,
    #[serde(serialize_with = "serialize_to_string")]
    level: lumberjack_parse::data::Level,
    timestamp: NaiveDateTime,
    domain: String,
    #[serde(serialize_with = "serialize_to_string")]
    event_type: lumberjack_parse::data::EventType,
    event_data: Option<String>,
    object_path: Option<String>,
}

impl From<lumberjack_parse::data::File> for File {
    fn from(file: lumberjack_parse::data::File) -> Self {
        File {
            id: file.id,
            path: file.path,
        }
    }
}

impl From<lumberjack_parse::data::Line> for Line {
    fn from(value: lumberjack_parse::data::Line) -> Self {
        Line {
            file_id: value.file_id,
            line_num: value.line_num,
            level: value.level,
            timestamp: value.timestamp,
            domain: value.domain,
            event_type: value.event_type,
            event_data: value.event_data,
            object_path: value.object_path,
        }
    }
}
