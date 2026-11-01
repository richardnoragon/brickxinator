use brickxinator_core::{
    inventory::{
        AssemblyState,
        Measure::{Known, Unknown},
    },
    ledger::{Body, Id, Operation},
    purchases::{Draft, InvoiceLine, LineKind, PurchaseAction, PurchaseCommand, Status},
};
use brickxinator_infra::sqlite::Store;
use rusqlite::Connection;

fn command(key: &str, body: PurchaseAction) -> PurchaseCommand {
    PurchaseCommand {
        id: Id::new(),
        request_key: key.into(),
        actor: "local-user".into(),
        reason: String::new(),
        body,
    }
}
fn line(kind: LineKind) -> InvoiceLine {
    InvoiceLine {
        source_url: None,
        notes: String::new(),
        id: Id::new(),
        kind,
        description: None,
        set_number: None,
        copy_count: None,
        pieces: Unknown,
        weight_mg: Unknown,
        paid_total_cents: None,
        assembly: AssemblyState::Loose,
    }
}
fn receive(key: &str, id: Id, revision: u64) -> PurchaseCommand {
    command(
        key,
        PurchaseAction::Receive {
            purchase_id: id,
            expected_revision: revision,
        },
    )
}
fn setup() -> (tempfile::TempDir, std::path::PathBuf, Store) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("purchases.sqlite");
    let store = Store::create(&path).unwrap();
    (dir, path, store)
}

#[test]
fn acquisition_edits_do_not_overwrite_an_independent_physical_stock_observation() {
    let (_dir, path, mut store) = setup();
    let id = Id::new();
    let mut batch = line(LineKind::LooseBatch);
    batch.pieces = Known(100);
    store
        .execute_purchase(command(
            "observed-create",
            PurchaseAction::Create {
                purchase_id: id,
                draft: Draft {
                    lines: vec![batch],
                    ..Default::default()
                },
            },
        ))
        .unwrap();
    let receipt = store
        .execute_purchase(receive("observed-receive", id, 1))
        .unwrap();
    let source = *receipt.receipt.unwrap().holdings.keys().next().unwrap();
    store
        .execute_stock(brickxinator_core::stock::Command {
            id: Id::new(),
            request_key: "physical-observation".into(),
            actor: "test".into(),
            reason: "Physically counted 90 remaining pieces".into(),
            holding_id: source,
            expected_revision: 1,
            action: brickxinator_core::stock::Action::Adjust(
                brickxinator_core::stock::Adjustment::Correction {
                    pieces: brickxinator_core::journal::MeasureChanges {
                        owned: brickxinator_core::journal::Action::SetMeasured(90),
                        damaged: brickxinator_core::journal::Action::Unchanged,
                    },
                    weight_mg: Default::default(),
                },
            ),
        })
        .unwrap();
    let mut draft = store.purchase(id).unwrap().unwrap().draft;
    draft.lines[0].pieces = Known(90);
    let preview = store.preview_purchase_correction(id, 2, &draft).unwrap();
    assert!(preview.effects.is_empty());
    assert!(
        preview
            .warnings
            .iter()
            .any(|warning| warning.contains("independent stock observation"))
    );
    let mut correction = command(
        "observed-correct",
        PurchaseAction::Correct {
            purchase_id: id,
            expected_revision: 2,
            draft,
            preview,
        },
    );
    correction.reason = "Correct original invoice transcription; preserve physical count".into();
    store.execute_purchase(correction).unwrap();
    assert_eq!(
        store.rebuild().unwrap().holdings()[&source].pieces.owned(),
        Known(90)
    );
    drop(store);
    Store::open(path).unwrap();
}

