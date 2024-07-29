use std::str::FromStr;

use serde::{Deserialize, Serialize};
use util::as_event;

use crate::parser::regex_patterns::Patterns;
use crate::Result;

use super::{Event, EventType};

pub trait AsEvent: Sized {
    const EVENT_TYPE: EventType;
    fn json(&self) -> Option<String>;
    fn from_line(line: &str, regex_cache: &Patterns) -> Result<Self>;
}

impl<T> From<T> for Event
where
    T: AsEvent,
{
    fn from(event: T) -> Self {
        Event {
            event_type: T::EVENT_TYPE,
            data: event.json(),
        }
    }
}

as_event!(
    pub struct DBOpenEvent {
        REGEX_YAML = db_open,
        EVENT_TYPE = EventType::DBOpening,
    }
);

as_event!(
    pub struct DBUpgradeEvent {
        REGEX_YAML = db_upgrade,
        EVENT_TYPE = EventType::DBUpgrade,
        old_ver: i32,
        new_ver: i32,
    }
);

as_event!(
    pub struct DBTxBeginEvent {
        REGEX_YAML = db_tx_begin,
        EVENT_TYPE = EventType::DBTxBegin,
    }
);

as_event!(
    pub struct DBTxCommitEvent {
        REGEX_YAML = db_tx_commit,
        EVENT_TYPE = EventType::DBTxCommit,
    }
);

as_event!(
    pub struct DBTxAbortEvent {
        REGEX_YAML = db_tx_abort,
        EVENT_TYPE = EventType::DBTxAbort,
    }
);

as_event!(
    pub struct DBSavedRevEvent {
        REGEX_YAML = db_saved_rev,
        EVENT_TYPE = EventType::DBSavedRev,
        doc_id: String,
        rev_id: String,
        sequence: i64,
    }
);

as_event!(
    pub struct SubreplStartEvent {
        REGEX_YAML = subrepl_start,
        EVENT_TYPE = EventType::SubreplStart,
        mode: String,
        sequence: i64,
    }
);

as_event!(
    pub struct PullerHandledRevsEvent {
        REGEX_YAML = puller_handled_revs,
        EVENT_TYPE = EventType::PullerHandledRevs,
        handled: i32,
        pending: i32,
    }
);

as_event!(
    pub struct BLIPSendRequestStartEvent {
        REGEX_YAML = blip_send_request_start,
        EVENT_TYPE = EventType::BLIPSendRequestStart,
        request: String,
        id: i32,
    }
);

as_event!(
    pub struct BLIPQueueRequestEvent {
        REGEX_YAML = blip_queue_request,
        EVENT_TYPE = EventType::BLIPQueueRequest,
        id: i32,
    }
);

as_event!(
    pub struct BLIPWSWriteStartEvent {
        REGEX_YAML = blip_ws_write_start,
        EVENT_TYPE = EventType::BLIPWSWriteStart,
    }
);

as_event!(
    pub struct BLIPSendFrameEvent {
        REGEX_YAML = blip_send_frame,
        EVENT_TYPE = EventType::BLIPSendFrame,
        id: i32,
        message_type: String,
        more_coming: char,
        urgent: char,
        no_reply: char,
        compressed: char,
        from_byte: i32,
        to_byte: i32,
    }
);

as_event!(
    pub struct BLIPSendRequestEndEvent {
        REGEX_YAML = blip_send_request_end,
        EVENT_TYPE = EventType::BLIPSendRequestEnd,
        id: i32,
    }
);

as_event!(
    pub struct BLIPWSWriteEndEvent {
        REGEX_YAML = blip_ws_write_end,
        EVENT_TYPE = EventType::BLIPWSWriteEnd,
        bytes: i32,
        writeable: bool,
    }
);

as_event!(
    pub struct BLIPReceiveFrameEvent {
        REGEX_YAML = blip_receive_frame,
        EVENT_TYPE = EventType::BLIPReceiveFrame,
        message_type: String,
        id: i32,
        more_coming: char,
        urgent: char,
        no_reply: char,
        compressed: char,
        length: i32,
    }
);

as_event!(
    pub struct HousekeeperMonitorEvent {
        REGEX_YAML = housekeeper_monitor,
        EVENT_TYPE = EventType::HousekeeperMonitoring,
    }
);

as_event!(
    pub struct ReplConflictScanEvent {
        REGEX_YAML = repl_conflict_scan,
        EVENT_TYPE = EventType::ReplConflictScan,
        num_conflicts: i32,
    }
);

as_event!(
    pub struct ReplConnectedEvent {
        REGEX_YAML = repl_connected,
        EVENT_TYPE = EventType::ReplConnected,
    }
);

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
#[repr(i32)]
pub enum BLIPConnectionState {
    Disconnected = -1,
    Closed = 0,
    Connecting,
    Connected,
    Closing,
}

impl FromStr for BLIPConnectionState {
    type Err = crate::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let discriminant: i32 = s.parse()?;
        match discriminant {
            -1 => Ok(BLIPConnectionState::Disconnected),
            0 => Ok(BLIPConnectionState::Closed),
            1 => Ok(BLIPConnectionState::Connecting),
            2 => Ok(BLIPConnectionState::Connected),
            3 => Ok(BLIPConnectionState::Closing),
            other => Err(crate::Error::InvalidEnumValue(other, "BLIPConnectionState")),
        }
    }
}

as_event!(
    pub struct ReplActivityUpdateEvent {
        REGEX_YAML = repl_activity_update,
        EVENT_TYPE = EventType::ReplActivityUpdate,
        activity: String,
        connection_state: BLIPConnectionState,
        saving_checkpoint: i32,
    }
);

as_event!(
    pub struct ReplStatusUpdateEvent {
        REGEX_YAML = repl_status_update,
        EVENT_TYPE = EventType::ReplStatusUpdate,
        push_status: String,
        pull_status: String,
        completed: i32,
        total: i32,
        doc_count: i32,
    }
);

mod util {
    macro_rules! as_event {
        (pub struct $type_name:ident {
            REGEX_YAML = $regex:tt,
            EVENT_TYPE = $event_type:expr,
            $($field:ident: $field_ty:ty),+$(,)?
        }) => {
            #[deny(unused)]
            #[derive(serde::Serialize)]
            pub struct $type_name {
                $($field: $field_ty),+
            }

            impl AsEvent for $type_name {
                const EVENT_TYPE: EventType = $event_type;

                fn json(&self) -> Option<String> {
                    serde_json::to_string(self).ok()
                }

                fn from_line(line: &str, regex_cache: &Patterns) -> Result<Self> {
                    let Some(captures) = regex_cache.$regex.captures(line) else {
                        return Err(crate::Error::NoMatches);
                    };

                    #[allow(unused_parens)]
                    let ($($field),+) = ($({
                        captures
                            .name(stringify!($field))
                            .and_then(|m| m.as_str().parse::<$field_ty>().ok())
                            .ok_or_else(|| crate::Error::NoMatches)?
                    }),+);

                    Ok(Self { $($field),+ })
                }
            }
        };
        (pub struct $type_name:ident {
            REGEX_YAML = $regex:tt,
            EVENT_TYPE = $event_type:expr,
        }) => {
            pub struct $type_name;

            impl AsEvent for $type_name {
                const EVENT_TYPE: EventType = $event_type;

                fn json(&self) -> Option<String> {
                    None
                }

                fn from_line(line: &str, regex_cache: &Patterns) -> Result<Self> {
                    if !regex_cache.$regex.is_match(line) {
                        return Err(crate::Error::NoMatches);
                    }
                    Ok(Self)
                }
            }
        };
    }

    pub(super) use as_event;
}
