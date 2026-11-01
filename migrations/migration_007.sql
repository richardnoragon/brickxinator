-- Rebuildable inventory/search cache. Domain UUIDs remain separate from rowids.
CREATE TABLE IF NOT EXISTS inventory_search_document (
    rowid INTEGER PRIMARY KEY,
    target_kind TEXT NOT NULL,
    target_id TEXT NOT NULL,
    row_kind TEXT NOT NULL,
    normalized_number TEXT,
    manufacturer_id TEXT NOT NULL,
    color_id TEXT NOT NULL,
    condition_id TEXT NOT NULL,
    location_id TEXT,
    location_ids_json TEXT NOT NULL CHECK(json_valid(location_ids_json)),
    available INTEGER NOT NULL,
    assembled INTEGER NOT NULL,
    damaged INTEGER NOT NULL,
    unknown_measure INTEGER NOT NULL,
    incomplete INTEGER NOT NULL,
    archived_reference INTEGER NOT NULL,
    snapshot_json TEXT NOT NULL CHECK(json_valid(snapshot_json)),
    UNIQUE(target_kind,target_id)
) STRICT;
CREATE INDEX IF NOT EXISTS inventory_search_number ON inventory_search_document(normalized_number);
CREATE INDEX IF NOT EXISTS inventory_search_location ON inventory_search_document(location_id);
CREATE INDEX IF NOT EXISTS inventory_search_attributes ON inventory_search_document(manufacturer_id,color_id,condition_id);
CREATE VIRTUAL TABLE IF NOT EXISTS inventory_search_fts USING fts5(search_text,tokenize='unicode61');
CREATE TABLE IF NOT EXISTS inventory_search_state (
    id INTEGER PRIMARY KEY CHECK(id=1),
    dirty INTEGER NOT NULL CHECK(dirty IN (0,1)),
    format_version INTEGER NOT NULL,
    dashboard_json TEXT CHECK(dashboard_json IS NULL OR json_valid(dashboard_json))
) STRICT;
INSERT OR IGNORE INTO inventory_search_state(id,dirty,format_version) VALUES(1,1,1);
CREATE TRIGGER IF NOT EXISTS inventory_search_dirty_inventory_operation_insert AFTER INSERT ON inventory_operation
BEGIN UPDATE inventory_search_state SET dirty=1 WHERE id=1; END;
CREATE TRIGGER IF NOT EXISTS inventory_search_dirty_inventory_operation_update AFTER UPDATE ON inventory_operation
BEGIN UPDATE inventory_search_state SET dirty=1 WHERE id=1; END;
CREATE TRIGGER IF NOT EXISTS inventory_search_dirty_inventory_operation_delete AFTER DELETE ON inventory_operation
BEGIN UPDATE inventory_search_state SET dirty=1 WHERE id=1; END;
CREATE TRIGGER IF NOT EXISTS inventory_search_dirty_purchase_insert AFTER INSERT ON purchase
BEGIN UPDATE inventory_search_state SET dirty=1 WHERE id=1; END;
CREATE TRIGGER IF NOT EXISTS inventory_search_dirty_purchase_update AFTER UPDATE ON purchase
BEGIN UPDATE inventory_search_state SET dirty=1 WHERE id=1; END;
CREATE TRIGGER IF NOT EXISTS inventory_search_dirty_purchase_delete AFTER DELETE ON purchase
BEGIN UPDATE inventory_search_state SET dirty=1 WHERE id=1; END;
CREATE TRIGGER IF NOT EXISTS inventory_search_dirty_invoice_line_insert AFTER INSERT ON invoice_line
BEGIN UPDATE inventory_search_state SET dirty=1 WHERE id=1; END;
CREATE TRIGGER IF NOT EXISTS inventory_search_dirty_invoice_line_update AFTER UPDATE ON invoice_line
BEGIN UPDATE inventory_search_state SET dirty=1 WHERE id=1; END;
CREATE TRIGGER IF NOT EXISTS inventory_search_dirty_invoice_line_delete AFTER DELETE ON invoice_line
BEGIN UPDATE inventory_search_state SET dirty=1 WHERE id=1; END;
CREATE TRIGGER IF NOT EXISTS inventory_search_dirty_catalog_definition_insert AFTER INSERT ON catalog_definition
BEGIN UPDATE inventory_search_state SET dirty=1 WHERE id=1; END;
CREATE TRIGGER IF NOT EXISTS inventory_search_dirty_catalog_definition_update AFTER UPDATE ON catalog_definition
BEGIN UPDATE inventory_search_state SET dirty=1 WHERE id=1; END;
CREATE TRIGGER IF NOT EXISTS inventory_search_dirty_catalog_definition_delete AFTER DELETE ON catalog_definition
BEGIN UPDATE inventory_search_state SET dirty=1 WHERE id=1; END;
CREATE TRIGGER IF NOT EXISTS inventory_search_dirty_inventory_profile_insert AFTER INSERT ON inventory_profile
BEGIN UPDATE inventory_search_state SET dirty=1 WHERE id=1; END;
CREATE TRIGGER IF NOT EXISTS inventory_search_dirty_inventory_profile_update AFTER UPDATE ON inventory_profile
BEGIN UPDATE inventory_search_state SET dirty=1 WHERE id=1; END;
CREATE TRIGGER IF NOT EXISTS inventory_search_dirty_inventory_profile_delete AFTER DELETE ON inventory_profile
BEGIN UPDATE inventory_search_state SET dirty=1 WHERE id=1; END;
CREATE TRIGGER IF NOT EXISTS inventory_search_dirty_settings_record_insert AFTER INSERT ON settings_record
BEGIN UPDATE inventory_search_state SET dirty=1 WHERE id=1; END;
CREATE TRIGGER IF NOT EXISTS inventory_search_dirty_settings_record_update AFTER UPDATE ON settings_record
BEGIN UPDATE inventory_search_state SET dirty=1 WHERE id=1; END;
CREATE TRIGGER IF NOT EXISTS inventory_search_dirty_settings_record_delete AFTER DELETE ON settings_record
BEGIN UPDATE inventory_search_state SET dirty=1 WHERE id=1; END;
