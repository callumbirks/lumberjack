// @generated automatically by Diesel CLI.

diesel::table! {
    files (id) {
        id -> Integer,
        path -> Text,
        level -> Integer,
        timestamp -> Timestamp,
    }
}

diesel::table! {
    lines (line_num, file_id) {
        level -> Integer,
        line_num -> BigInt,
        timestamp -> Timestamp,
        message -> Text,
        event_type -> Integer,
        object_id -> Integer,
        file_id -> Integer,
    }
}

diesel::table! {
    objects (id) {
        id -> Integer,
        ty -> Integer,
    }
}

diesel::table! {
    repls (object_id) {
        object_id -> Integer,
        config -> Text,
    }
}

diesel::joinable!(lines -> files (file_id));
diesel::joinable!(lines -> objects (object_id));
diesel::joinable!(repls -> objects (object_id));

diesel::allow_tables_to_appear_in_same_query!(
    files,
    lines,
    objects,
    repls,
);