#[test]
fn repeated_acquisition_corrections_can_restore_a_known_baseline_without_double_counting() {
    let (_dir, _path, mut store) = setup();
    let id = Id::new();
    let mut lot = line(LineKind::MixedLot);
    lot.pieces = Known(100);
    store
        .execute_purchase(command(
            "baseline-create",
            PurchaseAction::Create {
                purchase_id: id,
                draft: Draft {
                    lines: vec![lot],
                    ..Default::default()
                },
            },
        ))
        .unwrap();
    let receipt = store
        .execute_purchase(receive("baseline-receive", id, 1))
        .unwrap();
    let source = *receipt.receipt.unwrap().holdings.keys().next().unwrap();
    let child = Id::new();
    store
        .commit(Operation {
            id: Id::new(),
            request_key: "baseline-identify".into(),
            actor: "test".into(),
            reason: "Extract 40".into(),
            body: Body::Identify {
                source_id: source,
                child_id: child,
                expected_revision: 1,
                pieces: Some(40),
                weight_mg: None,
            },
        })
        .unwrap();
    for (index, quantity) in [Known(90), Unknown, Known(80)].into_iter().enumerate() {
        let current = store.purchase(id).unwrap().unwrap();
        let mut draft = current.draft;
        draft.lines[0].pieces = quantity;
        let preview = store
            .preview_purchase_correction(id, current.revision, &draft)
            .unwrap();
        let mut correction = command(
            &format!("baseline-correct-{index}"),
            PurchaseAction::Correct {
                purchase_id: id,
                expected_revision: current.revision,
                draft,
                preview,
            },
        );
        correction.reason = "Reconcile acquisition baseline".into();
        store.execute_purchase(correction).unwrap();
    }
    assert_eq!(
        store.rebuild().unwrap().holdings()[&source].pieces.owned(),
        Known(40)
    );
    assert_eq!(
        store.rebuild().unwrap().holdings()[&child].pieces.owned(),
        Known(40)
    );
}

#[test]
fn received_correction_reconciles_only_residual_stock_after_classification() {
    let (_dir, path, mut store) = setup();
    let id = Id::new();
    let mut lot = line(LineKind::MixedLot);
    lot.pieces = Known(100);
    store
        .execute_purchase(command(
            "create-correction",
            PurchaseAction::Create {
                purchase_id: id,
                draft: Draft {
                    lines: vec![lot],
                    ..Default::default()
                },
            },
        ))
        .unwrap();
    let receipt = store
        .execute_purchase(receive("receive-correction", id, 1))
        .unwrap();
    let source = *receipt.receipt.unwrap().holdings.keys().next().unwrap();
    let child = Id::new();
    store
        .commit(Operation {
            id: Id::new(),
            request_key: "identify-correction".into(),
            actor: "test".into(),
            reason: "Classify 40".into(),
            body: Body::Identify {
                source_id: source,
                child_id: child,
                expected_revision: 1,
                pieces: Some(40),
                weight_mg: None,
            },
        })
        .unwrap();
    let mut draft = store.purchase(id).unwrap().unwrap().draft;
    draft.lines[0].pieces = Known(30);
    assert!(store.preview_purchase_correction(id, 2, &draft).is_err());
    draft.lines[0].pieces = Known(90);
    draft.lines[0].paid_total_cents = Some(1200);
    let preview = store.preview_purchase_correction(id, 2, &draft).unwrap();
    assert_eq!(preview.effects.len(), 1);
    assert_eq!(preview.effects[0].before.pieces.owned(), Known(60));
    assert_eq!(preview.effects[0].after.pieces.owned(), Known(50));
    let mut correction = command(
        "correct-source",
        PurchaseAction::Correct {
            purchase_id: id,
            expected_revision: 2,
            draft,
            preview,
        },
    );
    correction.reason = "Correct transcribed invoice count".into();
    let corrected = store.execute_purchase(correction.clone()).unwrap();
    assert_eq!(store.execute_purchase(correction).unwrap(), corrected);
    let ledger = store.rebuild().unwrap();
    assert_eq!(ledger.holdings()[&source].pieces.owned(), Known(50));
    assert_eq!(ledger.holdings()[&child].pieces.owned(), Known(40));
    assert_eq!(ledger.receipts().len(), 1);
    drop(store);
    let store = Store::open(path).unwrap();
    assert_eq!(store.purchase_history(id).unwrap().len(), 3);
    let original = &store.purchase_history(id).unwrap()[1].after.draft.lines[0];
    assert_eq!(original.pieces, Known(100));
}

