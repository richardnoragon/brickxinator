use brickxinator_core::{
    inventory::{
        AssemblyState,
        Measure::{Known, Unknown},
    },
    journal::{Action as MeasureAction, MeasureChanges},
    ledger::{Body, Id, Operation},
    profiles::{self, Target},
    purchases::{Draft, InvoiceLine, LineKind, PurchaseAction, PurchaseCommand},
    settings::{self, Kind, Vocabulary},
    verification::{Action, Command, Difference},
};
use brickxinator_infra::sqlite::Store;
use rusqlite::Connection;

fn setup(assembly: AssemblyState) -> (tempfile::TempDir, std::path::PathBuf, Store, Id) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("verification.sqlite");
    let mut store = Store::create(&path).unwrap();
    let purchase = Id::new();
    store
        .execute_purchase(PurchaseCommand {
            id: Id::new(),
            request_key: "create".into(),
            actor: "local-user".into(),
            reason: String::new(),
            body: PurchaseAction::Create {
                purchase_id: purchase,
                draft: Draft {
                    lines: vec![InvoiceLine {
                        source_url: None,
                        notes: String::new(),
                        id: Id::new(),
                        kind: LineKind::Set,
                        description: None,
                        set_number: Some("7838".into()),
                        copy_count: Some(1),
                        pieces: Known(451),
                        weight_mg: Unknown,
                        paid_total_cents: Some(30000),
                        assembly,
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
    let copy = *receipt.receipt.unwrap().set_copies.keys().next().unwrap();
    (dir, path, store, copy)
}
fn command(key: &str, copy_id: Id, expected_revision: u64, action: Action) -> Command {
    Command {
        id: Id::new(),
        request_key: key.into(),
        actor: "local-user".into(),
        reason: "Synthetic verification observation".into(),
        copy_id,
        expected_revision,
        action,
    }
}

#[test]
fn reusable_definition_checklist_copies_are_independent_and_never_owned_until_counted() {
    use brickxinator_core::catalog;
    let (_dir, _path, mut store, copy) = setup(AssemblyState::Loose);
    let manufacturer = store
        .settings()
        .unwrap()
        .values()
        .find(|record| {
            matches!(
                record.kind,
                Kind::Vocabulary {
                    vocabulary: Vocabulary::Manufacturer,
                    system_unknown: true
                }
            )
        })
        .unwrap()
        .id;
    let color = store.default_inventory_links().unwrap().color_id;
    let definition = Id::new();
    store
        .execute_catalog(catalog::Command {
            id: Id::new(),
            request_key: "template".into(),
            actor: "test".into(),
            reason: String::new(),
            action: catalog::Action::Create {
                id: definition,
                kind: catalog::Kind::Set,
                details: catalog::Details {
                    manufacturer_id: manufacturer,
                    number: Some("7838".into()),
                    name: None,
                    description: None,
                    release_year: None,
                    theme: None,
                    stated_piece_count: Some(451),
                    expected_contents: vec![
                        catalog::TemplateLine {
                            definition_id: None,
                            color_id: color,
                            expected: Some(7),
                        },
                        catalog::TemplateLine {
                            definition_id: None,
                            color_id: color,
                            expected: None,
                        },
                    ],
                },
            },
        })
        .unwrap();
    let first = store.prepare_definition_expected(definition).unwrap();
    let second = store.prepare_definition_expected(definition).unwrap();
    assert_ne!(first[0].line_id, second[0].line_id);
    assert_ne!(first[0].holding_id, second[0].holding_id);
    store
        .execute_verification(command(
            "from-template",
            copy,
            0,
            Action::AddExpected {
                lines: first.clone(),
            },
        ))
        .unwrap();
    assert!(store.rebuild().unwrap().holdings().is_empty());
    assert_eq!(
        store.set_verification(copy).unwrap().lines[&first[1].line_id].expected,
        None
    );
    store
        .execute_verification(command(
            "template-count",
            copy,
            1,
            Action::Count {
                line_id: first[0].line_id,
                total: 7,
            },
        ))
        .unwrap();
    assert_eq!(store.rebuild().unwrap().holdings().len(), 1);
    assert_eq!(
        store.catalog().unwrap()[&definition]
            .details
            .expected_contents[0]
            .expected,
        Some(7)
    );
    assert!(second.iter().all(|line| {
        !store
            .rebuild()
            .unwrap()
            .holdings()
            .contains_key(&line.holding_id)
    }));
}

#[test]
fn copy_rename_is_audited_and_does_not_change_receipt_or_stock() {
    let (_dir, path, mut store, copy) = setup(AssemblyState::Assembled);
    let before = store.rebuild().unwrap();
    let receipt_label = before.set_copies()[&copy].label.clone();
    let rename = command(
        "rename-copy",
        copy,
        0,
        Action::Rename {
            label: "Display case locomotive".into(),
        },
    );
    store.execute_verification(rename.clone()).unwrap();
    store.execute_verification(rename).unwrap();
    let after = store.rebuild().unwrap();
    assert_eq!(after.copy_label(copy), Some("Display case locomotive"));
    assert_eq!(after.set_copies()[&copy].label, receipt_label);
    assert_eq!(before.holdings(), after.holdings());
    assert_eq!(after.assembly(copy), Some(AssemblyState::Assembled));
    assert_eq!(store.verification_history(copy).unwrap().len(), 1);
    drop(store);
    assert_eq!(
        Store::open(path)
            .unwrap()
            .rebuild()
            .unwrap()
            .copy_label(copy),
        Some("Display case locomotive")
    );
}
fn add(store: &mut Store, copy: Id, expected: Option<u64>) -> (Id, Id) {
    let color = store
        .settings()
        .unwrap()
        .values()
        .find(|r| {
            matches!(
                r.kind,
                Kind::Vocabulary {
                    vocabulary: Vocabulary::Color,
                    system_unknown: true
                }
            )
        })
        .unwrap()
        .id;
    let line = Id::new();
    let holding = Id::new();
    let revision = store.set_verification(copy).unwrap().revision;
    store
        .execute_verification(command(
            &format!("add-{line}"),
            copy,
            revision,
            Action::AddLine {
                line_id: line,
                holding_id: holding,
                definition_id: None,
                color_id: color,
                expected,
            },
        ))
        .unwrap();
    (line, holding)
}
fn count(store: &mut Store, copy: Id, line: Id, total: u64) {
    let revision = store.set_verification(copy).unwrap().revision;
    store
        .execute_verification(command(
            &format!("count-{copy}-{revision}"),
            copy,
            revision,
            Action::Count {
                line_id: line,
                total,
            },
        ))
        .unwrap();
}

#[test]
fn checklist_identity_and_retirement_preserve_owned_stock_and_audited_initial_refs() {
    let (_dir, path, mut store, copy) = setup(AssemblyState::Loose);
    let (line, holding) = add(&mut store, copy, Some(7));
    count(&mut store, copy, line, 7);
    let red = Id::new();
    store
        .execute_settings(settings::Command {
            id: Id::new(),
            request_key: "red-checklist".into(),
            actor: "test".into(),
            reason: String::new(),
            accept_name_warnings: false,
            action: settings::Action::CreateVocabulary {
                id: red,
                vocabulary: Vocabulary::Color,
                name: "Red".into(),
            },
        })
        .unwrap();
    let before = store.rebuild().unwrap().holdings()[&holding];
    for (key, action) in [
        (
            "identity",
            Action::Identity {
                line_id: line,
                definition_id: None,
                color_id: red,
            },
        ),
        (
            "retire",
            Action::Active {
                line_id: line,
                active: false,
            },
        ),
    ] {
        let revision = store.set_verification(copy).unwrap().revision;
        store
            .execute_verification(command(key, copy, revision, action))
            .unwrap();
    }
    assert_eq!(
        store.set_verification(copy).unwrap().lines[&line].color_id,
        red
    );
    assert!(store.set_verification(copy).unwrap().lines[&line].retired);
    assert_eq!(store.rebuild().unwrap().holdings()[&holding], before);
    assert!(store.prepare_expected_copy(copy).unwrap().is_empty());
    let revision = store.set_verification(copy).unwrap().revision;
    assert!(
        store
            .execute_verification(command(
                "retired-count",
                copy,
                revision,
                Action::Count {
                    line_id: line,
                    total: 0
                }
            ))
            .is_err()
    );
    store
        .execute_verification(command(
            "assemble-retired",
            copy,
            revision,
            Action::Assembly {
                state: AssemblyState::Assembled,
            },
        ))
        .unwrap();
    assert_eq!(
        store.set_contents(copy).unwrap()[0]
            .holding
            .pieces
            .available(AssemblyState::Assembled),
        Known(0)
    );
    let revision = store.set_verification(copy).unwrap().revision;
    store
        .execute_verification(command(
            "restore-line",
            copy,
            revision,
            Action::Active {
                line_id: line,
                active: true,
            },
        ))
        .unwrap();
    drop(store);
    let store = Store::open(path).unwrap();
    assert!(!store.set_verification(copy).unwrap().lines[&line].retired);
    assert_eq!(store.set_contents(copy).unwrap()[0].links.color_id, red);
}

#[test]
fn expected_contents_create_no_stock_and_counts_are_absolute_after_restart() {
    let (_dir, path, mut store, copy) = setup(AssemblyState::Loose);
    let (line, holding) = add(&mut store, copy, Some(10));
    assert!(store.rebuild().unwrap().holdings().is_empty());
    assert!(store.set_contents(copy).unwrap().is_empty());
    let request = command(
        "count-seven",
        copy,
        1,
        Action::Count {
            line_id: line,
            total: 7,
        },
    );
    let original = store.execute_verification(request.clone()).unwrap();
    count(&mut store, copy, line, 7);
    count(&mut store, copy, line, 9);
    assert_eq!(
        store.rebuild().unwrap().holdings()[&holding].pieces.owned(),
        Known(9)
    );
    drop(store);
    let mut store = Store::open(path).unwrap();
    assert_eq!(
        store.execute_verification(request.clone()).unwrap(),
        original
    );
    assert_eq!(
        store.rebuild().unwrap().holdings()[&holding].pieces.owned(),
        Known(9)
    );
    let mut conflict = request;
    conflict.reason = "different payload".into();
    assert!(store.execute_verification(conflict).is_err());
    assert_eq!(
        store.set_verification(copy).unwrap().lines[&line].verified,
        Some(9)
    );
    let history = store.verification_history(copy).unwrap();
    assert_eq!(history.len(), 4);
    assert_eq!(history[3].before.lines[&line].verified, Some(7));
    assert_eq!(history[3].after.lines[&line].verified, Some(9));
    assert!(
        history.iter().all(|r| r.committed_at.ends_with('Z')
            && r.actor == "local-user"
            && !r.reason.is_empty())
    );
}

#[test]
fn completion_is_explicit_and_count_or_expected_edits_reopen_it() {
    let (_dir, _path, mut store, copy) = setup(AssemblyState::Loose);
    let (line, holding) = add(&mut store, copy, Some(10));
    count(&mut store, copy, line, 7);
    assert_eq!(
        store.set_verification(copy).unwrap().lines[&line].difference(),
        Difference::Unfinished
    );
    assert!(
        store
            .execute_verification(command("too-soon", copy, 2, Action::Complete))
            .is_err()
    );
    store
        .execute_verification(command("finish", copy, 2, Action::Finish { line_id: line }))
        .unwrap();
    let state = store.set_verification(copy).unwrap();
    assert!(!state.completed);
    assert_eq!(state.lines[&line].difference(), Difference::Short(3));
    store
        .execute_verification(command("complete", copy, 3, Action::Complete))
        .unwrap();
    assert!(store.set_verification(copy).unwrap().completed);
    store
        .execute_verification(command(
            "expected",
            copy,
            4,
            Action::Expected {
                line_id: line,
                expected: Some(12),
            },
        ))
        .unwrap();
    let state = store.set_verification(copy).unwrap();
    assert!(!state.completed);
    assert!(!state.lines[&line].finished);
    assert_eq!(
        store.rebuild().unwrap().holdings()[&holding].pieces.owned(),
        Known(7)
    );
    count(&mut store, copy, line, 14);
    store
        .execute_verification(command(
            "finish-excess",
            copy,
            6,
            Action::Finish { line_id: line },
        ))
        .unwrap();
    assert_eq!(
        store.set_verification(copy).unwrap().lines[&line].difference(),
        Difference::Excess(2)
    );
}

#[test]
fn assembled_unverified_sets_accept_counts_and_damage_survives_disassembly() {
    let (_dir, path, mut store, copy) = setup(AssemblyState::Assembled);
    let (line, holding) = add(&mut store, copy, Some(100));
    count(&mut store, copy, line, 100);
    let current = store.rebuild().unwrap().holdings()[&holding];
    assert_eq!(current.assembly, AssemblyState::Assembled);
    assert_eq!(current.pieces.available(current.assembly), Known(0));
    store
        .commit(Operation {
            id: Id::new(),
            request_key: "damage".into(),
            actor: "local-user".into(),
            reason: "Two broken pieces".into(),
            body: Body::Change {
                holding_id: holding,
                expected_revision: current.revision,
                pieces: MeasureChanges {
                    owned: MeasureAction::Unchanged,
                    damaged: MeasureAction::SetMeasured(2),
                },
                weight_mg: MeasureChanges::default(),
                assembly: None,
            },
        })
        .unwrap();
    for (revision, state) in [
        (2, AssemblyState::PartiallyAssembled),
        (3, AssemblyState::Loose),
    ] {
        store
            .execute_verification(command(
                &format!("assembly-{revision}"),
                copy,
                revision,
                Action::Assembly { state },
            ))
            .unwrap();
    }
    drop(store);
    let store = Store::open(path).unwrap();
    let actual = store.set_contents(copy).unwrap()[0].holding;
    assert_eq!(actual.pieces.owned(), Known(100));
    assert_eq!(actual.pieces.damaged(), Known(2));
    assert_eq!(actual.pieces.available(actual.assembly), Known(98));
    assert!(!store.set_verification(copy).unwrap().completed);
}

#[test]
fn verified_contents_inherit_copy_location_and_cannot_move_independently() {
    let (_dir, path, mut store, copy) = setup(AssemblyState::Loose);
    let drawer = Id::new();
    let shelf = Id::new();
    for (key, id, name) in [("drawer", drawer, "Drawer"), ("shelf", shelf, "Shelf")] {
        store
            .execute_settings(settings::Command {
                id: Id::new(),
                request_key: key.into(),
                actor: "local-user".into(),
                reason: String::new(),
                accept_name_warnings: false,
                action: settings::Action::CreateLocation {
                    id,
                    parent_id: None,
                    name: name.into(),
                },
            })
            .unwrap();
    }
    let mut links = store.default_inventory_links().unwrap();
    links.location_id = Some(drawer);
    store
        .execute_profile(profiles::Command {
            id: Id::new(),
            request_key: "copy-location".into(),
            actor: "local-user".into(),
            reason: String::new(),
            target: Target::SetCopy(copy),
            expected_revision: 0,
            links: links.clone(),
        })
        .unwrap();
    let (line, holding) = add(&mut store, copy, Some(10));
    count(&mut store, copy, line, 7);
    assert_eq!(
        store.set_contents(copy).unwrap()[0].links.location_id,
        Some(drawer)
    );
    links.location_id = Some(shelf);
    assert!(
        store
            .execute_profile(profiles::Command {
                id: Id::new(),
                request_key: "part-move".into(),
                actor: "local-user".into(),
                reason: String::new(),
                target: Target::Holding(holding),
                expected_revision: 0,
                links: links.clone()
            })
            .is_err()
    );
    store
        .execute_profile(profiles::Command {
            id: Id::new(),
            request_key: "copy-move".into(),
            actor: "local-user".into(),
            reason: String::new(),
            target: Target::SetCopy(copy),
            expected_revision: 1,
            links,
        })
        .unwrap();
    drop(store);
    let store = Store::open(path).unwrap();
    assert_eq!(
        store.set_contents(copy).unwrap()[0].links.location_id,
        Some(shelf)
    );
    assert_eq!(
        store.set_contents(copy).unwrap()[0].holding.pieces.owned(),
        Known(7)
    );
}

#[test]
fn failed_count_or_assembly_rolls_back_stock_lifecycle_and_references() {
    let (_dir, path, mut store, copy) = setup(AssemblyState::Loose);
    let (line, _) = add(&mut store, copy, Some(10));
    let raw = Connection::open(&path).unwrap();
    raw.execute_batch("CREATE TRIGGER fail_verification BEFORE INSERT ON verification_operation BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
    let request = command(
        "count",
        copy,
        1,
        Action::Count {
            line_id: line,
            total: 7,
        },
    );
    assert!(store.execute_verification(request.clone()).is_err());
    assert!(store.rebuild().unwrap().holdings().is_empty());
    assert_eq!(store.set_verification(copy).unwrap().revision, 1);
    raw.execute_batch("DROP TRIGGER fail_verification").unwrap();
    store.execute_verification(request).unwrap();
    raw.execute_batch("CREATE TRIGGER fail_verification BEFORE INSERT ON verification_operation BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
    assert!(
        store
            .execute_verification(command(
                "assemble",
                copy,
                2,
                Action::Assembly {
                    state: AssemblyState::Assembled
                }
            ))
            .is_err()
    );
    assert_eq!(
        store.set_contents(copy).unwrap()[0].holding.assembly,
        AssemblyState::Loose
    );
    assert_eq!(store.set_verification(copy).unwrap().revision, 2);
}

#[test]
fn disposal_requires_reconciliation_and_never_recreates_stock() {
    let (_dir, _path, mut store, copy) = setup(AssemblyState::Loose);
    let (line, holding) = add(&mut store, copy, Some(10));
    count(&mut store, copy, line, 7);
    store
        .execute_stock(brickxinator_core::stock::Command {
            id: Id::new(),
            request_key: "sold".into(),
            actor: "local-user".into(),
            reason: "Sold two pieces".into(),
            holding_id: holding,
            expected_revision: 1,
            action: brickxinator_core::stock::Action::Adjust(
                brickxinator_core::stock::Adjustment::Remove {
                    kind: brickxinator_core::stock::Removal::Sold,
                    pieces: Some(2),
                    weight_mg: None,
                    damaged_pieces: 0,
                    damaged_weight_mg: 0,
                },
            ),
        })
        .unwrap();
    assert!(
        store
            .execute_stock(brickxinator_core::stock::Command {
                id: Id::new(),
                request_key: "bypass-count".into(),
                actor: "local-user".into(),
                reason: "Try count correction".into(),
                holding_id: holding,
                expected_revision: 2,
                action: brickxinator_core::stock::Action::Adjust(
                    brickxinator_core::stock::Adjustment::Correction {
                        pieces: MeasureChanges {
                            owned: MeasureAction::SetMeasured(9),
                            damaged: MeasureAction::Unchanged
                        },
                        weight_mg: MeasureChanges::default()
                    }
                ),
            })
            .is_err()
    );
    assert!(
        store
            .commit(Operation {
                id: Id::new(),
                request_key: "raw-count".into(),
                actor: "local-user".into(),
                reason: "Try generic edit".into(),
                body: Body::Change {
                    holding_id: holding,
                    expected_revision: 2,
                    pieces: MeasureChanges {
                        owned: MeasureAction::SetMeasured(9),
                        damaged: MeasureAction::Unchanged
                    },
                    weight_mg: MeasureChanges::default(),
                    assembly: None
                }
            })
            .is_err()
    );
    for total in [7, 9] {
        assert!(
            store
                .execute_verification(command(
                    &format!("recount-{total}"),
                    copy,
                    2,
                    Action::Count {
                        line_id: line,
                        total
                    }
                ))
                .is_err()
        );
    }
    assert_eq!(
        store.set_contents(copy).unwrap()[0].holding.pieces.owned(),
        Known(5)
    );
    let state = store.set_verification(copy).unwrap();
    let proposal = command(
        "confirmed-recount",
        copy,
        state.revision,
        Action::ReconcileCount {
            line_id: line,
            total: 9,
            expected_holding_revision: 2,
            expected_owned: Known(5),
        },
    );
    let result = store.execute_verification(proposal.clone()).unwrap();
    assert_eq!(store.execute_verification(proposal).unwrap(), result);
    assert_eq!(
        store.rebuild().unwrap().holdings()[&holding].pieces.owned(),
        Known(9)
    );
    assert_eq!(
        store.set_verification(copy).unwrap().lines[&line].verified,
        Some(9)
    );
    assert!(
        store
            .execute_verification(command(
                "stale-recount",
                copy,
                store.set_verification(copy).unwrap().revision,
                Action::ReconcileCount {
                    line_id: line,
                    total: 10,
                    expected_holding_revision: 2,
                    expected_owned: Known(5)
                }
            ))
            .is_err()
    );
    assert_eq!(
        store.set_contents(copy).unwrap()[0].holding.pieces.owned(),
        Known(9)
    );
}

#[test]
fn concurrent_counts_reject_stale_revisions_and_zero_is_a_real_observation() {
    let (_dir, path, mut store, copy) = setup(AssemblyState::Loose);
    let (line, holding) = add(&mut store, copy, None);
    drop(store);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let workers: Vec<_> = [0, 7]
        .into_iter()
        .map(|total| {
            let path = path.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut store = Store::open(path).unwrap();
                barrier.wait();
                store
                    .execute_verification(command(
                        &format!("count-{total}"),
                        copy,
                        1,
                        Action::Count {
                            line_id: line,
                            total,
                        },
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
    let mut store = Store::open(path).unwrap();
    count(&mut store, copy, line, 0);
    assert_eq!(
        store.rebuild().unwrap().holdings()[&holding].pieces.owned(),
        Known(0)
    );
    let revision = store.set_verification(copy).unwrap().revision;
    store
        .execute_verification(command(
            "finish",
            copy,
            revision,
            Action::Finish { line_id: line },
        ))
        .unwrap();
    assert_eq!(
        store.set_verification(copy).unwrap().lines[&line].difference(),
        Difference::Unknown
    );
}

#[test]
fn uncounted_lines_cannot_finish_and_raw_verification_or_assembly_is_blocked() {
    let (_dir, _path, mut store, copy) = setup(AssemblyState::Loose);
    let (line, holding) = add(&mut store, copy, Some(10));
    assert!(
        store
            .execute_verification(command("finish", copy, 1, Action::Finish { line_id: line }))
            .is_err()
    );
    let raw = Operation {
        id: Id::new(),
        request_key: "bypass".into(),
        actor: "local-user".into(),
        reason: "Try bypass".into(),
        body: Body::Verify {
            copy_id: copy,
            expected_revision: 1,
            action: Action::Count {
                line_id: line,
                total: 10,
            },
        },
    };
    assert!(store.commit(raw).is_err());
    count(&mut store, copy, line, 10);
    assert!(
        store
            .commit(Operation {
                id: Id::new(),
                request_key: "part-assembly".into(),
                actor: "local-user".into(),
                reason: "Try independent assembly".into(),
                body: Body::Change {
                    holding_id: holding,
                    expected_revision: 1,
                    pieces: MeasureChanges::default(),
                    weight_mg: MeasureChanges::default(),
                    assembly: Some(AssemblyState::Assembled)
                }
            })
            .is_err()
    );
}

#[test]
fn copying_checklists_creates_unverified_lines_without_copying_stock() {
    let (_dir, path, mut store, source) = setup(AssemblyState::Loose);
    let (source_line, _) = add(&mut store, source, Some(10));
    count(&mut store, source, source_line, 7);
    let original_purchase = *store.rebuild().unwrap().receipts().keys().next().unwrap();
    let draft = store.purchase(original_purchase).unwrap().unwrap().draft;
    // A different invoice line is a different provenance identity.
    let mut draft = draft;
    draft.lines[0].id = Id::new();
    let purchase = Id::new();
    store
        .execute_purchase(PurchaseCommand {
            id: Id::new(),
            request_key: "copy-draft".into(),
            actor: "local-user".into(),
            reason: String::new(),
            body: PurchaseAction::Create {
                purchase_id: purchase,
                draft,
            },
        })
        .unwrap();
    let receipt = store
        .execute_purchase(PurchaseCommand {
            id: Id::new(),
            request_key: "copy-receive".into(),
            actor: "local-user".into(),
            reason: String::new(),
            body: PurchaseAction::Receive {
                purchase_id: purchase,
                expected_revision: 1,
            },
        })
        .unwrap();
    let target = *receipt.receipt.unwrap().set_copies.keys().next().unwrap();
    let lines = store.prepare_expected_copy(source).unwrap();
    let target_line = lines[0].line_id;
    let request = command(
        "copy-checklist",
        target,
        0,
        Action::CopyExpected {
            source_copy_id: source,
            lines,
        },
    );
    let original = store.execute_verification(request.clone()).unwrap();
    assert!(original.holdings.is_empty());
    let state = store.set_verification(target).unwrap();
    assert_eq!(state.lines[&target_line].expected, Some(10));
    assert_eq!(state.lines[&target_line].verified, None);
    assert!(!state.lines[&target_line].finished);
    assert_eq!(store.rebuild().unwrap().holdings().len(), 1);
    let stale_plan = store.prepare_expected_copy(source).unwrap();
    store
        .execute_verification(command(
            "change-source",
            source,
            2,
            Action::Expected {
                line_id: source_line,
                expected: Some(11),
            },
        ))
        .unwrap();
    assert!(
        store
            .execute_verification(command(
                "stale-copy",
                target,
                1,
                Action::CopyExpected {
                    source_copy_id: source,
                    lines: stale_plan
                }
            ))
            .is_err()
    );
    drop(store);
    let mut store = Store::open(path).unwrap();
    assert_eq!(store.execute_verification(request).unwrap(), original);
    count(&mut store, target, target_line, 3);
    let total: u64 = store
        .rebuild()
        .unwrap()
        .holdings()
        .values()
        .map(|h| match h.pieces.owned() {
            Known(n) => n,
            Unknown => 0,
        })
        .sum();
    assert_eq!(total, 10);
}

#[test]
fn lower_counts_cannot_hide_existing_damage_and_reasons_are_required() {
    let (_dir, _path, mut store, copy) = setup(AssemblyState::Loose);
    let (line, holding) = add(&mut store, copy, Some(10));
    count(&mut store, copy, line, 7);
    store
        .commit(Operation {
            id: Id::new(),
            request_key: "damage".into(),
            actor: "local-user".into(),
            reason: "Two damaged pieces".into(),
            body: Body::Change {
                holding_id: holding,
                expected_revision: 1,
                pieces: MeasureChanges {
                    owned: MeasureAction::Unchanged,
                    damaged: MeasureAction::SetMeasured(2),
                },
                weight_mg: MeasureChanges::default(),
                assembly: None,
            },
        })
        .unwrap();
    assert!(
        store
            .execute_verification(command(
                "too-low",
                copy,
                2,
                Action::Count {
                    line_id: line,
                    total: 1
                }
            ))
            .is_err()
    );
    let mut missing_reason = command(
        "no-reason",
        copy,
        2,
        Action::Count {
            line_id: line,
            total: 9,
        },
    );
    missing_reason.reason = String::new();
    assert!(store.execute_verification(missing_reason).is_err());
    assert_eq!(store.set_verification(copy).unwrap().revision, 2);
    assert_eq!(
        store.set_contents(copy).unwrap()[0].holding.pieces.owned(),
        Known(7)
    );
}

#[test]
fn changed_counts_reopen_completion_but_identical_observations_preserve_it() {
    let (_dir, _path, mut store, copy) = setup(AssemblyState::Loose);
    let (line, _) = add(&mut store, copy, Some(10));
    count(&mut store, copy, line, 7);
    store
        .execute_verification(command("finish", copy, 2, Action::Finish { line_id: line }))
        .unwrap();
    store
        .execute_verification(command("complete", copy, 3, Action::Complete))
        .unwrap();
    count(&mut store, copy, line, 7);
    assert!(store.set_verification(copy).unwrap().completed);
    count(&mut store, copy, line, 9);
    let state = store.set_verification(copy).unwrap();
    assert!(!state.completed);
    assert!(!state.lines[&line].finished);
    assert_eq!(
        state.status(),
        brickxinator_core::verification::Status::InProgress
    );
}

#[test]
fn schema_four_keeps_catalog_without_silently_installing_verification() {
    use sha2::{Digest, Sha256};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("schema-four.sqlite");
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
    let store = Store::open(&path).unwrap();
    assert!(store.catalog().unwrap().is_empty());
    assert!(store.set_verification(Id::new()).is_err());
    let raw = Connection::open(path).unwrap();
    assert_eq!(
        raw.query_row("SELECT count(*) FROM schema_migration", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        4
    );
}
