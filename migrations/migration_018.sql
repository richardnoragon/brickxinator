-- Reviewed offline color/theme enrichment; source and local override decisions persist.
CREATE TABLE enrichment_command (
 sequence INTEGER PRIMARY KEY AUTOINCREMENT,
 id TEXT NOT NULL UNIQUE,
 request_key TEXT NOT NULL UNIQUE,
 committed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 settings_sequence INTEGER NOT NULL CHECK(settings_sequence>=0),
 source_sha256 TEXT NOT NULL,
 payload_json TEXT NOT NULL CHECK(json_valid(payload_json)),
 result_json TEXT NOT NULL CHECK(json_valid(result_json))
) STRICT;
CREATE TRIGGER enrichment_no_update BEFORE UPDATE ON enrichment_command BEGIN SELECT RAISE(ABORT,'enrichment history is immutable'); END;
CREATE TRIGGER enrichment_no_delete BEFORE DELETE ON enrichment_command BEGIN SELECT RAISE(ABORT,'enrichment history is immutable'); END;
