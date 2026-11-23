-- Local, immutable source cache and reviewed catalog enrichment batches.
CREATE TABLE integration_import (
 sequence INTEGER PRIMARY KEY AUTOINCREMENT,
 id TEXT NOT NULL UNIQUE,
 request_key TEXT NOT NULL UNIQUE,
 committed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 source_sha256 TEXT NOT NULL,
 payload_json TEXT NOT NULL CHECK(json_valid(payload_json)),
 result_json TEXT NOT NULL CHECK(json_valid(result_json))
) STRICT;
CREATE TRIGGER integration_no_update BEFORE UPDATE ON integration_import BEGIN SELECT RAISE(ABORT,'import history is immutable'); END;
CREATE TRIGGER integration_no_delete BEFORE DELETE ON integration_import BEGIN SELECT RAISE(ABORT,'import history is immutable'); END;
