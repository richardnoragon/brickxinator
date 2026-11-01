-- Catalog descriptions and descriptive ownership links; no balance columns.
CREATE TABLE catalog_definition (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN ('Part','Set')),
    revision INTEGER NOT NULL CHECK(revision>0),
    manufacturer_id TEXT NOT NULL REFERENCES settings_record(id),
    identity_number TEXT,
    archived INTEGER NOT NULL CHECK(archived IN (0,1)),
    snapshot_json TEXT NOT NULL CHECK(json_valid(snapshot_json)),
    UNIQUE(kind,manufacturer_id,identity_number)
) STRICT;
CREATE TABLE catalog_command (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    request_key TEXT NOT NULL UNIQUE,
    actor TEXT NOT NULL,
    reason TEXT NOT NULL,
    committed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    target_id TEXT NOT NULL REFERENCES catalog_definition(id),
    payload_json TEXT NOT NULL CHECK(json_valid(payload_json)),
    result_json TEXT NOT NULL CHECK(json_valid(result_json))
) STRICT;
CREATE INDEX catalog_command_target ON catalog_command(target_id,sequence);
CREATE TABLE inventory_profile (
    target_kind TEXT NOT NULL CHECK(target_kind IN ('Holding','SetCopy')),
    target_id TEXT NOT NULL,
    holding_id TEXT REFERENCES stock_holding(id),
    set_copy_id TEXT REFERENCES owned_set_copy(id),
    revision INTEGER NOT NULL CHECK(revision>0),
    definition_id TEXT REFERENCES catalog_definition(id),
    color_id TEXT NOT NULL REFERENCES settings_record(id),
    condition_id TEXT NOT NULL REFERENCES settings_record(id),
    location_id TEXT REFERENCES settings_record(id),
    snapshot_json TEXT NOT NULL CHECK(json_valid(snapshot_json)),
    PRIMARY KEY(target_kind,target_id),
    CHECK((target_kind='Holding' AND target_id=holding_id AND holding_id IS NOT NULL AND set_copy_id IS NULL) OR
          (target_kind='SetCopy' AND target_id=set_copy_id AND set_copy_id IS NOT NULL AND holding_id IS NULL))
) STRICT;
CREATE INDEX profile_definition ON inventory_profile(definition_id);
CREATE INDEX profile_location ON inventory_profile(location_id);
CREATE TABLE profile_command (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    request_key TEXT NOT NULL UNIQUE,
    actor TEXT NOT NULL,
    reason TEXT NOT NULL,
    committed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    target_kind TEXT NOT NULL,
    target_id TEXT NOT NULL,
    related_operation_id TEXT REFERENCES inventory_operation(id),
    payload_json TEXT NOT NULL CHECK(json_valid(payload_json)),
    result_json TEXT NOT NULL CHECK(json_valid(result_json)),
    FOREIGN KEY(target_kind,target_id) REFERENCES inventory_profile(target_kind,target_id)
) STRICT;
CREATE INDEX profile_command_target ON profile_command(target_kind,target_id,sequence);
CREATE TRIGGER catalog_definition_no_delete BEFORE DELETE ON catalog_definition
BEGIN SELECT RAISE(ABORT, 'descriptive history must be retained'); END;
CREATE TRIGGER inventory_profile_no_delete BEFORE DELETE ON inventory_profile
BEGIN SELECT RAISE(ABORT, 'descriptive history must be retained'); END;
CREATE TRIGGER catalog_command_no_update BEFORE UPDATE ON catalog_command
BEGIN SELECT RAISE(ABORT, 'descriptive audit is immutable'); END;
CREATE TRIGGER catalog_command_no_delete BEFORE DELETE ON catalog_command
BEGIN SELECT RAISE(ABORT, 'descriptive audit is immutable'); END;
CREATE TRIGGER profile_command_no_update BEFORE UPDATE ON profile_command
BEGIN SELECT RAISE(ABORT, 'descriptive audit is immutable'); END;
CREATE TRIGGER profile_command_no_delete BEFORE DELETE ON profile_command
BEGIN SELECT RAISE(ABORT, 'descriptive audit is immutable'); END;
