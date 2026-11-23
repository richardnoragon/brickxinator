-- Purchase metadata and receipt provenance. Do not change migration 001.
CREATE TABLE purchase (
    id TEXT PRIMARY KEY NOT NULL,
    revision INTEGER NOT NULL CHECK(revision > 0),
    status TEXT NOT NULL CHECK(status IN ('Draft','Ordered','Shipped','Received','Cancelled')),
    snapshot_json TEXT NOT NULL CHECK(json_valid(snapshot_json))
) STRICT;
CREATE TABLE invoice_line (
    id TEXT PRIMARY KEY NOT NULL,
    purchase_id TEXT NOT NULL REFERENCES purchase(id),
    position INTEGER NOT NULL CHECK(position >= 0),
    snapshot_json TEXT NOT NULL CHECK(json_valid(snapshot_json)),
    UNIQUE(purchase_id, position)
) STRICT;
CREATE INDEX invoice_line_purchase ON invoice_line(purchase_id);
CREATE TABLE purchase_command (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    request_key TEXT NOT NULL UNIQUE,
    purchase_id TEXT NOT NULL REFERENCES purchase(id),
    actor TEXT NOT NULL CHECK(length(trim(actor)) > 0),
    reason TEXT NOT NULL,
    committed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    payload_json TEXT NOT NULL CHECK(json_valid(payload_json)),
    before_json TEXT CHECK(before_json IS NULL OR json_valid(before_json)),
    after_json TEXT NOT NULL CHECK(json_valid(after_json)),
    result_json TEXT NOT NULL CHECK(json_valid(result_json))
) STRICT;
CREATE INDEX purchase_command_purchase ON purchase_command(purchase_id, sequence);
CREATE TABLE receipt (
    purchase_id TEXT PRIMARY KEY NOT NULL REFERENCES purchase(id),
    operation_id TEXT NOT NULL UNIQUE REFERENCES inventory_operation(id),
    received_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE TABLE stock_provenance (
    holding_id TEXT PRIMARY KEY NOT NULL REFERENCES stock_holding(id),
    invoice_line_id TEXT NOT NULL REFERENCES invoice_line(id),
    parent_holding_id TEXT REFERENCES stock_holding(id)
) STRICT;
CREATE INDEX stock_provenance_line ON stock_provenance(invoice_line_id);
CREATE TABLE owned_set_copy (
    id TEXT PRIMARY KEY NOT NULL,
    invoice_line_id TEXT NOT NULL REFERENCES invoice_line(id),
    receipt_operation_id TEXT NOT NULL REFERENCES inventory_operation(id),
    snapshot_json TEXT NOT NULL CHECK(json_valid(snapshot_json))
) STRICT;
CREATE INDEX owned_set_copy_line ON owned_set_copy(invoice_line_id);
CREATE TRIGGER purchase_command_no_update BEFORE UPDATE ON purchase_command
BEGIN SELECT RAISE(ABORT, 'purchase audit is immutable'); END;
CREATE TRIGGER purchase_command_no_delete BEFORE DELETE ON purchase_command
BEGIN SELECT RAISE(ABORT, 'purchase audit is immutable'); END;
CREATE TRIGGER receipt_no_update BEFORE UPDATE ON receipt
BEGIN SELECT RAISE(ABORT, 'receipt is immutable'); END;
CREATE TRIGGER receipt_no_delete BEFORE DELETE ON receipt
BEGIN SELECT RAISE(ABORT, 'receipt is immutable'); END;
CREATE TRIGGER provenance_no_update BEFORE UPDATE ON stock_provenance
BEGIN SELECT RAISE(ABORT, 'stock provenance is immutable'); END;
CREATE TRIGGER provenance_no_delete BEFORE DELETE ON stock_provenance
BEGIN SELECT RAISE(ABORT, 'stock provenance is immutable'); END;
CREATE TRIGGER set_copy_no_update BEFORE UPDATE ON owned_set_copy
BEGIN SELECT RAISE(ABORT, 'initial set ownership is immutable'); END;
CREATE TRIGGER set_copy_no_delete BEFORE DELETE ON owned_set_copy
BEGIN SELECT RAISE(ABORT, 'initial set ownership is immutable'); END;
