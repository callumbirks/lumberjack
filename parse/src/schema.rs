// @generated automatically by Diesel CLI.

diesel::table! {
    files (id) {
        id -> Integer,
        path -> Text,
    }
}

diesel::table! {
    lines (level, line_num) {
        level -> Integer,
        line_num -> Integer,
        timestamp -> Integer,
        message -> Text,
        event_type -> Integer,
        object_id -> Integer,
        file_id -> Integer,
    }
}

diesel::table! {
    objects (id) {
        id -> Integer,
        type_ -> Integer,
    }
}

diesel::table! {
    replicators (id) {
        id -> Integer,
        config -> Binary,
        pusher_id -> Nullable<Integer>,
        puller_id -> Nullable<Integer>,
    }
}

diesel::joinable!(lines -> files (file_id));
diesel::joinable!(lines -> objects (object_id));

diesel::allow_tables_to_appear_in_same_query!(
    files,
    lines,
    objects,
    replicators,
);
