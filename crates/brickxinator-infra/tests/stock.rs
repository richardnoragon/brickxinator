use brickxinator_core::{
    inventory::{
        AssemblyState,
        Measure::{self, Known, Unknown},
    },
    journal::{Action as MeasureAction, MeasureChanges},
    ledger::{Body, Id, Operation},
    purchases::{Draft, InvoiceLine, LineKind, PurchaseAction, PurchaseCommand},
    stock::{Action, Adjustment, Command, Removal},
};
use brickxinator_infra::sqlite::Store;
use rusqlite::Connection;

fn setup(
    kind: LineKind,
    pieces: Measure,
    weight_mg: Measure,
) -> (tempfile::TempDir, std::path::PathBuf, Store, Id) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("stock.sqlite");
    let mut store = Store::create(&path).unwrap();
    let purchase = Id::new();
    store
        .execute_purchase(PurchaseCommand {
            id: Id::new(),
            request_key: "draft".into(),
            actor: "local-user".into(),
            reason: String::new(),
            body: PurchaseAction::Create {
                purchase_id: purchase,
                draft: Draft {
                    lines: vec![InvoiceLine {
                        source_url: None,
                        notes: String::new(),
                        id: Id::new(),
                        kind,
                        description: None,
                        set_number: None,
                        copy_count: None,
                        pieces,
                        weight_mg,
                        paid_total_cents: Some(8000),
                        assembly: AssemblyState::Loose,
                    }],
                    ..Draft::default()
                },
            },
        })
        .unwrap();
    let receipt = store
        .execute_purchase(PurchaseCommand {
            id: Id::new(),
            request_key: "receive".into(),
            actor: "local-user".into(),
            reason: String::new(),
            body: PurchaseAction::Receive {
                purchase_id: purchase,
                expected_revision: 1,
            },
        })
        .unwrap();
    let holding = *receipt.receipt.unwrap().holdings.keys().next().unwrap();
    (dir, path, store, holding)
}
fn command(key: &str, holding_id: Id, expected_revision: u64, action: Action) -> Command {
    Command {
        id: Id::new(),
        request_key: key.into(),
        actor: "local-user".into(),
        reason: "Synthetic stock operation".into(),
        holding_id,
        expected_revision,
        action,
    }
}
fn remove(kind: Removal, pieces: Option<u64>, weight_mg: Option<u64>) -> Action {
    Action::Adjust(Adjustment::Remove {
        kind,
        pieces,
        weight_mg,
        damaged_pieces: 0,
        damaged_weight_mg: 0,
    })
}
fn identify(
    key: &str,
    source: Id,
    revision: u64,
    pieces: Option<u64>,
    weight_mg: Option<u64>,
) -> Operation {
    Operation {
        id: Id::new(),
        request_key: key.into(),
        actor: "local-user".into(),
        reason: "Identify synthetic pieces".into(),
        body: Body::Identify {
            source_id: source,
            child_id: Id::new(),
            expected_revision: revision,
            pieces,
            weight_mg,
        },
    }
}

#[test]
fn broken_and_removals_preserve_ownership_damage_and_purchase_history() {
    let (_dir, path, mut store, holding) = setup(LineKind::LooseBatch, Known(100), Known(1000));
    store
        .execute_stock(command(
            "broken",
            holding,
            1,
            Action::Adjust(Adjustment::Broken {
                pieces: Some(2),
                weight_mg: None,
            }),
        ))
        .unwrap();
    let broken = store.current_inventory().unwrap()[&holding];
    assert_eq!(broken.pieces.owned(), Known(100));
    assert_eq!(broken.pieces.available(broken.assembly), Known(98));
    assert_eq!(broken.weight_mg.damaged(), Unknown);
    store
        .execute_stock(command(
            "donate",
            holding,
            2,
            remove(Removal::Donated, Some(5), None),
        ))
        .unwrap();
    store
        .execute_stock(command(
            "lost",
            holding,
            3,
            remove(Removal::Lost, Some(3), None),
        ))
        .unwrap();
    store
        .execute_stock(command(
            "sold",
            holding,
            4,
            remove(Removal::Sold, Some(2), None),
        ))
        .unwrap();
    drop(store);
    let store = Store::open(path).unwrap();
    let current = store.current_inventory().unwrap()[&holding];
    assert_eq!(current.pieces.owned(), Known(90));
    assert_eq!(current.pieces.damaged(), Known(2));
    assert_eq!(current.pieces.available(current.assembly), Known(88));
    assert_eq!(current.weight_mg.owned(), Unknown);
    let purchase = *store.rebuild().unwrap().receipts().keys().next().unwrap();
    let source = store.purchase(purchase).unwrap().unwrap();
    assert_eq!(source.draft.lines[0].pieces, Known(100));
    assert_eq!(source.draft.lines[0].weight_mg, Known(1000));
    assert_eq!(source.draft.lines[0].paid_total_cents, Some(8000));
    let history = store.holding_history(holding).unwrap();
    assert_eq!(history.len(), 5);
    assert_eq!(history[1].before.unwrap().pieces.damaged(), Known(0));
    assert_eq!(history[1].after.pieces.damaged(), Known(2));
    assert!(
        history
            .iter()
            .all(|r| r.committed_at.ends_with('Z') && !r.reason.is_empty())
    );
}