#[test]
fn unknown_copy_count_discovery_adds_independent_copies_without_receiving_again() {
    let (dir, path, mut store) = setup();
    let id = Id::new();
    let mut set = line(LineKind::Set);
    set.set_number = Some("7838".into());
    set.paid_total_cents = Some(10000);
    store
        .execute_purchase(command(
            "unknown-create",
            PurchaseAction::Create {
                purchase_id: id,
                draft: Draft {
                    shipping_cents: Some(0),
                    lines: vec![set],
                    ..Default::default()
                },
            },
        ))
        .unwrap();
    store
        .execute_purchase(receive("unknown-receive", id, 1))
        .unwrap();
    let mut draft = store.purchase(id).unwrap().unwrap().draft;
    draft.lines[0].copy_count = Some(2);
    let preview = store.preview_purchase_correction(id, 2, &draft).unwrap();
    let copies = preview.copy_additions[0].copies.clone();
    assert_eq!(copies.len(), 2);
    assert_ne!(copies[0].id, copies[1].id);
    let mut correction = command(
        "discover-copies",
        PurchaseAction::Correct {
            purchase_id: id,
            expected_revision: 2,
            draft,
            preview,
        },
    );
    correction.reason = "Count physical copies on arrival".into();
    store.execute_purchase(correction.clone()).unwrap();
    store.execute_purchase(correction).unwrap();
    assert!(store.rebuild().unwrap().holdings().is_empty());
    assert_eq!(store.rebuild().unwrap().set_copies().len(), 2);
    assert_eq!(store.rebuild().unwrap().receipts().len(), 1);
    assert_eq!(
        store.purchase(id).unwrap().unwrap().draft.total_cents(),
        Some(10000)
    );
    let mut draft = store.purchase(id).unwrap().unwrap().draft;
    draft.lines[0].copy_count = Some(1);
    assert!(store.preview_purchase_correction(id, 3, &draft).is_err());
    drop(store);
    Store::open(&path).unwrap();
    brickxinator_infra::snapshots::snapshot(&path, dir.path().join("discovered.zip"), "test", None)
        .unwrap();
}

#[test]
fn stale_or_modified_correction_preview_cannot_change_stock() {
    let (_dir, _path, mut store) = setup();
    let id = Id::new();
    let mut batch = line(LineKind::LooseBatch);
    batch.pieces = Known(10);
    store
        .execute_purchase(command(
            "stale-create",
            PurchaseAction::Create {
                purchase_id: id,
                draft: Draft {
                    lines: vec![batch],
                    ..Default::default()
                },
            },
        ))
        .unwrap();
    let result = store
        .execute_purchase(receive("stale-receive", id, 1))
        .unwrap();
    let holding = *result.receipt.unwrap().holdings.keys().next().unwrap();
    let mut draft = store.purchase(id).unwrap().unwrap().draft;
    draft.lines[0].pieces = Known(12);
    let preview = store.preview_purchase_correction(id, 2, &draft).unwrap();
    store
        .execute_stock(brickxinator_core::stock::Command {
            id: Id::new(),
            request_key: "damage-after-preview".into(),
            actor: "test".into(),
            reason: "Found damage".into(),
            holding_id: holding,
            expected_revision: 1,
            action: brickxinator_core::stock::Action::Adjust(
                brickxinator_core::stock::Adjustment::Broken {
                    pieces: Some(1),
                    weight_mg: None,
                },
            ),
        })
        .unwrap();
    let mut correction = command(
        "stale-correct",
        PurchaseAction::Correct {
            purchase_id: id,
            expected_revision: 2,
            draft,
            preview,
        },
    );
    correction.reason = "Correct invoice".into();
    assert!(store.execute_purchase(correction).is_err());
    assert_eq!(store.purchase(id).unwrap().unwrap().revision, 2);
    assert_eq!(
        store.rebuild().unwrap().holdings()[&holding].pieces.owned(),
        Known(10)
    );
}

