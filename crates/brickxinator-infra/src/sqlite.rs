//! SQLite journal adapter. Each command reads and validates authoritative history
//! under an IMMEDIATE transaction before appending. Replay is intentionally
//! unoptimized until rebuildable projections are introduced.

use brickxinator_core::ledger::{Body, Ledger, LedgerError, Operation, Outcome};
use rusqlite::{Connection, OpenFlags, TransactionBehavior, params};
use sha2::{Digest, Sha256};
use std::{error::Error, fs::OpenOptions, path::Path, time::Duration};

pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;
const MIGRATION: &str = include_str!("../../../migrations/migration_001.sql");
pub(crate) const MIGRATIONS: &[(&str, &str)] = &[
    ("migration_001.sql", MIGRATION),
    (
        "migration_002.sql",
        include_str!("../../../migrations/migration_002.sql"),
    ),
    (
        "migration_003.sql",
        include_str!("../../../migrations/migration_003.sql"),
    ),
    (
        "migration_004.sql",
        include_str!("../../../migrations/migration_004.sql"),
    ),
    (
        "migration_005.sql",
        include_str!("../../../migrations/migration_005.sql"),
    ),
    (
        "migration_006.sql",
        include_str!("../../../migrations/migration_006.sql"),
    ),
    (
        "migration_007.sql",
        include_str!("../../../migrations/migration_007.sql"),
    ),
    (
        "migration_008.sql",
        include_str!("../../../migrations/migration_008.sql"),
    ),
    (
        "migration_009.sql",
        include_str!("../../../migrations/migration_009.sql"),
    ),
    (
        "migration_010.sql",
        include_str!("../../../migrations/migration_010.sql"),
    ),
    (
        "migration_011.sql",
        include_str!("../../../migrations/migration_011.sql"),
    ),
    (
        "migration_012.sql",
        include_str!("../../../migrations/migration_012.sql"),
    ),
];

pub struct Store {
    pub(crate) connection: Connection,
    pub(crate) database: std::path::PathBuf,
    _lease: Option<crate::maintenance::Lease>,
}

