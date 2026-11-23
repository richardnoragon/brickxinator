-- Shipping allocation metadata is separate from paid prices and ownership.
CREATE TABLE shipping_allocation (
    purchase_id TEXT PRIMARY KEY NOT NULL REFERENCES purchase(id),
    revision INTEGER NOT NULL CHECK(revision>0),
    method TEXT NOT NULL CHECK(method IN ('PurchaseLevel','Equal','PriceProportional','PieceCount','Weight','Manual')),
    snapshot_json TEXT NOT NULL CHECK(json_valid(snapshot_json))
) STRICT;
CREATE TABLE shipping_command (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    request_key TEXT NOT NULL UNIQUE,
    purchase_id TEXT NOT NULL REFERENCES purchase(id),
    actor TEXT NOT NULL CHECK(length(trim(actor))>0),
    reason TEXT NOT NULL,
    committed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    source_snapshot_json TEXT NOT NULL CHECK(json_valid(source_snapshot_json)),
    payload_json TEXT NOT NULL CHECK(json_valid(payload_json)),
    result_json TEXT NOT NULL CHECK(json_valid(result_json))
) STRICT;
CREATE INDEX shipping_command_purchase ON shipping_command(purchase_id,sequence);
CREATE TRIGGER shipping_allocation_no_delete BEFORE DELETE ON shipping_allocation
BEGIN SELECT RAISE(ABORT,'shipping allocation history must be retained'); END;
CREATE TRIGGER shipping_command_no_update BEFORE UPDATE ON shipping_command
BEGIN SELECT RAISE(ABORT,'shipping audit is immutable'); END;
CREATE TRIGGER shipping_command_no_delete BEFORE DELETE ON shipping_command
BEGIN SELECT RAISE(ABORT,'shipping audit is immutable'); END;