#[test]
fn failed_correction_audit_rolls_back_discovered_copies_and_source_metadata() {
    let (_dir, path, mut store) = setup();
    let id = Id::new();
    let set = line(LineKind::Set);
    store
        .execute_purchase(command(
            "rollback-create",
            PurchaseAction::Create {
                purchase_id: id,
                draft: Draft {
                    lines: vec![set],
                    ..Default::default()
                },
            },
        ))
        .unwrap();
    store
        .execute_purchase(receive("rollback-receive", id, 1))
        .unwrap();
    let mut draft = store.purchase(id).unwrap().unwrap().draft;
    draft.lines[0].copy_count = Some(2);
    let preview = store.preview_purchase_correction(id, 2, &draft).unwrap();
    let mut correction = command(
        "rollback-correct",
        PurchaseAction::Correct {
            purchase_id: id,
            expected_revision: 2,
            draft,
            preview,
        },
    );
    correction.reason = "Discover copies".into();
    let raw = Connection::open(&path).unwrap();
    raw.execute_batch("CREATE TRIGGER injected_correction_failure BEFORE INSERT ON purchase_command WHEN NEW.request_key='rollback-correct' BEGIN SELECT RAISE(ABORT,'injected audit failure'); END").unwrap();
    drop(raw);
    assert!(store.execute_purchase(correction).is_err());
    assert!(store.rebuild().unwrap().set_copies().is_empty());
    assert_eq!(
        store.purchase(id).unwrap().unwrap().draft.lines[0].copy_count,
        None
    );
}

#[test]
fn blank_draft_is_valid_and_pre_receipt_states_never_create_stock() {
    let (_dir, _path, mut store) = setup();
    let id = Id::new();
    let created = store
        .execute_purchase(command(
            "blank",
            PurchaseAction::Create {
                purchase_id: id,
                draft: Draft::default(),
            },
        ))
        .unwrap();
    assert_eq!(created.purchase.status, Status::Draft);
    assert!(store.execute_purchase(receive("empty", id, 1)).is_err());
    let mut loose = line(LineKind::LooseBatch);
    loose.pieces = Known(100);
    let draft = Draft {
        lines: vec![loose],
        ..Draft::default()
    };
    for (revision, status) in [
        (1, Status::Ordered),
        (2, Status::Shipped),
        (3, Status::Cancelled),
    ] {
        store
            .execute_purchase(command(
                &format!("save-{revision}"),
                PurchaseAction::Save {
                    purchase_id: id,
                    expected_revision: revision,
                    status,
                    draft: draft.clone(),
                },
            ))
            .unwrap();
        assert!(store.rebuild().unwrap().holdings().is_empty());
    }
    assert!(store.execute_purchase(receive("cancelled", id, 4)).is_err());
    assert_eq!(store.purchase_history(id).unwrap().len(), 4);
}

#[test]
fn whole_purchase_receipt_creates_batches_and_independent_set_copies_once() {
    let (_dir, path, mut store) = setup();
    let id = Id::new();
    let mut sets = line(LineKind::Set);
    sets.copy_count = Some(3);
    sets.set_number = Some("7838".into());
    sets.pieces = Known(1000);
    sets.paid_total_cents = Some(30_000);
    sets.assembly = AssemblyState::PartiallyAssembled;
    let set_line = sets.id;
    let mut loose = line(LineKind::LooseBatch);
    loose.pieces = Known(100);
    loose.paid_total_cents = Some(9300);
    let mut lot = line(LineKind::MixedLot);
    lot.weight_mg = Known(5_000_000);
    lot.paid_total_cents = Some(8000);
    let draft = Draft {
        seller: Some("Synthetic seller".into()),
        shipping_cents: Some(699),
        lines: vec![sets, loose, lot],
        ..Draft::default()
    };
    let total = draft.total_cents();
    let create = command(
        "create",
        PurchaseAction::Create {
            purchase_id: id,
            draft,
        },
    );
    store.execute_purchase(create.clone()).unwrap();
    let receive_request = receive("receive", id, 1);
    let original = store.execute_purchase(receive_request.clone()).unwrap();
    let result = original.receipt.as_ref().unwrap();
    assert_eq!(result.holdings.len(), 2);
    assert_eq!(result.set_copies.len(), 3);
    assert_eq!(original.purchase.draft.total_cents(), total);
    assert_eq!(total, Some(47_999));
    assert_eq!(
        result
            .set_copies
            .values()
            .map(|c| c.label.clone())
            .collect::<Vec<_>>(),
        vec!["7838_01", "7838_02", "7838_03"]
    );
    assert!(
        result
            .set_copies
            .values()
            .all(|c| c.invoice_line_id == set_line
                && c.assembly == AssemblyState::PartiallyAssembled)
    );
    // Box piece count and three physical copies create no detailed parts.
    let ledger = store.rebuild().unwrap();
    let known: u64 = ledger
        .holdings()
        .values()
        .filter_map(|h| {
            if let Known(n) = h.pieces.owned() {
                Some(n)
            } else {
                None
            }
        })
        .sum();
    assert_eq!(known, 100);
    drop(store);
    let mut store = Store::open(path).unwrap();
    assert_eq!(
        store.execute_purchase(receive_request.clone()).unwrap(),
        original
    );
    assert!(
        store
            .execute_purchase(receive("another-receive", id, 2))
            .is_err()
    );
    let mut conflicting = receive_request;
    conflicting.reason = "different payload".into();
    assert!(store.execute_purchase(conflicting).is_err());
    assert_eq!(store.rebuild().unwrap().operations().len(), 1);
    // Retrying draft creation returns the original Draft result, not today's state.
    assert_eq!(
        store.execute_purchase(create).unwrap().purchase.status,
        Status::Draft
    );
    assert_eq!(
        store.purchase(id).unwrap().unwrap().status,
        Status::Received
    );
    assert_eq!(store.purchase_history(id).unwrap().len(), 2);
}