impl Store {
    /// Reserves a NEW file exclusively. Never migrates an existing database.
    /// No destructive upgrade path exists yet; later upgrades require a safety
    /// snapshot and cross-process maintenance locking before they can be added.
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        let path = crate::maintenance::database_path(path.as_ref())?;
        let lease = crate::maintenance::shared(&path)?;
        if crate::recovery::pending(&path) {
            return Err("restore was interrupted; run recover before opening this database".into());
        }
        let path = path.as_path();
        let reservation = OpenOptions::new().write(true).create_new(true).open(path)?;
        drop(reservation);
        let mut connection = connect(path)?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Exclusive)?;
        for (index, (name, sql)) in MIGRATIONS.iter().enumerate() {
            tx.execute_batch(sql)?;
            tx.execute(
                "INSERT INTO schema_migration(version,name,checksum) VALUES(?1,?2,?3)",
                params![i64::try_from(index + 1)?, name, checksum(sql)],
            )?;
        }
        check_integrity(&tx)?;
        tx.commit()?;
        Ok(Self {
            connection,
            database: path.to_path_buf(),
            _lease: Some(lease),
        })
    }

    /// Opens an existing, current-schema database without applying migrations.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = crate::maintenance::database_path(path.as_ref())?;
        let lease = crate::maintenance::shared(&path)?;
        if crate::recovery::pending(&path) {
            return Err("restore was interrupted; run recover before opening this database".into());
        }
        Self::open_validated(&path, Some(lease))
    }

    pub(crate) fn under_maintenance(guard: &crate::maintenance::Maintenance) -> Result<Self> {
        Self::open_validated(&guard.database, None)
    }

    fn open_validated(path: &Path, lease: Option<crate::maintenance::Lease>) -> Result<Self> {
        let connection = connect(path)?;
        let migrations: Vec<(i64, String, String)> = connection
            .prepare("SELECT version,name,checksum FROM schema_migration ORDER BY version")?
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<std::result::Result<_, _>>()?;
        let expected: Vec<_> = MIGRATIONS
            .iter()
            .enumerate()
            .map(|(index, (name, sql))| ((index + 1) as i64, name.to_string(), checksum(sql)))
            .collect();
        if migrations.is_empty()
            || migrations.len() > expected.len()
            || migrations != expected[..migrations.len()]
        {
            return Err("unsupported schema or historical migration checksum mismatch".into());
        }
        check_integrity(&connection)?;
        let store = Self {
            connection,
            database: path.to_path_buf(),
            _lease: lease,
        };
        store.rebuild()?;
        crate::purge::validate(&store.connection)?;
        if has_settings(&store.connection)? {
            store.settings()?;
        }
        if has_catalog(&store.connection)? {
            store.catalog()?;
            store.inventory_profiles()?;
        }
        if has_finance(&store.connection)? {
            store.allocation_states()?;
        }
        Ok(store)
    }

    pub fn commit(&mut self, operation: Operation) -> Result<Outcome> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut ledger = load(&tx)?;
        if matches!(
            operation.body,
            Body::Receive { .. }
                | Body::AddCopies { .. }
                | Body::Verify { .. }
                | Body::Stock { .. }
        ) {
            return Err(
                "receipts, verification and stock actions require their domain command methods"
                    .into(),
            );
        }
        if let Body::Change {
            holding_id,
            pieces,
            weight_mg,
            ..
        } = &operation.body
            && has_stock_actions(&tx)?
            && ledger.set_contents().contains_key(holding_id)
            && (pieces.owned != brickxinator_core::journal::Action::Unchanged
                || weight_mg.owned != brickxinator_core::journal::Action::Unchanged)
            && !ledger
                .operations()
                .iter()
                .any(|op| op.request_key == operation.request_key)
        {
            return Err(
                "set-owned quantities require verification or a named stock adjustment".into(),
            );
        }
        if let Body::Identify { source_id, .. } = operation.body
            && let Some(line_id) = ledger.provenance().get(&source_id)
        {
            let (json, parent): (String, Option<String>) = tx.query_row(
                "SELECT invoice_line.snapshot_json,stock_provenance.parent_holding_id FROM stock_provenance JOIN invoice_line ON invoice_line.id=stock_provenance.invoice_line_id WHERE holding_id=?1 AND invoice_line.id=?2",
                params![source_id.to_string(), line_id.to_string()], |r| Ok((r.get(0)?,r.get(1)?)),
            )?;
            let line: brickxinator_core::purchases::InvoiceLine = serde_json::from_str(&json)?;
            if line.kind != brickxinator_core::purchases::LineKind::MixedLot || parent.is_some() {
                return Err("only an original mixed lot can be classified; arbitrary batch splitting is deferred".into());
            }
        }
        if has_purchases(&tx)?
            && tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM purchase_command WHERE request_key=?1)",
                [&operation.request_key],
                |r| r.get::<_, bool>(0),
            )?
        {
            return Err("request key already belongs to a purchase command".into());
        }
        if metadata_key_exists(&tx, &operation.request_key)? {
            return Err("request key belongs to a metadata command".into());
        }
        let count = ledger.operations().len();
        let outcome = ledger.append(operation.clone())?;
        if ledger.operations().len() != count {
            persist_operation(&tx, &ledger, &operation, &outcome)?;
        }
        tx.commit()?;
        Ok(outcome)
    }

    /// Rebuilds all holdings from ordered facts, never from purchase metadata.
    pub fn rebuild(&self) -> Result<Ledger> {
        // A read transaction gives all replay queries the same WAL snapshot.
        let tx = self.connection.unchecked_transaction()?;
        let ledger = load(&tx)?;
        tx.commit()?;
        Ok(ledger)
    }

    pub fn sqlite_version(&self) -> &'static str {
        rusqlite::version()
    }
}

pub(crate) fn checksum(sql: &str) -> String {
    format!("{:x}", Sha256::digest(sql.as_bytes()))
}

