CREATE TABLE IF NOT EXISTS {0}
(
    id              TEXT     NOT NULL,
    aggregate_id    TEXT     NOT NULL,
    payload         TEXT     NOT NULL,
    occurred_on     TEXT     NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    sequence_number INTEGER  NOT NULL DEFAULT 1,
    version         INTEGER  NULL DEFAULT 0,
    CONSTRAINT {0}_pkey PRIMARY KEY (id)
)
