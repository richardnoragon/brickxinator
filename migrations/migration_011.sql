CREATE TABLE purchase_reconciliation (
    purchase_command_id TEXT NOT NULL REFERENCES purchase_command(id),
    operation_id TEXT NOT NULL UNIQUE REFERENCES inventory_operation(id),
    PRIMARY KEY(purchase_command_id,operation_id)
) STRICT;
CREATE TRIGGER reconciliation_no_update BEFORE UPDATE ON purchase_reconciliation BEGIN SELECT RAISE(ABORT,'purchase reconciliation is immutable'); END;
CREATE TRIGGER reconciliation_no_delete BEFORE DELETE ON purchase_reconciliation BEGIN SELECT RAISE(ABORT,'purchase reconciliation is immutable'); END;