#[test]
fn damaged_removal_subsets_are_explicit_and_validated() {
    let (_dir, _path, mut store, holding) = setup(LineKind::LooseBatch, Known(10), Unknown);
    store
        .execute_stock(command(
            "broken",
            holding,
            1,
            Action::Adjust(Adjustment::Broken {
                pieces: Some(4),
                weight_mg: None,
            }),
        ))
        .unwrap();
    assert!(
        store
            .execute_stock(command(
                "missing-damage",
                holding,
                2,
                remove(Removal::Sold, Some(8), None)
            ))
            .is_err()
    );
    let action = Action::Adjust(Adjustment::Remove {
        kind: Removal::Donated,
        pieces: Some(8),
        weight_mg: None,
        damaged_pieces: 3,
        damaged_weight_mg: 0,
    });
    store
        .execute_stock(command("explicit-subset", holding, 2, action))
        .unwrap();
    let current = store.current_inventory().unwrap()[&holding];
    assert_eq!(current.pieces.owned(), Known(2));
    assert_eq!(current.pieces.damaged(), Known(1));
    assert!(
        store
            .execute_stock(command(
                "impossible",
                holding,
                3,
                Action::Adjust(Adjustment::Remove {
                    kind: Removal::Lost,
                    pieces: Some(1),
                    weight_mg: None,
                    damaged_pieces: 2,
                    damaged_weight_mg: 0
                })
            ))
            .is_err()
    );
}

#[test]
fn unknown_stock_requires_measurement_before_reduction_and_weight_removal_invalidates_count() {
    let (_dir, _path, mut store, holding) = setup(LineKind::LooseBatch, Unknown, Known(5000));
    assert!(
        store
            .execute_stock(command(
                "unknown-count",
                holding,
                1,
                remove(Removal::Lost, Some(1), None)
            ))
            .is_err()
    );
    store
        .execute_stock(command(
            "weighed",
            holding,
            1,
            remove(Removal::Sold, None, Some(1000)),
        ))
        .unwrap();
    assert_eq!(
        store.current_inventory().unwrap()[&holding]
            .weight_mg
            .owned(),
        Known(4000)
    );
    assert_eq!(
        store.current_inventory().unwrap()[&holding].pieces.owned(),
        Unknown
    );
    store
        .execute_stock(command(
            "measure-count",
            holding,
            2,
            Action::Adjust(Adjustment::Correction {
                pieces: MeasureChanges {
                    owned: MeasureAction::SetMeasured(100),
                    damaged: MeasureAction::Unchanged,
                },
                weight_mg: MeasureChanges::default(),
            }),
        ))
        .unwrap();
    store
        .execute_stock(command(
            "remove-piece",
            holding,
            3,
            remove(Removal::Lost, Some(1), None),
        ))
        .unwrap();
    assert_eq!(
        store.current_inventory().unwrap()[&holding].pieces.owned(),
        Known(99)
    );
    assert_eq!(
        store.current_inventory().unwrap()[&holding]
            .weight_mg
            .owned(),
        Unknown
    );
}

#[test]
fn physical_empty_confirmation_excludes_unknown_residue_without_fabricating_zero() {
    let (_dir, path, mut store, source) = setup(LineKind::MixedLot, Unknown, Known(5_000_000));
    let transfer = identify("identify", source, 1, Some(100), None);
    let child = *store
        .commit(transfer)
        .unwrap()
        .holdings
        .keys()
        .find(|id| **id != source)
        .unwrap();
    assert!(
        store
            .execute_stock(command(
                "not-confirmed",
                source,
                2,
                Action::Complete {
                    confirm_physical_empty: false
                }
            ))
            .is_err()
    );
    let request = command(
        "confirm-empty",
        source,
        2,
        Action::Complete {
            confirm_physical_empty: true,
        },
    );
    let original = store.execute_stock(request.clone()).unwrap();
    let current = store.current_inventory().unwrap();
    assert!(!current.contains_key(&source));
    assert!(current.contains_key(&child));
    assert_eq!(
        store.rebuild().unwrap().holdings()[&source]
            .weight_mg
            .owned(),
        Unknown
    );
    drop(store);
    let mut store = Store::open(path).unwrap();
    assert_eq!(store.execute_stock(request.clone()).unwrap(), original);
    let mut conflict = request;
    conflict.reason = "different".into();
    assert!(store.execute_stock(conflict).is_err());
    assert!(!store.current_inventory().unwrap().contains_key(&source));
    let history = store.holding_history(source).unwrap();
    let closure = history.last().unwrap();
    assert!(closure.classification_after.complete);
    assert!(
        closure
            .classification_after
            .last_completion
            .as_ref()
            .unwrap()
            .physical_empty_confirmed
    );
    assert_eq!(closure.after.weight_mg.owned(), Unknown);
}

