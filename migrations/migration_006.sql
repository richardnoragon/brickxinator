-- Typed adjustment and lot lifecycle references; facts remain in the journal.
CREATE TABLE stock_operation (
    operation_id TEXT PRIMARY KEY NOT NULL REFERENCES inventory_operation(id),
    holding_id TEXT NOT NULL REFERENCES stock_holding(id),
    action_kind TEXT NOT NULL CHECK(action_kind IN ('Lost','Donated','Sold','Broken','Correction','Complete','Reopen'))
) STRICT;
CREATE INDEX stock_operation_holding ON stock_operation(holding_id);
CREATE TRIGGER stock_operation_no_update BEFORE UPDATE ON stock_operation
BEGIN SELECT RAISE(ABORT,'stock action history is immutable'); END;
CREATE TRIGGER stock_operation_no_delete BEFORE DELETE ON stock_operation
BEGIN SELECT RAISE(ABORT,'stock action history is immutable'); END;
