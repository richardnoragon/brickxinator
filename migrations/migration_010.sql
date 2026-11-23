-- Explicit permanent deletion of unreceived, line-free, attachment-free drafts.
-- No entered purchase metadata survives; request identities stay reserved.
CREATE TABLE purchase_deletion (
    purchase_id TEXT PRIMARY KEY NOT NULL,
    request_key TEXT NOT NULL UNIQUE,
    actor TEXT NOT NULL CHECK(length(trim(actor))>0),
    committed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    payload_json TEXT NOT NULL CHECK(json_valid(payload_json))
) STRICT;
CREATE TABLE deleted_request_key (
    request_key TEXT PRIMARY KEY NOT NULL,
    purchase_id TEXT NOT NULL REFERENCES purchase_deletion(purchase_id)
) STRICT;
CREATE TRIGGER deletion_only_empty_drafts BEFORE INSERT ON purchase_deletion
WHEN NOT EXISTS(SELECT 1 FROM purchase WHERE id=NEW.purchase_id AND status!='Received')
  OR EXISTS(SELECT 1 FROM receipt WHERE purchase_id=NEW.purchase_id)
  OR EXISTS(SELECT 1 FROM invoice_line WHERE purchase_id=NEW.purchase_id)
  OR EXISTS(SELECT 1 FROM attachment_association WHERE purchase_id=NEW.purchase_id)
BEGIN SELECT RAISE(ABORT,'only never-received drafts without lines or attachments may be purged'); END;
CREATE TRIGGER deleted_purchase_no_recreate BEFORE INSERT ON purchase
WHEN EXISTS(SELECT 1 FROM purchase_deletion WHERE purchase_id=NEW.id)
BEGIN SELECT RAISE(ABORT,'deleted purchase identity cannot be reused'); END;
CREATE TRIGGER deletion_no_update BEFORE UPDATE ON purchase_deletion BEGIN SELECT RAISE(ABORT,'deletion history is immutable'); END;
CREATE TRIGGER deletion_no_delete BEFORE DELETE ON purchase_deletion BEGIN SELECT RAISE(ABORT,'deletion history is immutable'); END;
CREATE TRIGGER deleted_key_no_update BEFORE UPDATE ON deleted_request_key BEGIN SELECT RAISE(ABORT,'deleted request identities are immutable'); END;
CREATE TRIGGER deleted_key_no_delete BEFORE DELETE ON deleted_request_key BEGIN SELECT RAISE(ABORT,'deleted request identities are immutable'); END;
DROP TRIGGER purchase_command_no_delete;
CREATE TRIGGER purchase_command_no_delete BEFORE DELETE ON purchase_command
WHEN NOT EXISTS(SELECT 1 FROM purchase_deletion WHERE purchase_id=OLD.purchase_id)
BEGIN SELECT RAISE(ABORT,'purchase audit is immutable except explicit draft purge'); END;
DROP TRIGGER shipping_command_no_delete;
CREATE TRIGGER shipping_command_no_delete BEFORE DELETE ON shipping_command
WHEN NOT EXISTS(SELECT 1 FROM purchase_deletion WHERE purchase_id=OLD.purchase_id)
BEGIN SELECT RAISE(ABORT,'shipping audit is immutable except explicit draft purge'); END;
DROP TRIGGER shipping_allocation_no_delete;
CREATE TRIGGER shipping_allocation_no_delete BEFORE DELETE ON shipping_allocation
WHEN NOT EXISTS(SELECT 1 FROM purchase_deletion WHERE purchase_id=OLD.purchase_id)
BEGIN SELECT RAISE(ABORT,'shipping allocation must be retained except explicit draft purge'); END;
