CREATE TABLE lines(
    file_id    INTEGER   NOT NULL,
    line_num   INTEGER   NOT NULL,
    -- Log Level (Info, Verbose, etc.)
    level      INTEGER   NOT NULL,
    timestamp  TIMESTAMP NOT NULL,
    -- Log Domain (DB, Sync, etc.)
    domain     TEXT      NOT NULL,
    -- The EventType (an enum)
    event_type INTEGER   NOT NULL,
    -- Extra data for the event. The JSON schema is defined by the event_type.
    event_data TEXT,
    -- The object path, if any, i.e. /Repl#76/Pusher#123/ for Pusher#123 which belongs to Repl#76
    object_path TEXT             ,
    -- Composite primary key, level and line_num are always unique. In the case of rollover, the line_num in the
    -- next file starts after the last line_num in the previous file of that level.
    PRIMARY KEY (file_id, line_num),
    FOREIGN KEY (file_id)
        REFERENCES files(id),
    FOREIGN KEY (event_type)
        REFERENCES event_types(id)
);

CREATE TABLE files(
    -- `id` is unrelated to CBL, it's just a sequential ID.
    id        INTEGER   PRIMARY KEY NOT NULL,
    path      TEXT      NOT NULL,
    -- Log level
    level     INTEGER,
    timestamp TIMESTAMP NOT NULL
);

-- A store of the event type names to make querying easier.
-- event_type is stored as an integer in the lines table, and the corresponding name is stored here.
CREATE TABLE event_types(
    id         INTEGER PRIMARY KEY NOT NULL,
    name       TEXT    NOT NULL
);
