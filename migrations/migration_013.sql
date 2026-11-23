-- Audited planning commands reference the immutable inventory prefix observed.
CREATE TABLE project_command (
    sequence INTEGER PRIMARY KEY NOT NULL,
    id TEXT NOT NULL UNIQUE,
    request_key TEXT NOT NULL UNIQUE,
    inventory_sequence INTEGER NOT NULL CHECK(inventory_sequence>=0),
    profile_sequence INTEGER NOT NULL CHECK(profile_sequence>=0),
    committed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    payload_json TEXT NOT NULL CHECK(json_valid(payload_json)),
    result_json TEXT NOT NULL CHECK(json_valid(result_json))
) STRICT;
CREATE TRIGGER project_command_no_update BEFORE UPDATE ON project_command
BEGIN SELECT RAISE(ABORT,'project history is immutable'); END;
CREATE TRIGGER project_command_no_delete BEFORE DELETE ON project_command
BEGIN SELECT RAISE(ABORT,'project history is immutable'); END;
-- Extend only the derived shipping projection's CHECK; preserve audit tables.
DROP TRIGGER shipping_allocation_no_delete;
CREATE TABLE shipping_allocation_next (
    purchase_id TEXT PRIMARY KEY NOT NULL REFERENCES purchase(id),
    revision INTEGER NOT NULL CHECK(revision>0),
    method TEXT NOT NULL CHECK(method IN ('PurchaseLevel','Equal','PriceProportional','PieceCount','Weight','Volume','Manual')),
    snapshot_json TEXT NOT NULL CHECK(json_valid(snapshot_json))
) STRICT;
INSERT INTO shipping_allocation_next SELECT * FROM shipping_allocation;
DROP TABLE shipping_allocation;
ALTER TABLE shipping_allocation_next RENAME TO shipping_allocation;
CREATE TRIGGER shipping_allocation_no_delete BEFORE DELETE ON shipping_allocation
WHEN NOT EXISTS(SELECT 1 FROM purchase_deletion WHERE purchase_id=OLD.purchase_id)
BEGIN SELECT RAISE(ABORT,'shipping allocation must be retained except explicit draft purge'); END;

CREATE TRIGGER project_search_dirty AFTER INSERT ON project_command
BEGIN UPDATE inventory_search_state SET dirty=1 WHERE id=1; END;
