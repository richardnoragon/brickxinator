-- Preserve initial checklist identity while auditing corrected part/color references.
CREATE TABLE verification_attribute_revision (
    operation_id TEXT PRIMARY KEY NOT NULL REFERENCES inventory_operation(id),
    line_id TEXT NOT NULL REFERENCES verification_line_identity(id),
    definition_id TEXT REFERENCES catalog_definition(id),
    color_id TEXT NOT NULL REFERENCES settings_record(id)
) STRICT;
CREATE TRIGGER verification_attribute_no_update BEFORE UPDATE ON verification_attribute_revision BEGIN SELECT RAISE(ABORT,'checklist attribute history is immutable'); END;
CREATE TRIGGER verification_attribute_no_delete BEFORE DELETE ON verification_attribute_revision BEGIN SELECT RAISE(ABORT,'checklist attribute history is immutable'); END;
