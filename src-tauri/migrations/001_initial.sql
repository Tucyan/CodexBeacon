CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY NOT NULL,
    value_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS widget_instances (
    id TEXT PRIMARY KEY NOT NULL,
    type TEXT NOT NULL,
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    visible_wanted INTEGER NOT NULL CHECK (visible_wanted IN (0, 1)),
    x REAL NOT NULL,
    y REAL NOT NULL,
    width REAL NOT NULL,
    height REAL NOT NULL,
    scale REAL NOT NULL,
    scale_mode TEXT NOT NULL,
    config_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS memo (
    widget_id TEXT PRIMARY KEY NOT NULL REFERENCES widget_instances(id) ON DELETE CASCADE,
    text TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS todos (
    widget_id TEXT NOT NULL REFERENCES widget_instances(id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    position INTEGER NOT NULL,
    text TEXT NOT NULL,
    completed INTEGER NOT NULL CHECK (completed IN (0, 1)),
    PRIMARY KEY (widget_id, id)
);

CREATE TABLE IF NOT EXISTS countdowns (
    widget_id TEXT NOT NULL REFERENCES widget_instances(id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    position INTEGER NOT NULL,
    name TEXT NOT NULL,
    target REAL NOT NULL,
    PRIMARY KEY (widget_id, id)
);
