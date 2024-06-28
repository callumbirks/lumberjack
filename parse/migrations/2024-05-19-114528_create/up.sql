CREATE TABLE lines(
    -- Log Level (Info, Verbose, etc.)
    level      INTEGER unsigned NOT NULL,
    line_num   BIGINT  unsigned NOT NULL,
    timestamp  TIMESTAMP        NOT NULL,
    -- Log line not including the boilerplate (timestamp, object ID) at the start
    message    TEXT             NOT NULL,
    -- EventType, an enum of every different event which can take place in the logs
    event_type INTEGER          NOT NULL,
    -- The Object which logged the log line
    object_id  INTEGER          NOT NULL,
    file_id    INTEGER          NOT NULL,
    -- Composite primary key, level and line_num are always unique. In the case of rollover, the line_num in the
    -- next file starts after the last line_num in the previous file of that level.
    PRIMARY KEY (file_id, line_num),
    FOREIGN KEY (object_id)
        REFERENCES objects(id),
    FOREIGN KEY (file_id)
        REFERENCES files(id)
);

CREATE TABLE files(
    id        INTEGER   PRIMARY KEY NOT NULL,
    path      TEXT      NOT NULL,
    -- Log Level
    level     INTEGER   NOT NULL,
    timestamp TIMESTAMP NOT NULL
);

CREATE TABLE objects(
    id    INTEGER PRIMARY KEY NOT NULL,
    -- Object Type (Repl, DB, Pusher, Query, etc.)
    ty    INTEGER NOT NULL
);

CREATE TABLE repls(
    object_id INTEGER PRIMARY KEY NOT NULL,
    -- Repl config, stored as JSON
    config    TEXT    NOT NULL,
    FOREIGN KEY (object_id)
        REFERENCES objects(id)
);