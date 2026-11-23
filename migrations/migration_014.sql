-- Physical source layers and the metadata prefix used for reviewed movements.
CREATE TABLE physical_operation (
 operation_id TEXT PRIMARY KEY NOT NULL REFERENCES inventory_operation(id),
 profile_sequence INTEGER NOT NULL CHECK(profile_sequence>=0)
) STRICT;
CREATE TABLE holding_flow (
 operation_id TEXT NOT NULL REFERENCES physical_operation(operation_id),
 source_id TEXT NOT NULL REFERENCES stock_holding(id),
 target_id TEXT NOT NULL REFERENCES stock_holding(id),
 PRIMARY KEY(operation_id,source_id,target_id),
 CHECK(source_id<>target_id)
) STRICT;
CREATE TRIGGER physical_operation_no_update BEFORE UPDATE ON physical_operation BEGIN SELECT RAISE(ABORT,'physical history is immutable'); END;
CREATE TRIGGER physical_operation_no_delete BEFORE DELETE ON physical_operation BEGIN SELECT RAISE(ABORT,'physical history is immutable'); END;
CREATE TRIGGER holding_flow_no_update BEFORE UPDATE ON holding_flow BEGIN SELECT RAISE(ABORT,'source flow is immutable'); END;
CREATE TRIGGER holding_flow_no_delete BEFORE DELETE ON holding_flow BEGIN SELECT RAISE(ABORT,'source flow is immutable'); END;
