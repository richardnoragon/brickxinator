use brickxinator_core::{
    inventory::Measure::{self, Known, Unknown},
    journal::{Action, MeasureChanges},
    ledger::{Body, Id, Ledger, Operation},
};
use brickxinator_infra::sqlite::Store;
use rusqlite::{Connection, params};

fn operation(key: &str, body: Body) -> Operation {
    Operation {
        id: Id::new(),
        request_key: key.into(),
        actor: "local-user".into(),
        reason: "Synthetic inventory test".into(),
        body,
    }
}

fn create(key: &str, id: Id, pieces: Measure, weight_mg: Measure) -> Operation {
    operation(
        key,
        Body::Create {
            holding_id: id,
            pieces,
            weight_mg,
        },
    )
}

fn transfer(
    key: &str,
    source_id: Id,
    child_id: Id,
    revision: u64,
    pieces: Option<u64>,
    weight_mg: Option<u64>,
) -> Operation {
    operation(
        key,
        Body::Identify {
            source_id,
            child_id,
            expected_revision: revision,
            pieces,
            weight_mg,
        },
    )
}

#[test]
fn persisted_transfer_conserves_stock_and_retries_survive_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inventory.sqlite");
    let source = Id::new();
    let child = Id::new();
    let receipt = create("observe", source, Known(3500), Known(5_000_000));
    let identify = transfer("identify", source, child, 1, Some(2500), Some(3_000_000));
    let expected;
    {
        let mut store = Store::create(&path).unwrap();
        println!("tested SQLite {}", store.sqlite_version());
        store.commit(receipt.clone()).unwrap();
        expected = store.commit(identify.clone()).unwrap();
    }
    let mut store = Store::open(&path).unwrap();
    assert_eq!(store.commit(identify.clone()).unwrap(), expected);
    let rebuilt = store.rebuild().unwrap();
    assert_eq!(rebuilt.operations().len(), 2);
    assert_eq!(rebuilt.holdings()[&source].pieces.owned(), Known(1000));
    assert_eq!(rebuilt.holdings()[&child].pieces.owned(), Known(2500));
    assert_eq!(
        rebuilt.holdings()[&source].weight_mg.owned(),
        Known(2_000_000)
    );
    assert_eq!(
        rebuilt.holdings()[&child].weight_mg.owned(),
        Known(3_000_000)
    );
    let replay = Ledger::replay(rebuilt.operations().iter().cloned()).unwrap();
    assert_eq!(replay.holdings(), rebuilt.holdings());
    let original = store.commit(receipt).unwrap();
    assert_eq!(original.sequence, 1);
    assert_eq!(original.holdings[&source].pieces.owned(), Known(3500));
    let mut conflicting = identify;
    conflicting.reason = "Changed payload".into();
    assert!(store.commit(conflicting).is_err());
}

#[test]
fn unknown_residual_weight_does_not_restore_acquisition_measurement() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inventory.sqlite");
    let source = Id::new();
    let child = Id::new();
    let mut store = Store::create(&path).unwrap();
    store
        .commit(create("receive", source, Unknown, Known(5_000_000)))
        .unwrap();
    store
        .commit(transfer("extract", source, child, 1, Some(100), None))
        .unwrap();
    drop(store);
    let ledger = Store::open(path).unwrap().rebuild().unwrap();
    assert_eq!(ledger.holdings()[&source].weight_mg.owned(), Unknown);
    assert_eq!(ledger.holdings()[&source].pieces.owned(), Unknown);
    assert_eq!(ledger.holdings()[&child].pieces.owned(), Known(100));
    assert!(matches!(
        ledger.operations()[0].body,
        Body::Create {
            weight_mg: Known(5_000_000),
            ..
        }
    ));
}

#[test]
fn rejected_transfers_leave_no_children_or_journal_entries() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inventory.sqlite");
    let source = Id::new();
    let mut store = Store::create(&path).unwrap();
    store
        .commit(create("receive", source, Known(50), Known(500)))
        .unwrap();
    assert!(
        store
            .commit(transfer(
                "too-many",
                source,
                Id::new(),
                1,
                Some(51),
                Some(10)
            ))
            .is_err()
    );
    assert!(
        store
            .commit(transfer(
                "too-heavy",
                source,
                Id::new(),
                1,
                Some(10),
                Some(501)
            ))
            .is_err()
    );
    assert!(
        store
            .commit(transfer("empty", source, Id::new(), 1, Some(0), None))
            .is_err()
    );
    let ledger = store.rebuild().unwrap();
    assert_eq!(ledger.operations().len(), 1);
    assert_eq!(ledger.holdings().len(), 1);
    assert_eq!(ledger.holdings()[&source].pieces.owned(), Known(50));
}

#[test]
fn database_failure_rolls_back_all_operation_rows() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inventory.sqlite");
    let source = Id::new();
    let child = Id::new();
    let mut store = Store::create(&path).unwrap();
    store
        .commit(create("receive", source, Known(100), Known(1000)))
        .unwrap();
    let raw = Connection::open(&path).unwrap();
    raw.execute_batch("CREATE TRIGGER injected_failure BEFORE INSERT ON operation_target WHEN (SELECT count(*) FROM operation_target) >= 2 BEGIN SELECT RAISE(ABORT, 'injected disk/write failure'); END;").unwrap();
    let extraction = transfer("extract", source, child, 1, Some(10), Some(100));
    assert!(store.commit(extraction.clone()).is_err());
    let ledger = store.rebuild().unwrap();
    assert_eq!(ledger.operations().len(), 1);
    assert_eq!(ledger.holdings().len(), 1);
    raw.execute_batch("DROP TRIGGER injected_failure").unwrap();
    assert_eq!(store.commit(extraction).unwrap().sequence, 2);
    assert_eq!(
        store.rebuild().unwrap().holdings()[&source].pieces.owned(),
        Known(90)
    );
}

