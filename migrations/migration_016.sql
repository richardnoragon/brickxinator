-- Derived child costs: immutable inputs and reviewed allocations, not paid-price edits.
CREATE TABLE child_cost_command (
 sequence INTEGER PRIMARY KEY AUTOINCREMENT,
 id TEXT NOT NULL UNIQUE,
 request_key TEXT NOT NULL UNIQUE,
 committed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
 payload_json TEXT NOT NULL CHECK(json_valid(payload_json)),
 basis_json TEXT NOT NULL CHECK(json_valid(basis_json)),
 result_json TEXT NOT NULL CHECK(json_valid(result_json))
) STRICT;
CREATE TRIGGER child_cost_no_update BEFORE UPDATE ON child_cost_command BEGIN SELECT RAISE(ABORT,'cost history is immutable'); END;
CREATE TRIGGER child_cost_no_delete BEFORE DELETE ON child_cost_command BEGIN SELECT RAISE(ABORT,'cost history is immutable'); END;
