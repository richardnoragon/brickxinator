CREATE TABLE attachment (
    id TEXT PRIMARY KEY NOT NULL,
    relative_path TEXT NOT NULL UNIQUE,
    original_name TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    byte_size INTEGER NOT NULL CHECK(byte_size>0),
    media_type TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE TABLE attachment_association (
    id TEXT PRIMARY KEY NOT NULL,
    attachment_id TEXT NOT NULL REFERENCES attachment(id),
    purchase_id TEXT REFERENCES purchase(id),
    invoice_line_id TEXT REFERENCES invoice_line(id),
    definition_id TEXT REFERENCES catalog_definition(id),
    set_copy_id TEXT REFERENCES owned_set_copy(id),
    CHECK((purchase_id IS NOT NULL)+(invoice_line_id IS NOT NULL)+(definition_id IS NOT NULL)+(set_copy_id IS NOT NULL)=1)
) STRICT;
CREATE TABLE attachment_command (
    request_key TEXT PRIMARY KEY NOT NULL,
    attachment_id TEXT NOT NULL UNIQUE REFERENCES attachment(id),
    actor TEXT NOT NULL,
    payload_json TEXT NOT NULL CHECK(json_valid(payload_json)),
    result_json TEXT NOT NULL CHECK(json_valid(result_json)),
    committed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE TRIGGER attachment_no_update BEFORE UPDATE ON attachment BEGIN SELECT RAISE(ABORT,'attachment bytes are immutable'); END;
CREATE TRIGGER attachment_no_delete BEFORE DELETE ON attachment BEGIN SELECT RAISE(ABORT,'attachments must be retained'); END;
CREATE TRIGGER attachment_link_no_update BEFORE UPDATE ON attachment_association BEGIN SELECT RAISE(ABORT,'attachment associations are immutable'); END;
CREATE TRIGGER attachment_link_no_delete BEFORE DELETE ON attachment_association BEGIN SELECT RAISE(ABORT,'attachment associations must be retained'); END;
CREATE TRIGGER attachment_audit_no_update BEFORE UPDATE ON attachment_command BEGIN SELECT RAISE(ABORT,'attachment history is immutable'); END;
CREATE TRIGGER attachment_audit_no_delete BEFORE DELETE ON attachment_command BEGIN SELECT RAISE(ABORT,'attachment history is immutable'); END;
