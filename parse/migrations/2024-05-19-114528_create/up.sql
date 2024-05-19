-- Your SQL goes here
CREATE TABLE lines(
    level      INTEGER unsigned NOT NULL,
    line_num   INTEGER unsigned NOT NULL,
    timestamp  INTEGER          NOT NULL,
    message    TEXT             NOT NULL,
    event_type INTEGER          NOT NULL,    
    object_id  INTEGER          NOT NULL,
    file_id    INTEGER          NOT NULL,
    PRIMARY KEY (level, line_num),
    FOREIGN KEY (object_id)
        REFERENCES objects(id),
    FOREIGN KEY (file_id)
        REFERENCES files(id)
);

CREATE TABLE files(
    id   INTEGER PRIMARY KEY NOT NULL,
    path TEXT    NOT NULL
);

CREATE TABLE objects(
    id    INTEGER PRIMARY KEY NOT NULL,
    type_ INTEGER NOT NULL
);

CREATE TABLE replicators(
    id        INTEGER PRIMARY KEY NOT NULL,
    config    BLOB    NOT NULL,
    pusher_id INTEGER,
    puller_id INTEGER,
    FOREIGN KEY (pusher_id)
        REFERENCES objects(id),
    FOREIGN KEY (puller_id)
        REFERENCES objects(id)
);