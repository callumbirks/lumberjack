// @generated automatically by Diesel CLI.

diesel::table! {
    files (id) {
        id -> Integer,
        path -> Text,
    }
}

diesel::table! {
    lines (file_id, line_num) {
        file_id -> Integer,
        line_num -> Integer,
        level -> Integer,
        timestamp -> Timestamp,
        domain -> Integer,
        object_id -> Integer,
        event_type -> Integer,
        event_data -> Nullable<Text>,
    }
}

diesel::table! {
    objects (id) {
        id -> Integer,
        object_type -> Integer,
        data -> Nullable<Text>,
    }
}

diesel::joinable!(lines -> files (file_id));
diesel::joinable!(lines -> objects (object_id));

diesel::allow_tables_to_appear_in_same_query!(
    files,
    lines,
    objects,
);