#[test]
fn known_nonzero_residue_cannot_complete_even_with_confirmation() {
    let (_dir, _path, mut store, source) = setup(LineKind::MixedLot, Known(100), Unknown);
    for confirmed in [false, true] {
        assert!(
            store
                .execute_stock(command(
                    &format!("complete-{confirmed}"),
                    source,
                    1,
                    Action::Complete {
                        confirm_physical_empty: confirmed
                    }
                ))
                .is_err()
        );
    }
    store
        .commit(identify("all-pieces", source, 1, Some(100), None))
        .unwrap();
    assert!(
        store
            .execute_stock(command(
                "unknown-weight",
                source,
                2,
                Action::Complete {
                    confirm_physical_empty: false
                }
            ))
            .is_err()
    );
    store
        .execute_stock(command(
            "physical-empty",
            source,
            2,
            Action::Complete {
                confirm_physical_empty: true,
            },
        ))
        .unwrap();
    assert!(!store.current_inventory().unwrap().contains_key(&source));
}

#[test]
fn known_zero_completion_is_explicit_and_reopen_never_restores_acquisition_measures() {
    let (_dir, path, mut store, source) = setup(LineKind::MixedLot, Known(100), Known(1000));
    store
        .commit(identify("all-stock", source, 1, Some(100), Some(1000)))
        .unwrap();
    assert!(store.current_inventory().unwrap().contains_key(&source));
    store
        .execute_stock(command(
            "complete",
            source,
            2,
            Action::Complete {
                confirm_physical_empty: false,
            },
        ))
        .unwrap();
    assert!(
        store
            .commit(identify("closed-transfer", source, 3, Some(1), None))
            .is_err()
    );
    assert!(
        store
            .execute_stock(command(
                "closed-correction",
                source,
                3,
                Action::Adjust(Adjustment::Correction {
                    pieces: MeasureChanges {
                        owned: MeasureAction::SetMeasured(5),
                        damaged: MeasureAction::Unchanged
                    },
                    weight_mg: MeasureChanges::default()
                })
            ))
            .is_err()
    );
    store
        .execute_stock(command("reopen", source, 3, Action::Reopen))
        .unwrap();
    assert_eq!(
        store.current_inventory().unwrap()[&source].pieces.owned(),
        Known(0)
    );
    assert_eq!(
        store.current_inventory().unwrap()[&source]
            .weight_mg
            .owned(),
        Known(0)
    );
    assert!(
        store
            .commit(identify("unmeasured-new-find", source, 4, Some(1), None))
            .is_err()
    );
    store
        .execute_stock(command(
            "new-measurement",
            source,
            4,
            Action::Adjust(Adjustment::Correction {
                pieces: MeasureChanges {
                    owned: MeasureAction::SetMeasured(5),
                    damaged: MeasureAction::Unchanged,
                },
                weight_mg: MeasureChanges {
                    owned: MeasureAction::Invalidate,
                    damaged: MeasureAction::Unchanged,
                },
            }),
        ))
        .unwrap();
    drop(store);
    let store = Store::open(path).unwrap();
    let ledger = store.rebuild().unwrap();
    let state = &ledger.classification()[&source];
    assert!(!state.complete);
    assert!(state.last_completion.is_some());
    assert!(state.last_reopen.is_some());
    assert_eq!(ledger.holdings()[&source].pieces.owned(), Known(5));
}

#[test]
fn loose_batches_and_children_cannot_use_lot_lifecycle() {
    let (_dir, _path, mut store, source) = setup(LineKind::LooseBatch, Known(0), Known(0));
    assert!(
        store
            .execute_stock(command(
                "not-lot",
                source,
                1,
                Action::Complete {
                    confirm_physical_empty: false
                }
            ))
            .is_err()
    );
    let (_dir, _path, mut store, source) = setup(LineKind::MixedLot, Known(100), Known(1000));
    let outcome = store
        .commit(identify("identify", source, 1, Some(10), Some(100)))
        .unwrap();
    let child = *outcome.holdings.keys().find(|id| **id != source).unwrap();
    assert!(
        store
            .execute_stock(command(
                "child",
                child,
                1,
                Action::Complete {
                    confirm_physical_empty: true
                }
            ))
            .is_err()
    );
}

