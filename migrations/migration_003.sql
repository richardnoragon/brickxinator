-- Managed vocabularies and location tree. Prior migrations remain immutable.
CREATE TABLE settings_record (
    id TEXT PRIMARY KEY NOT NULL,
    revision INTEGER NOT NULL CHECK(revision > 0),
    display_name TEXT NOT NULL CHECK(length(trim(display_name)) > 0),
    normalized_name TEXT NOT NULL,
    archived INTEGER NOT NULL CHECK(archived IN (0,1)),
    vocabulary TEXT CHECK(vocabulary IN ('Manufacturer','Color','Condition')),
    system_unknown INTEGER NOT NULL CHECK(system_unknown IN (0,1)),
    is_location INTEGER NOT NULL CHECK(is_location IN (0,1)),
    parent_id TEXT REFERENCES settings_record(id),
    CHECK(parent_id IS NULL OR parent_id != id),
    CHECK((is_location=1 AND vocabulary IS NULL AND system_unknown=0) OR
          (is_location=0 AND vocabulary IS NOT NULL AND parent_id IS NULL)),
    CHECK(system_unknown=0 OR (display_name='Unknown' AND archived=0 AND revision=1))
) STRICT;
CREATE UNIQUE INDEX protected_unknown_kind ON settings_record(vocabulary) WHERE system_unknown=1;
CREATE INDEX settings_parent ON settings_record(parent_id);
CREATE INDEX settings_name ON settings_record(vocabulary,normalized_name);
CREATE TABLE settings_command (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    request_key TEXT NOT NULL UNIQUE,
    actor TEXT NOT NULL CHECK(length(trim(actor))>0),
    reason TEXT NOT NULL,
    committed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    target_id TEXT NOT NULL REFERENCES settings_record(id),
    payload_json TEXT NOT NULL CHECK(json_valid(payload_json)),
    result_json TEXT NOT NULL CHECK(json_valid(result_json))
) STRICT;
CREATE INDEX settings_command_target ON settings_command(target_id,sequence);
INSERT INTO settings_record VALUES('01900000-0000-7000-8000-000000000001',1,'Unknown','unknown',0,'Manufacturer',1,0,NULL);
INSERT INTO settings_record VALUES('01900000-0000-7000-8000-000000000002',1,'Unknown','unknown',0,'Color',1,0,NULL);
INSERT INTO settings_record VALUES('01900000-0000-7000-8000-000000000003',1,'Unknown','unknown',0,'Condition',1,0,NULL);
INSERT INTO settings_record VALUES('01900000-0000-7000-8000-000000000004',1,'New','new',0,'Condition',0,0,NULL);
INSERT INTO settings_record VALUES('01900000-0000-7000-8000-000000000005',1,'Used','used',0,'Condition',0,0,NULL);
INSERT INTO settings_record VALUES('01900000-0000-7000-8000-000000000006',1,'Mixed','mixed',0,'Condition',0,0,NULL);
CREATE TRIGGER settings_unknown_no_update BEFORE UPDATE ON settings_record WHEN OLD.system_unknown=1 OR NEW.system_unknown != OLD.system_unknown
BEGIN SELECT RAISE(ABORT, 'system Unknown is protected'); END;
CREATE TRIGGER settings_no_delete BEFORE DELETE ON settings_record
BEGIN SELECT RAISE(ABORT, 'settings records must be archived'); END;
CREATE TRIGGER settings_audit_no_update BEFORE UPDATE ON settings_command
BEGIN SELECT RAISE(ABORT, 'settings audit is immutable'); END;
CREATE TRIGGER settings_audit_no_delete BEFORE DELETE ON settings_command
BEGIN SELECT RAISE(ABORT, 'settings audit is immutable'); END;
