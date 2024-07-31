CREATE TABLE lines(
    file_id    INTEGER   NOT NULL,
    line_num   INTEGER   NOT NULL,
    -- Log Level (Info, Verbose, etc.)
    level      INTEGER   NOT NULL,
    timestamp  TIMESTAMP NOT NULL,
    -- Log Domain (DB, Sync, etc.)
    domain     INTEGER   NOT NULL,
    -- The Object which logged the log line
    object_id  INTEGER   NOT NULL,
    -- The EventType (an enum)
    event_type INTEGER   NOT NULL,
    -- Extra data for the event. The JSON schema is defined by the event_type.
    event_data TEXT,
    -- Composite primary key, level and line_num are always unique. In the case of rollover, the line_num in the
    -- next file starts after the last line_num in the previous file of that level.
    PRIMARY KEY (file_id, line_num),
    FOREIGN KEY (object_id)
        REFERENCES objects(id),
    FOREIGN KEY (file_id)
        REFERENCES files(id)
);

CREATE TABLE files(
    -- `id` is unrelated to CBL, it's just a sequential ID.
    id        INTEGER   PRIMARY KEY NOT NULL,
    path      TEXT      NOT NULL
);

CREATE TABLE objects(
    id          INTEGER PRIMARY KEY NOT NULL,
    -- Object Type (Repl, DB, Pusher, Query, etc.)
    object_type INTEGER NOT NULL,
    -- Extra data for the object. The JSON schema is defined by the object_type.
    data        TEXT
);