#[test]
fn separate_connections_reject_stale_edits_and_use_foreign_keys() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inventory.sqlite");
    let source = Id::new();
    let mut first = Store::create(&path).unwrap();
    first
        .commit(create("receive", source, Known(100), Known(1000)))
        .unwrap();
    let mut second = Store::open(&path).unwrap();
    first
        .commit(transfer("first", source, Id::new(), 1, Some(10), Some(100)))
        .unwrap();
    assert!(
        second
            .commit(transfer("stale", source, Id::new(), 1, Some(10), Some(100)))
            .is_err()
    );
    assert_eq!(
        second.rebuild().unwrap().holdings()[&source].pieces.owned(),
        Known(90)
    );
}

#[test]
fn immutable_history_checksum_and_target_validation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inventory.sqlite");
    let mut store = Store::create(&path).unwrap();
    store
        .commit(create("receive", Id::new(), Known(100), Unknown))
        .unwrap();
    let raw = Connection::open(&path).unwrap();
    assert!(
        raw.execute("UPDATE inventory_operation SET reason='rewrite'", [])
            .is_err()
    );
    assert!(raw.execute("DELETE FROM inventory_operation", []).is_err());
    let committed: String = raw
        .query_row("SELECT committed_at FROM inventory_operation", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(committed.ends_with('Z'));
    raw.execute(
        "UPDATE schema_migration SET checksum='changed' WHERE version=1",
        [],
    )
    .unwrap();
    assert!(Store::open(&path).is_err());
    // Restore the original hash to demonstrate payload/target consistency checks.
    use sha2::{Digest, Sha256};
    let checksum = format!(
        "{:x}",
        Sha256::digest(include_str!("../../../migrations/migration_001.sql").as_bytes())
    );
    raw.execute(
        "UPDATE schema_migration SET checksum=?1 WHERE version=1",
        [checksum],
    )
    .unwrap();
    raw.execute_batch("DROP TRIGGER target_no_delete; DELETE FROM operation_target;")
        .unwrap();
    assert!(Store::open(&path).is_err());
}

#[test]
fn uuid_json_validation_and_damage_guards() {
    let source = Id::new();
    let op = create("create", source, Known(100), Known(1000));
    let json = serde_json::to_string(&op).unwrap();
    assert_eq!(serde_json::from_str::<Operation>(&json).unwrap(), op);
    let invalid = json.replace(&source.to_string(), "00000000-0000-4000-8000-000000000000");
    assert!(serde_json::from_str::<Operation>(&invalid).is_err());
    let mut ledger = Ledger::default();
    ledger.append(op).unwrap();
    ledger
        .append(operation(
            "damage",
            Body::Change {
                holding_id: source,
                expected_revision: 1,
                pieces: MeasureChanges {
                    owned: Action::Unchanged,
                    damaged: Action::SetMeasured(2),
                },
                weight_mg: MeasureChanges::default(),
                assembly: None,
            },
        ))
        .unwrap();
    assert!(
        ledger
            .append(transfer(
                "extract",
                source,
                Id::new(),
                2,
                Some(10),
                Some(100)
            ))
            .is_err()
    );
    assert_eq!(ledger.holdings()[&source].pieces.owned(), Known(100));
}

#[test]
fn create_never_overwrites_and_open_never_creates_missing_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inventory.sqlite");
    assert!(Store::open(&path).is_err());
    assert!(!path.exists());
    let mut store = Store::create(&path).unwrap();
    store
        .commit(create("receive", Id::new(), Known(1), Unknown))
        .unwrap();
    assert!(Store::create(&path).is_err());
    assert_eq!(store.rebuild().unwrap().holdings().len(), 1);
    let raw = Connection::open(path).unwrap();
    let count: i64 = raw
        .query_row("SELECT count(*) FROM inventory_operation", params![], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn concurrent_writers_cannot_spend_the_same_revision_twice() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inventory.sqlite");
    let source = Id::new();
    Store::create(&path)
        .unwrap()
        .commit(create("receive", source, Known(100), Known(1000)))
        .unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let workers: Vec<_> = (0..2)
        .map(|index| {
            let path = path.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut store = Store::open(path).unwrap();
                barrier.wait();
                store
                    .commit(transfer(
                        &format!("writer-{index}"),
                        source,
                        Id::new(),
                        1,
                        Some(60),
                        Some(600),
                    ))
                    .is_ok()
            })
        })
        .collect();
    let successes = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .filter(|success| *success)
        .count();
    assert_eq!(successes, 1);
    let ledger = Store::open(path).unwrap().rebuild().unwrap();
    assert_eq!(ledger.operations().len(), 2);
    assert_eq!(ledger.holdings().len(), 2);
    assert_eq!(ledger.holdings()[&source].pieces.owned(), Known(40));
}

#[test]
fn deserialization_cannot_bypass_balance_validation() {
    use brickxinator_core::inventory::Balance;
    assert!(
        serde_json::from_str::<Balance>(r#"{"owned":{"Known":1},"damaged":{"Known":2}}"#).is_err()
    );
    let valid = Balance::new(Known(100), Known(2)).unwrap();
    let encoded = serde_json::to_string(&valid).unwrap();
    assert_eq!(serde_json::from_str::<Balance>(&encoded).unwrap(), valid);
}