fn connect(path: &Path) -> Result<Connection> {
    if rusqlite::version_number() < 3_051_003 {
        return Err("SQLite 3.51.3 or later is required for the WAL-reset fix".into());
    }
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    connection.busy_timeout(Duration::from_secs(5))?;
    connection.pragma_update(None, "foreign_keys", true)?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.pragma_update(None, "synchronous", "FULL")?;
    let fk: bool = connection.pragma_query_value(None, "foreign_keys", |r| r.get(0))?;
    let wal: String = connection.pragma_query_value(None, "journal_mode", |r| r.get(0))?;
    let sync: i64 = connection.pragma_query_value(None, "synchronous", |r| r.get(0))?;
    let timeout: i64 = connection.pragma_query_value(None, "busy_timeout", |r| r.get(0))?;
    if !fk || !wal.eq_ignore_ascii_case("wal") || sync != 2 || timeout != 5000 {
        return Err("SQLite connection settings could not be verified".into());
    }
    connection.execute_batch(
        "CREATE VIRTUAL TABLE temp.fts5_probe USING fts5(value); DROP TABLE temp.fts5_probe;",
    )?;
    Ok(connection)
}

pub(crate) fn check_integrity(connection: &Connection) -> Result<()> {
    let status: String = connection.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    let foreign_key_errors: i64 =
        connection.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| {
            r.get(0)
        })?;
    if status != "ok" || foreign_key_errors != 0 {
        return Err("database integrity validation failed".into());
    }
    Ok(())
}

pub(crate) fn load(connection: &Connection) -> Result<Ledger> {
    let mut ledger = Ledger::default();
    let mut statement = connection.prepare(
        "SELECT sequence,id,request_key,actor,reason,event_schema_version,payload_json FROM inventory_operation ORDER BY sequence"
    )?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let sequence = u64::try_from(row.get::<_, i64>(0)?)?;
        let id: String = row.get(1)?;
        let key: String = row.get(2)?;
        let actor: String = row.get(3)?;
        let reason: String = row.get(4)?;
        let version: i64 = row.get(5)?;
        let payload: String = row.get(6)?;
        let op: Operation = serde_json::from_str(&payload)?;
        if version != 1
            || op.id.to_string() != id
            || op.request_key != key
            || op.actor != actor
            || op.reason != reason
            || sequence != u64::try_from(ledger.operations().len())? + 1
        {
            return Err("journal envelope does not match its versioned payload/sequence".into());
        }
        let mut targets: Vec<String> = connection
            .prepare(
                "SELECT holding_id FROM operation_target WHERE operation_id=?1 ORDER BY holding_id",
            )?
            .query_map([&id], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        let previous_count = ledger.operations().len();
        let result = ledger.append(op)?;
        let mut expected: Vec<String> = result.holdings.keys().map(ToString::to_string).collect();
        targets.sort();
        expected.sort();
        if targets != expected {
            return Err("journal targets do not match replayed effects".into());
        }
        if ledger.operations().len() != previous_count + 1 {
            return Err(Box::new(LedgerError::Conflict));
        }
    }
    let stored_ids: Vec<String> = connection
        .prepare("SELECT id FROM stock_holding ORDER BY id")?
        .query_map([], |r| r.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    let mut replay_ids: Vec<String> = ledger.holdings().keys().map(ToString::to_string).collect();
    replay_ids.sort();
    if stored_ids != replay_ids {
        return Err("holding identities do not match replayed history".into());
    }
    if has_purchases(connection)? {
        crate::purchase_store::validate_ownership(connection, &ledger)?;
    }
    if has_verification(connection)? {
        crate::verification_store::validate(connection, &ledger)?;
    }
    if has_stock_actions(connection)? {
        crate::stock_store::validate(connection, &ledger)?;
    }
    Ok(ledger)
}

pub(crate) fn has_purchases(connection: &Connection) -> Result<bool> {
    Ok(connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migration WHERE version=2)",
        [],
        |r| r.get(0),
    )?)
}

