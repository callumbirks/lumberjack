-- Your SQL goes here
CREATE TABLE lines(
    level      INTEGER unsigned NOT NULL,
    line_num   BIGINT  unsigned NOT NULL,
    timestamp  TIMESTAMP        NOT NULL,
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
    id        INTEGER   PRIMARY KEY NOT NULL,
    path      TEXT      NOT NULL,
    level     INTEGER   NOT NULL,
    timestamp TIMESTAMP NOT NULL
);

CREATE TABLE objects(
    id    INTEGER PRIMARY KEY NOT NULL,
    ty    INTEGER NOT NULL
);

CREATE TABLE repls(
    object_id INTEGER PRIMARY KEY NOT NULL,
    config    TEXT    NOT NULL,
    FOREIGN KEY (object_id)
        REFERENCES objects(id)
);