CREATE TABLE fulfilment_command (
 sequence INTEGER PRIMARY KEY AUTOINCREMENT,
 id TEXT NOT NULL UNIQUE,
 request_key TEXT NOT NULL UNIQUE,
 purchase_id TEXT NOT NULL REFERENCES purchase(id),
 inventory_sequence INTEGER NOT NULL CHECK(inventory_sequence>=0),
 profile_sequence INTEGER NOT NULL CHECK(profile_sequence>=0),
 committed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 source_snapshot_json TEXT NOT NULL CHECK(json_valid(source_snapshot_json)),
 payload_json TEXT NOT NULL CHECK(json_valid(payload_json)),
 result_json TEXT NOT NULL CHECK(json_valid(result_json))
) STRICT;
CREATE INDEX fulfilment_purchase ON fulfilment_command(purchase_id,sequence);
CREATE TRIGGER fulfilment_no_update BEFORE UPDATE ON fulfilment_command BEGIN SELECT RAISE(ABORT,'fulfilment history is immutable'); END;
CREATE TRIGGER fulfilment_no_delete BEFORE DELETE ON fulfilment_command
WHEN NOT EXISTS(SELECT 1 FROM purchase_deletion WHERE purchase_id=OLD.purchase_id) BEGIN SELECT RAISE(ABORT,'fulfilment history is immutable'); END;
CREATE TRIGGER fulfilment_search_dirty AFTER INSERT ON fulfilment_command BEGIN UPDATE inventory_search_state SET dirty=1 WHERE id=1; END;
