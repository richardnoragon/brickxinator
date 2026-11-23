-- Immutable after introduction. Correct schema changes with later migrations.
-- Minimal standalone inventory journal; purchases/sets follow in later migrations.
CREATE TABLE schema_migration (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    checksum TEXT NOT NULL,
    applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;

CREATE TABLE stock_holding (
    id TEXT PRIMARY KEY NOT NULL
) STRICT;

CREATE TABLE inventory_operation (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    request_key TEXT NOT NULL UNIQUE CHECK(length(trim(request_key)) > 0),
    actor TEXT NOT NULL CHECK(length(trim(actor)) > 0),
    reason TEXT NOT NULL CHECK(length(trim(reason)) > 0),
    committed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    event_schema_version INTEGER NOT NULL CHECK(event_schema_version = 1),
    payload_json TEXT NOT NULL CHECK(json_valid(payload_json))
) STRICT;

CREATE TABLE operation_target (
    operation_id TEXT NOT NULL REFERENCES inventory_operation(id),
    holding_id TEXT NOT NULL REFERENCES stock_holding(id),
    PRIMARY KEY(operation_id, holding_id)
) STRICT;
CREATE INDEX operation_target_holding ON operation_target(holding_id);

CREATE TRIGGER operation_no_update BEFORE UPDATE ON inventory_operation
BEGIN SELECT RAISE(ABORT, 'inventory history is immutable'); END;
CREATE TRIGGER operation_no_delete BEFORE DELETE ON inventory_operation
BEGIN SELECT RAISE(ABORT, 'inventory history is immutable'); END;
CREATE TRIGGER target_no_update BEFORE UPDATE ON operation_target
BEGIN SELECT RAISE(ABORT, 'inventory targets are immutable'); END;
CREATE TRIGGER target_no_delete BEFORE DELETE ON operation_target
BEGIN SELECT RAISE(ABORT, 'inventory targets are immutable'); END;
CREATE TRIGGER holding_no_update BEFORE UPDATE ON stock_holding
BEGIN SELECT RAISE(ABORT, 'holding identities are immutable'); END;
CREATE TRIGGER holding_no_delete BEFORE DELETE ON stock_holding
BEGIN SELECT RAISE(ABORT, 'holding identities are immutable'); END;
