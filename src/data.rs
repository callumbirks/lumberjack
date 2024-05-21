use enum_iterator::Sequence;
use lumberjack_parse::data;
use lumberjack_parse::data::{Object, ObjectExtra};

#[derive(Debug, Clone)]
pub struct UIObject {
    pub object: Object,
    pub extra: ObjectExtra,
}

macro_rules! ui_enum {
    (from $other_enum:ty, pub enum $enum_name:ident {
        $(none $none_variant:ident => $other_none:expr,)?$(,)*
        $($variant:ident => $other:expr),*$(,)*
        $($nested:ident($nested_inner:ty) => $other_nested:expr),*$(,)*
    }) => {
        #[derive(Debug, Copy, Clone, PartialEq, Sequence)]
        pub enum $enum_name {
            $($none_variant,)?
            $($variant),*,
            $($nested($nested_inner)),*
        }

        impl From<$enum_name> for $other_enum {
            fn from(value: $enum_name) -> Self {
                match value {
                    $($enum_name::$none_variant => $other_none,)?
                    $($enum_name::$variant => $other),*
                    $($enum_name::$nested(x) => $other_nested(x)),*
                }
            }
        }

        impl From<$other_enum> for $enum_name {
            fn from(value: $other_enum) -> Self {
                match value {
                    $($other => $enum_name::$variant),*
                    $($other_nested(x) => $enum_name::$nested(x)),*
                }
            }
        }
    };
}

ui_enum!(from data::Level,
    pub enum UILevel {
        none None => data::Level::Info,
        Info => data::Level::Info,
        Verbose => data::Level::Verbose,
        Debug => data::Level::Debug,
        Warn => data::Level::Warn,
        Error => data::Level::Error
    }
);

ui_enum!(from data::ObjectType,
    pub enum UIObjectType {
        none None => data::ObjectType::DB,
        DB => data::ObjectType::DB,
        Repl => data::ObjectType::Repl,
        Pusher => data::ObjectType::Pusher,
        Puller => data::ObjectType::Puller
    }
);

ui_enum!(from data::EventType,
    pub enum UIEventType {
        None => data::EventType::None,
        Common(data::CommonEvent) => data::EventType::Common,
        DB(data::DBEvent) => data::EventType::DB
    }
);

ui_enum!(from data::CommonEvent,
    pub enum UICommonEvent {
        Created => data::CommonEvent::Created,
        Destroyed => data::CommonEvent::Destroyed,
    }
);

ui_enum!(from data::DBEvent,
    pub enum UIDBEvent {
        Opening => data::DBEvent::Opening,
        TxBegin => data::DBEvent::TxBegin,
        TxCommit => data::DBEvent::TxCommit,
        TxEnd => data::DBEvent::TxEnd,
        TxAbort => data::DBEvent::TxAbort,
    }
);
