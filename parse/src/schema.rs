// @generated automatically by Diesel CLI.

diesel::table! {
    event_types (id) {
        id -> Integer,
        name -> Text,
    }
}

diesel::table! {
    files (id) {
        id -> Integer,
        path -> Text,
        level -> Nullable<Integer>,
        timestamp -> Timestamp,
    }
}

diesel::table! {
    lines (file_id, line_num) {
        file_id -> Integer,
        line_num -> Integer,
        level -> Integer,
        timestamp -> Timestamp,
        domain -> Text,
        event_type -> Integer,
        event_data -> Nullable<Text>,
        object_path -> Nullable<Text>,
    }
}

diesel::joinable!(lines -> event_types (event_type));
diesel::joinable!(lines -> files (file_id));

diesel::allow_tables_to_appear_in_same_query!(
    event_types,
    files,
    lines,
);