#[test]
fn unknown_set_copy_count_is_received_without_inventing_copies() {
    let (_dir, path, mut store) = setup();
    let id = Id::new();
    let mut sets = line(LineKind::Set);
    sets.pieces = Known(500);
    store
        .execute_purchase(command(
            "create",
            PurchaseAction::Create {
                purchase_id: id,
                draft: Draft {
                    lines: vec![sets],
                    ..Draft::default()
                },
            },
        ))
        .unwrap();
    let result = store.execute_purchase(receive("receive", id, 1)).unwrap();
    assert_eq!(result.purchase.status, Status::Received);
    assert_eq!(result.purchase.draft.lines[0].copy_count, None);
    let ledger = store.rebuild().unwrap();
    assert!(ledger.holdings().is_empty());
    assert!(ledger.set_copies().is_empty());
    assert_eq!(ledger.receipts().len(), 1);
    drop(store);
    assert_eq!(
        Store::open(path)
            .unwrap()
            .purchase(id)
            .unwrap()
            .unwrap()
            .status,
        Status::Received
    );
}

#[test]
fn stale_edits_status_bypass_and_post_receipt_edits_are_rejected() {
    let (_dir, _path, mut store) = setup();
    let id = Id::new();
    let draft = Draft {
        lines: vec![line(LineKind::MixedLot)],
        ..Draft::default()
    };
    store
        .execute_purchase(command(
            "create",
            PurchaseAction::Create {
                purchase_id: id,
                draft: draft.clone(),
            },
        ))
        .unwrap();
    assert!(
        store
            .execute_purchase(command(
                "bypass",
                PurchaseAction::Save {
                    purchase_id: id,
                    expected_revision: 1,
                    status: Status::Received,
                    draft: draft.clone()
                }
            ))
            .is_err()
    );
    let mut correction = draft.clone();
    correction.seller = Some("Corrected seller".into());
    store
        .execute_purchase(command(
            "save",
            PurchaseAction::Save {
                purchase_id: id,
                expected_revision: 1,
                status: Status::Ordered,
                draft: correction.clone(),
            },
        ))
        .unwrap();
    assert!(
        store
            .execute_purchase(command(
                "stale",
                PurchaseAction::Save {
                    purchase_id: id,
                    expected_revision: 1,
                    status: Status::Shipped,
                    draft: draft.clone()
                }
            ))
            .is_err()
    );
    assert!(
        store
            .execute_purchase(receive("stale-receive", id, 1))
            .is_err()
    );
    store.execute_purchase(receive("receive", id, 2)).unwrap();
    assert!(
        store
            .execute_purchase(command(
                "edit-received",
                PurchaseAction::Save {
                    purchase_id: id,
                    expected_revision: 3,
                    status: Status::Cancelled,
                    draft: correction
                }
            ))
            .is_err()
    );
    let history = store.purchase_history(id).unwrap();
    assert_eq!(history.len(), 3);
    assert_eq!(history[1].before.as_ref().unwrap().draft.seller, None);
    assert_eq!(
        history[1].after.draft.seller.as_deref(),
        Some("Corrected seller")
    );
    assert!(
        history
            .iter()
            .all(|h| h.committed_at.ends_with('Z') && h.actor == "local-user")
    );
}

