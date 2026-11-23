-- Verification quantities and lifecycle remain in the immutable operation journal.
CREATE TABLE verification_line_identity (
    id TEXT PRIMARY KEY NOT NULL,
    copy_id TEXT NOT NULL REFERENCES owned_set_copy(id),
    reserved_holding_id TEXT NOT NULL UNIQUE,
    definition_id TEXT REFERENCES catalog_definition(id),
    color_id TEXT NOT NULL REFERENCES settings_record(id),
    UNIQUE(id,copy_id)
) STRICT;
CREATE TABLE verified_content (
    holding_id TEXT PRIMARY KEY NOT NULL REFERENCES stock_holding(id),
    copy_id TEXT NOT NULL REFERENCES owned_set_copy(id),
    line_id TEXT NOT NULL UNIQUE,
    FOREIGN KEY(line_id,copy_id) REFERENCES verification_line_identity(id,copy_id)
) STRICT;
CREATE TABLE verification_operation (
    operation_id TEXT PRIMARY KEY NOT NULL REFERENCES inventory_operation(id),
    copy_id TEXT NOT NULL REFERENCES owned_set_copy(id)
) STRICT;
CREATE INDEX verified_content_copy ON verified_content(copy_id);
CREATE INDEX verification_line_copy ON verification_line_identity(copy_id);
CREATE INDEX verification_operation_copy ON verification_operation(copy_id);
CREATE TRIGGER verification_line_no_update BEFORE UPDATE ON verification_line_identity
BEGIN SELECT RAISE(ABORT,'verification identities are immutable'); END;
CREATE TRIGGER verification_line_no_delete BEFORE DELETE ON verification_line_identity
BEGIN SELECT RAISE(ABORT,'verification identities are immutable'); END;
CREATE TRIGGER verified_content_no_update BEFORE UPDATE ON verified_content
BEGIN SELECT RAISE(ABORT,'set-content membership is immutable'); END;
CREATE TRIGGER verified_content_no_delete BEFORE DELETE ON verified_content
BEGIN SELECT RAISE(ABORT,'set-content membership is immutable'); END;
CREATE TRIGGER verification_operation_no_update BEFORE UPDATE ON verification_operation
BEGIN SELECT RAISE(ABORT,'verification history is immutable'); END;
CREATE TRIGGER verification_operation_no_delete BEFORE DELETE ON verification_operation
BEGIN SELECT RAISE(ABORT,'verification history is immutable'); END;