pub(crate) fn persist_operation(
    connection: &Connection,
    ledger: &Ledger,
    operation: &Operation,
    outcome: &Outcome,
) -> Result<()> {
    let mut created = Vec::new();
    for id in outcome.holdings.keys() {
        let exists: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM stock_holding WHERE id=?1)",
            [id.to_string()],
            |r| r.get(0),
        )?;
        if !exists {
            connection.execute("INSERT INTO stock_holding(id) VALUES(?1)", [id.to_string()])?;
            created.push(*id);
        }
    }
    connection.execute(
        "INSERT INTO inventory_operation(sequence,id,request_key,actor,reason,event_schema_version,payload_json) VALUES(?1,?2,?3,?4,?5,1,?6)",
        params![i64::try_from(outcome.sequence)?, operation.id.to_string(), operation.request_key,
            operation.actor, operation.reason, serde_json::to_string(operation)?],
    )?;
    for id in outcome.holdings.keys() {
        connection.execute(
            "INSERT INTO operation_target(operation_id,holding_id) VALUES(?1,?2)",
            params![operation.id.to_string(), id.to_string()],
        )?;
    }
    if has_purchases(connection)? {
        for id in &created {
            if let Some(line_id) = ledger.provenance().get(id) {
                let parent = match operation.body {
                    Body::Identify { source_id, .. } => Some(source_id.to_string()),
                    _ => None,
                };
                connection.execute("INSERT INTO stock_provenance(holding_id,invoice_line_id,parent_holding_id) VALUES(?1,?2,?3)", params![id.to_string(), line_id.to_string(), parent])?;
            }
        }
        for copy in outcome.set_copies.values() {
            connection.execute("INSERT INTO owned_set_copy(id,invoice_line_id,receipt_operation_id,snapshot_json) VALUES(?1,?2,?3,?4)",
                params![copy.id.to_string(), copy.invoice_line_id.to_string(), operation.id.to_string(), serde_json::to_string(copy)?])?;
        }
    }
    if has_verification(connection)? {
        crate::verification_store::persist(connection, ledger, operation, &created)?;
    }
    if has_stock_actions(connection)? {
        crate::stock_store::persist(connection, operation)?;
    }
    if has_catalog(connection)? {
        crate::catalog_store::inherit_location(connection, operation)?;
    }
    Ok(())
}

pub(crate) fn has_settings(connection: &Connection) -> Result<bool> {
    Ok(connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migration WHERE version=3)",
        [],
        |r| r.get(0),
    )?)
}
pub(crate) fn metadata_key_exists(connection: &Connection, key: &str) -> Result<bool> {
    Ok(catalog_key_exists(connection, key)?
        || has_settings(connection)?
            && connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM settings_command WHERE request_key=?1)",
                [key],
                |r| r.get(0),
            )?)
}

pub(crate) fn has_catalog(connection: &Connection) -> Result<bool> {
    Ok(connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migration WHERE version=4)",
        [],
        |r| r.get(0),
    )?)
}
pub(crate) fn catalog_key_exists(connection: &Connection, key: &str) -> Result<bool> {
    Ok(finance_key_exists(connection,key)? || has_catalog(connection)? && connection.query_row("SELECT EXISTS(SELECT 1 FROM catalog_command WHERE request_key=?1) OR EXISTS(SELECT 1 FROM profile_command WHERE request_key=?1)",[key],|r|r.get(0))?)
}

pub(crate) fn has_verification(connection: &Connection) -> Result<bool> {
    Ok(connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migration WHERE version=5)",
        [],
        |r| r.get(0),
    )?)
}

pub(crate) fn has_stock_actions(connection: &Connection) -> Result<bool> {
    Ok(connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migration WHERE version=6)",
        [],
        |r| r.get(0),
    )?)
}

pub(crate) fn has_search(connection: &Connection) -> Result<bool> {
    Ok(connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migration WHERE version=7)",
        [],
        |r| r.get(0),
    )?)
}

pub(crate) fn has_finance(connection: &Connection) -> Result<bool> {
    Ok(connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migration WHERE version=8)",
        [],
        |r| r.get(0),
    )?)
}
pub(crate) fn finance_key_exists(connection: &Connection, key: &str) -> Result<bool> {
    Ok(attachment_key_exists(connection, key)?
        || has_finance(connection)?
            && connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM shipping_command WHERE request_key=?1)",
                [key],
                |r| r.get(0),
            )?)
}

pub(crate) fn has_attachments(connection: &Connection) -> Result<bool> {
    Ok(connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migration WHERE version=9)",
        [],
        |r| r.get(0),
    )?)
}
pub(crate) fn attachment_key_exists(connection: &Connection, key: &str) -> Result<bool> {
    Ok(crate::purge::key_exists(connection, key)?
        || has_attachments(connection)?
            && connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM attachment_command WHERE request_key=?1)",
                [key],
                |r| r.get(0),
            )?)
}