#[test]
fn failures_roll_back_adjustments_and_completion_and_reasons_are_required() {
    let (_dir, path, mut store, source) = setup(LineKind::MixedLot, Unknown, Unknown);
    let raw = Connection::open(&path).unwrap();
    raw.execute_batch("CREATE TRIGGER injected_failure BEFORE INSERT ON stock_operation BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
    let completion = command(
        "complete",
        source,
        1,
        Action::Complete {
            confirm_physical_empty: true,
        },
    );
    assert!(store.execute_stock(completion.clone()).is_err());
    assert!(store.current_inventory().unwrap().contains_key(&source));
    assert_eq!(store.rebuild().unwrap().holdings()[&source].revision, 1);
    raw.execute_batch("DROP TRIGGER injected_failure").unwrap();
    store.execute_stock(completion).unwrap();
    let mut reopen = command("reopen", source, 2, Action::Reopen);
    reopen.reason = String::new();
    assert!(store.execute_stock(reopen).is_err());
    store
        .execute_stock(command("reopen-valid", source, 2, Action::Reopen))
        .unwrap();
    let mut correction = command(
        "no-reason",
        source,
        3,
        Action::Adjust(Adjustment::Correction {
            pieces: MeasureChanges {
                owned: MeasureAction::SetMeasured(10),
                damaged: MeasureAction::Unchanged,
            },
            weight_mg: MeasureChanges::default(),
        }),
    );
    correction.reason = String::new();
    assert!(store.execute_stock(correction).is_err());
    assert!(raw.execute("DELETE FROM stock_operation", []).is_err());
}

#[test]
fn concurrent_adjustments_cannot_spend_the_same_revision_twice() {
    let (_dir, path, store, source) = setup(LineKind::LooseBatch, Known(100), Unknown);
    drop(store);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let workers: Vec<_> = (0..2)
        .map(|i| {
            let path = path.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut store = Store::open(path).unwrap();
                barrier.wait();
                store
                    .execute_stock(command(
                        &format!("remove-{i}"),
                        source,
                        1,
                        remove(Removal::Sold, Some(60), None),
                    ))
                    .is_ok()
            })
        })
        .collect();
    assert_eq!(
        workers
            .into_iter()
            .map(|w| w.join().unwrap())
            .filter(|ok| *ok)
            .count(),
        1
    );
    assert_eq!(
        Store::open(path).unwrap().current_inventory().unwrap()[&source]
            .pieces
            .owned(),
        Known(40)
    );
}

#[test]
fn adjustment_retries_are_durable_and_atomic_failures_leave_stock_unchanged() {
    let (_dir, path, mut store, source) = setup(LineKind::LooseBatch, Known(100), Unknown);
    let raw = Connection::open(&path).unwrap();
    raw.execute_batch("CREATE TRIGGER fail_adjustment BEFORE INSERT ON stock_operation BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
    let request = command("sold", source, 1, remove(Removal::Sold, Some(5), None));
    assert!(store.execute_stock(request.clone()).is_err());
    assert_eq!(
        store.current_inventory().unwrap()[&source].pieces.owned(),
        Known(100)
    );
    raw.execute_batch("DROP TRIGGER fail_adjustment").unwrap();
    let original = store.execute_stock(request.clone()).unwrap();
    drop(store);
    let mut store = Store::open(path).unwrap();
    assert_eq!(store.execute_stock(request.clone()).unwrap(), original);
    assert_eq!(
        store.current_inventory().unwrap()[&source].pieces.owned(),
        Known(95)
    );
    let mut conflict = request;
    conflict.reason = "different".into();
    assert!(store.execute_stock(conflict).is_err());
}

#[test]
fn schema_five_remains_usable_without_silently_installing_stock_actions() {
    use sha2::{Digest, Sha256};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("schema-five.sqlite");
    let raw = Connection::open(&path).unwrap();
    for (index, (name, sql)) in [
        (
            "migration_001.sql",
            include_str!("../../../migrations/migration_001.sql"),
        ),
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
    ]
    .into_iter()
    .enumerate()
    {
        raw.execute_batch(sql).unwrap();
        raw.execute(
            "INSERT INTO schema_migration(version,name,checksum) VALUES(?1,?2,?3)",
            rusqlite::params![
                (index + 1) as i64,
                name,
                format!("{:x}", Sha256::digest(sql.as_bytes()))
            ],
        )
        .unwrap();
    }
    drop(raw);
    let mut store = Store::open(&path).unwrap();
    assert!(store.current_inventory().unwrap().is_empty());
    assert!(
        store
            .execute_stock(command(
                "new-action",
                Id::new(),
                1,
                remove(Removal::Lost, Some(1), None)
            ))
            .is_err()
    );
    let raw = Connection::open(path).unwrap();
    assert_eq!(
        raw.query_row("SELECT count(*) FROM schema_migration", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        5
    );
}