#[test]
fn failed_receipt_rolls_back_purchase_stock_copies_provenance_and_audit() {
    let (_dir, path, mut store) = setup();
    let id = Id::new();
    let mut sets = line(LineKind::Set);
    sets.copy_count = Some(2);
    store
        .execute_purchase(command(
            "create",
            PurchaseAction::Create {
                purchase_id: id,
                draft: Draft {
                    lines: vec![line(LineKind::LooseBatch), sets],
                    ..Draft::default()
                },
            },
        ))
        .unwrap();
    let raw = Connection::open(&path).unwrap();
    raw.execute_batch("CREATE TRIGGER fail_receipt BEFORE INSERT ON purchase_command WHEN (SELECT count(*) FROM receipt)>0 BEGIN SELECT RAISE(ABORT,'injected audit failure'); END;").unwrap();
    let request = receive("receive", id, 1);
    assert!(store.execute_purchase(request.clone()).is_err());
    assert_eq!(store.purchase(id).unwrap().unwrap().status, Status::Draft);
    let ledger = store.rebuild().unwrap();
    assert!(ledger.operations().is_empty());
    assert!(ledger.set_copies().is_empty());
    for table in [
        "inventory_operation",
        "stock_holding",
        "stock_provenance",
        "owned_set_copy",
        "receipt",
    ] {
        let count: i64 = raw
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
    raw.execute_batch("DROP TRIGGER fail_receipt").unwrap();
    store.execute_purchase(request).unwrap();
    assert_eq!(store.purchase_history(id).unwrap().len(), 2);
}

#[test]
fn identified_children_keep_invoice_provenance_and_price_is_not_duplicated() {
    let (_dir, path, mut store) = setup();
    let id = Id::new();
    let mut lot = line(LineKind::MixedLot);
    lot.pieces = Known(3500);
    lot.paid_total_cents = Some(8000);
    let line_id = lot.id;
    store
        .execute_purchase(command(
            "create",
            PurchaseAction::Create {
                purchase_id: id,
                draft: Draft {
                    lines: vec![lot],
                    shipping_cents: Some(0),
                    ..Draft::default()
                },
            },
        ))
        .unwrap();
    let result = store.execute_purchase(receive("receive", id, 1)).unwrap();
    let source = *result.receipt.unwrap().holdings.keys().next().unwrap();
    let child = Id::new();
    store
        .commit(Operation {
            id: Id::new(),
            request_key: "identify".into(),
            actor: "local-user".into(),
            reason: "Identify 2500 pieces".into(),
            body: Body::Identify {
                source_id: source,
                child_id: child,
                expected_revision: 1,
                pieces: Some(2500),
                weight_mg: None,
            },
        })
        .unwrap();
    drop(store);
    let store = Store::open(path).unwrap();
    let ledger = store.rebuild().unwrap();
    assert_eq!(ledger.provenance()[&source], line_id);
    assert_eq!(ledger.provenance()[&child], line_id);
    assert_eq!(ledger.holdings()[&source].pieces.owned(), Known(1000));
    assert_eq!(
        store.purchase(id).unwrap().unwrap().draft.total_cents(),
        Some(8000)
    );
}

#[test]
fn concurrent_receipts_with_different_keys_create_one_receipt() {
    let (_dir, path, mut store) = setup();
    let id = Id::new();
    store
        .execute_purchase(command(
            "create",
            PurchaseAction::Create {
                purchase_id: id,
                draft: Draft {
                    lines: vec![line(LineKind::LooseBatch)],
                    ..Draft::default()
                },
            },
        ))
        .unwrap();
    drop(store);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let workers: Vec<_> = (0..2)
        .map(|i| {
            let barrier = barrier.clone();
            let path = path.clone();
            std::thread::spawn(move || {
                let mut store = Store::open(path).unwrap();
                barrier.wait();
                store
                    .execute_purchase(receive(&format!("receive-{i}"), id, 1))
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
    let ledger = Store::open(path).unwrap().rebuild().unwrap();
    assert_eq!(ledger.operations().len(), 1);
    assert_eq!(ledger.holdings().len(), 1);
}

#[test]
fn malformed_numbers_duplicate_lines_and_missing_costs_are_distinct() {
    let mut part = line(LineKind::LooseBatch);
    part.paid_total_cents = Some(9300);
    let mut draft = Draft {
        lines: vec![part.clone()],
        shipping_cents: Some(699),
        ..Draft::default()
    };
    assert_eq!(draft.total_cents(), Some(9999));
    draft.shipping_cents = None;
    assert_eq!(draft.total_cents(), None);
    draft.shipping_cents = Some(0);
    assert_eq!(draft.total_cents(), Some(9300));
    let json = serde_json::to_string(&draft).unwrap().replace("9300", "-1");
    assert!(serde_json::from_str::<Draft>(&json).is_err());
    draft.lines.push(part);
    assert!(draft.validate().is_err());
    let (_dir, _path, mut store) = setup();
    assert!(
        store
            .execute_purchase(command(
                "invalid",
                PurchaseAction::Create {
                    purchase_id: Id::new(),
                    draft
                }
            ))
            .is_err()
    );
    assert!(store.rebuild().unwrap().operations().is_empty());
}

#[test]
fn receipt_payload_cannot_bypass_purchase_commands() {
    let (_dir, _path, mut store) = setup();
    let operation = Operation {
        id: Id::new(),
        request_key: "bypass".into(),
        actor: "local-user".into(),
        reason: "Forged receipt".into(),
        body: Body::Receive {
            purchase_id: Id::new(),
            lines: vec![],
        },
    };
    assert!(store.commit(operation).is_err());
    assert!(store.rebuild().unwrap().operations().is_empty());
}

#[test]
fn schema_one_remains_usable_without_silent_upgrade() {
    use sha2::{Digest, Sha256};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.sqlite");
    let raw = Connection::open(&path).unwrap();
    let sql = include_str!("../../../migrations/migration_001.sql");
    raw.execute_batch(sql).unwrap();
    let hash = format!("{:x}", Sha256::digest(sql.as_bytes()));
    raw.execute(
        "INSERT INTO schema_migration(version,name,checksum) VALUES(1,'migration_001.sql',?1)",
        [hash],
    )
    .unwrap();
    drop(raw);
    let mut store = Store::open(&path).unwrap();
    let holding = Id::new();
    store
        .commit(Operation {
            id: Id::new(),
            request_key: "legacy-observation".into(),
            actor: "local-user".into(),
            reason: "Synthetic observation".into(),
            body: Body::Create {
                holding_id: holding,
                pieces: Known(10),
                weight_mg: Unknown,
            },
        })
        .unwrap();
    assert!(
        store
            .execute_purchase(command(
                "purchase",
                PurchaseAction::Create {
                    purchase_id: Id::new(),
                    draft: Draft::default()
                }
            ))
            .is_err()
    );
    drop(store);
    assert_eq!(
        Store::open(&path).unwrap().rebuild().unwrap().holdings()[&holding]
            .pieces
            .owned(),
        Known(10)
    );
    let raw = Connection::open(path).unwrap();
    let versions: i64 = raw
        .query_row("SELECT count(*) FROM schema_migration", [], |r| r.get(0))
        .unwrap();
    assert_eq!(versions, 1);
}

#[test]
fn purchased_loose_batches_cannot_be_split_as_mixed_lots() {
    let (_dir, _path, mut store) = setup();
    let id = Id::new();
    store
        .execute_purchase(command(
            "create",
            PurchaseAction::Create {
                purchase_id: id,
                draft: Draft {
                    lines: vec![line(LineKind::LooseBatch)],
                    ..Draft::default()
                },
            },
        ))
        .unwrap();
    let result = store.execute_purchase(receive("receive", id, 1)).unwrap();
    let source = *result.receipt.unwrap().holdings.keys().next().unwrap();
    let splitting = Operation {
        id: Id::new(),
        request_key: "split".into(),
        actor: "local-user".into(),
        reason: "Try to split".into(),
        body: Body::Identify {
            source_id: source,
            child_id: Id::new(),
            expected_revision: 1,
            pieces: Some(10),
            weight_mg: None,
        },
    };
    assert!(store.commit(splitting).is_err());
    assert_eq!(store.rebuild().unwrap().holdings().len(), 1);
}
