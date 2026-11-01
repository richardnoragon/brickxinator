use brickxinator_core::{
    finance::{self, Action, Command, Exact, Issue, Method, Remainder, State, Status},
    inventory::{
        AssemblyState,
        Measure::{self, Known, Unknown},
    },
    ledger::Id,
    purchases::{
        Draft, InvoiceLine, LineKind, Purchase, PurchaseAction, PurchaseCommand,
        Status as PurchaseStatus,
    },
};
use brickxinator_infra::sqlite::Store;
use rusqlite::Connection;
use std::collections::BTreeMap;

fn line(price: Option<u64>, pieces: Measure, weight_mg: Measure) -> InvoiceLine {
    InvoiceLine {
        source_url: None,
        notes: String::new(),
        id: Id::new(),
        kind: LineKind::LooseBatch,
        description: None,
        set_number: None,
        copy_count: None,
        pieces,
        weight_mg,
        paid_total_cents: price,
        assembly: AssemblyState::Loose,
    }
}
fn purchase(lines: Vec<InvoiceLine>, shipping: Option<u64>) -> Purchase {
    Purchase {
        archived: false,
        id: Id::new(),
        revision: 1,
        status: PurchaseStatus::Draft,
        draft: Draft {
            lines,
            shipping_cents: shipping,
            ..Draft::default()
        },
    }
}
fn state(purchase: &Purchase, method: Method) -> State {
    let mut state = State::new(purchase.id);
    state.method = method;
    state
}
fn command(
    key: &str,
    purchase_id: Id,
    revision: u64,
    purchase_revision: u64,
    action: Action,
) -> Command {
    Command {
        id: Id::new(),
        request_key: key.into(),
        actor: "local-user".into(),
        reason: String::new(),
        purchase_id,
        expected_revision: revision,
        expected_purchase_revision: purchase_revision,
        action,
    }
}
fn setup(purchase: Purchase) -> (tempfile::TempDir, std::path::PathBuf, Store, Id) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("finance.sqlite");
    let mut store = Store::create(&path).unwrap();
    let id = purchase.id;
    store
        .execute_purchase(PurchaseCommand {
            id: Id::new(),
            request_key: "draft".into(),
            actor: "local-user".into(),
            reason: String::new(),
            body: PurchaseAction::Create {
                purchase_id: id,
                draft: purchase.draft,
            },
        })
        .unwrap();
    (dir, path, store, id)
}
fn shares(purchase: &Purchase, method: Method) -> Vec<Option<u64>> {
    let allocation = finance::allocate(purchase, &state(purchase, method)).unwrap();
    purchase
        .draft
        .lines
        .iter()
        .map(|line| allocation.shares.get(&line.id).copied().flatten())
        .collect()
}

#[test]
fn purchase_level_adds_shipping_once_without_line_shares() {
    let purchase = purchase(vec![line(Some(9300), Known(100), Unknown)], Some(699));
    let costs = finance::costs(&purchase, &State::new(purchase.id)).unwrap();
    assert_eq!(costs.purchase_total_cents, Some(Exact(9999)));
    assert_eq!(costs.allocation.status, Status::PurchaseLevel);
    assert!(costs.allocation.shares.is_empty());
    assert_eq!(costs.lines[0].allocated_total_cents, None);
    assert!(costs.lines[0].paid_unit_costs.is_some());
}

#[test]
fn largest_remainder_uses_invoice_order_and_equal_counts_lines_not_copies() {
    let mut purchase = purchase(
        vec![
            line(Some(1), Known(1), Unknown),
            line(Some(1), Known(1), Unknown),
            line(Some(1), Known(1), Unknown),
        ],
        Some(100),
    );
    assert_eq!(
        shares(&purchase, Method::Equal),
        vec![Some(34), Some(33), Some(33)]
    );
    purchase.draft.lines.swap(0, 2);
    assert_eq!(
        shares(&purchase, Method::Equal),
        vec![Some(34), Some(33), Some(33)]
    );
    purchase.draft.lines.truncate(2);
    purchase.draft.lines[0].kind = LineKind::Set;
    purchase.draft.lines[0].copy_count = Some(3);
    purchase.draft.shipping_cents = Some(1000);
    assert_eq!(shares(&purchase, Method::Equal), vec![Some(500), Some(500)]);
}

#[test]
fn proportional_modes_use_independent_historical_invoice_bases() {
    let purchase = purchase(
        vec![
            line(Some(2000), Known(100), Known(1000)),
            line(Some(8000), Known(500), Known(4000)),
        ],
        Some(1000),
    );
    assert_eq!(
        shares(&purchase, Method::PriceProportional),
        vec![Some(200), Some(800)]
    );
    assert_eq!(
        shares(&purchase, Method::PieceCount),
        vec![Some(167), Some(833)]
    );
    assert_eq!(
        shares(&purchase, Method::Weight),
        vec![Some(200), Some(800)]
    );
    for method in [
        Method::Equal,
        Method::PriceProportional,
        Method::PieceCount,
        Method::Weight,
    ] {
        assert_eq!(
            finance::costs(&purchase, &state(&purchase, method))
                .unwrap()
                .purchase_total_cents,
            Some(Exact(11000))
        );
    }
    assert_eq!(purchase.draft.lines[0].paid_total_cents, Some(2000));
    assert_eq!(purchase.draft.lines[0].pieces, Known(100));
}

#[test]
fn missing_inputs_all_zero_bases_and_known_zero_shares_are_distinct() {
    let mut purchase = purchase(
        vec![
            line(Some(0), Known(0), Known(0)),
            line(Some(2000), Known(5), Known(10)),
        ],
        Some(100),
    );
    for method in [
        Method::PriceProportional,
        Method::PieceCount,
        Method::Weight,
    ] {
        assert_eq!(shares(&purchase, method), vec![Some(0), Some(100)]);
    }
    purchase.draft.lines[1].paid_total_cents = None;
    assert!(matches!(
        finance::allocate(&purchase, &state(&purchase, Method::PriceProportional))
            .unwrap()
            .issues[0],
        Issue::MissingBasis(_)
    ));
    purchase.draft.lines[1].paid_total_cents = Some(0);
    assert_eq!(
        finance::allocate(&purchase, &state(&purchase, Method::PriceProportional))
            .unwrap()
            .issues,
        vec![Issue::ZeroBasis]
    );
    purchase.draft.shipping_cents = None;
    assert!(
        finance::allocate(&purchase, &state(&purchase, Method::Equal))
            .unwrap()
            .issues
            .contains(&Issue::MissingShipping)
    );
    purchase.draft.shipping_cents = Some(0);
    assert_eq!(shares(&purchase, Method::Equal), vec![Some(0), Some(0)]);
    purchase.draft.lines[0].kind = LineKind::Set;
    purchase.draft.lines[0].copy_count = Some(3);
    purchase.draft.lines[0].pieces = Unknown;
    assert!(matches!(
        finance::allocate(&purchase, &state(&purchase, Method::PieceCount))
            .unwrap()
            .issues[0],
        Issue::MissingBasis(_)
    ));
}

#[test]
fn manual_drafts_retain_values_remainders_and_no_effective_costs_until_complete() {
    let purchase = purchase(
        vec![
            line(Some(1000), Known(10), Unknown),
            line(Some(2000), Known(10), Unknown),
            line(Some(3000), Known(10), Unknown),
        ],
        Some(1000),
    );
    let ids: Vec<_> = purchase.draft.lines.iter().map(|l| l.id).collect();
    let mut state = state(&purchase, Method::Manual);
    state.manual_shares =
        BTreeMap::from([(ids[0], Some(300)), (ids[1], Some(400)), (ids[2], None)]);
    let draft = finance::costs(&purchase, &state).unwrap();
    assert_eq!(
        draft.allocation.manual_remainder,
        Some(Remainder::Unallocated(Exact(300)))
    );
    assert_eq!(draft.allocation.status, Status::Incomplete);
    assert_eq!(draft.lines[0].shipping_share_cents, Some(300));
    assert!(
        draft
            .lines
            .iter()
            .all(|l| l.allocated_total_cents.is_none() && l.paid_unit_costs.is_some())
    );
    state.manual_shares.insert(ids[2], Some(300));
    let complete = finance::costs(&purchase, &state).unwrap();
    assert_eq!(complete.allocation.status, Status::Complete);
    assert_eq!(complete.lines[0].allocated_total_cents, Some(Exact(1300)));
    state.manual_shares.insert(ids[0], Some(900));
    let overshoot = finance::costs(&purchase, &state).unwrap();
    assert_eq!(
        overshoot.allocation.manual_remainder,
        Some(Remainder::Overallocated(Exact(600)))
    );
    assert!(
        overshoot
            .lines
            .iter()
            .all(|l| l.allocated_total_cents.is_none())
    );
}

#[test]
fn unit_rates_are_exact_with_requested_display_precision_and_no_zero_denominators() {
    let purchase = purchase(
        vec![line(Some(6000), Known(3500), Known(5_000_000))],
        Some(1000),
    );
    let costs = finance::costs(&purchase, &state(&purchase, Method::Equal)).unwrap();
    let paid = costs.lines[0].paid_unit_costs.as_ref().unwrap();
    assert_eq!(paid.eur_per_kg.unwrap().display(2).unwrap(), "12.00");
    assert_eq!(
        paid.eur_per_piece.unwrap().display(9).unwrap(),
        "0.017142857"
    );
    assert_eq!(
        costs.lines[0]
            .allocated_unit_costs
            .as_ref()
            .unwrap()
            .eur_per_kg
            .unwrap()
            .display(2)
            .unwrap(),
        "14.00"
    );
    assert_eq!(
        finance::Rate::new(1, 200).unwrap().display(2).unwrap(),
        "0.01"
    );
    assert_eq!(
        finance::Rate::new(1, 3).unwrap().display(12).unwrap(),
        "0.333333333333"
    );
    assert!(finance::Rate::new(1, 3).unwrap().display(13).is_err());
    assert_eq!(finance::Rate::new(1, 0), None);
    let empty = purchase_with_zero_measure();
    let costs = finance::costs(&empty, &State::new(empty.id)).unwrap();
    let rates = costs.lines[0].paid_unit_costs.as_ref().unwrap();
    assert_eq!(rates.eur_per_piece, None);
    assert_eq!(rates.eur_per_kg, None);
}
fn purchase_with_zero_measure() -> Purchase {
    purchase(vec![line(Some(0), Known(0), Unknown)], Some(0))
}

#[test]
fn wide_totals_and_products_do_not_overflow_u64_or_lose_json_precision() {
    let purchase = purchase(
        vec![
            line(Some(u64::MAX), Known(u64::MAX), Known(1)),
            line(Some(u64::MAX), Known(u64::MAX), Known(1)),
        ],
        Some(u64::MAX),
    );
    let costs = finance::costs(&purchase, &state(&purchase, Method::PieceCount)).unwrap();
    let allocated: u128 = costs
        .allocation
        .shares
        .values()
        .map(|s| u128::from(s.unwrap()))
        .sum();
    assert_eq!(allocated, u128::from(u64::MAX));
    assert_eq!(
        costs.purchase_total_cents,
        Some(Exact(u128::from(u64::MAX) * 3))
    );
    assert!(costs.lines[0].allocated_total_cents.unwrap().0 > u128::from(u64::MAX));
    let json = serde_json::to_string(&costs).unwrap();
    assert_eq!(
        serde_json::from_str::<finance::Costs>(&json).unwrap(),
        costs
    );
}

#[test]
fn unknown_paid_prices_keep_costs_partial_even_when_shipping_allocation_completes() {
    let purchase = purchase(
        vec![
            line(None, Known(10), Unknown),
            line(Some(1000), Known(10), Unknown),
        ],
        Some(200),
    );
    let costs = finance::costs(&purchase, &state(&purchase, Method::Equal)).unwrap();
    assert_eq!(costs.allocation.status, Status::Complete);
    assert_eq!(costs.unknown_price_lines, 1);
    assert_eq!(costs.known_subtotal_cents, Exact(1000));
    assert_eq!(costs.purchase_total_cents, None);
    assert_eq!(costs.lines[0].allocated_total_cents, None);
    assert_eq!(costs.lines[1].allocated_total_cents, Some(Exact(1100)));
}

#[test]
fn switching_methods_preserves_manual_drafts_and_history_after_restart() {
    let purchase = purchase(
        vec![
            line(Some(1000), Known(10), Known(1000)),
            line(Some(2000), Known(20), Known(2000)),
        ],
        Some(1000),
    );
    let ids: Vec<_> = purchase.draft.lines.iter().map(|l| l.id).collect();
    let (_dir, path, mut store, id) = setup(purchase);
    let manual = BTreeMap::from([(ids[0], Some(300)), (ids[1], None)]);
    store
        .execute_allocation(command(
            "manual",
            id,
            0,
            1,
            Action::SetManualShares(manual.clone()),
        ))
        .unwrap();
    store
        .execute_allocation(command(
            "method-manual",
            id,
            1,
            1,
            Action::SetMethod(Method::Manual),
        ))
        .unwrap();
    let equal = command("equal", id, 2, 1, Action::SetMethod(Method::Equal));
    let original = store.execute_allocation(equal.clone()).unwrap();
    store
        .execute_allocation(command(
            "back-manual",
            id,
            3,
            1,
            Action::SetMethod(Method::Manual),
        ))
        .unwrap();
    drop(store);
    let mut store = Store::open(path).unwrap();
    assert_eq!(store.execute_allocation(equal).unwrap(), original);
    let costs = store.purchase_costs(id).unwrap();
    assert_eq!(costs.state.manual_shares, manual);
    assert_eq!(costs.allocation.status, Status::Incomplete);
    assert_eq!(
        costs.allocation.manual_remainder,
        Some(Remainder::Unallocated(Exact(700)))
    );
    assert_eq!(store.rebuild().unwrap().operations().len(), 0);
    let history = store.allocation_history(id).unwrap();
    assert_eq!(history.len(), 4);
    assert!(
        history
            .iter()
            .all(|r| r.committed_at.ends_with('Z') && r.actor == "local-user")
    );
    assert_eq!(store.purchase(id).unwrap().unwrap().revision, 1);
}

#[test]
fn purchase_edits_recompute_current_costs_and_old_retry_results_remain_historical() {
    let purchase = purchase(
        vec![
            line(Some(1000), Known(10), Unknown),
            line(Some(1000), Known(10), Unknown),
        ],
        Some(1000),
    );
    let (_dir, _path, mut store, id) = setup(purchase);
    let request = command(
        "price",
        id,
        0,
        1,
        Action::SetMethod(Method::PriceProportional),
    );
    let original = store.execute_allocation(request.clone()).unwrap();
    let mut draft = store.purchase(id).unwrap().unwrap().draft;
    draft.lines[0].paid_total_cents = Some(3000);
    store
        .execute_purchase(PurchaseCommand {
            id: Id::new(),
            request_key: "edit-purchase".into(),
            actor: "local-user".into(),
            reason: "Correct paid price".into(),
            body: PurchaseAction::Save {
                purchase_id: id,
                expected_revision: 1,
                status: PurchaseStatus::Ordered,
                draft,
            },
        })
        .unwrap();
    assert_eq!(store.execute_allocation(request).unwrap(), original);
    let current = store.purchase_costs(id).unwrap();
    assert_eq!(current.purchase_revision, 2);
    assert_eq!(current.lines[0].shipping_share_cents, Some(750));
    assert_eq!(original.costs.lines[0].shipping_share_cents, Some(500));
    assert!(
        store
            .execute_allocation(command(
                "stale-source",
                id,
                1,
                1,
                Action::SetMethod(Method::Equal)
            ))
            .is_err()
    );
    assert!(
        store
            .execute_allocation(command(
                "stale-config",
                id,
                0,
                2,
                Action::SetMethod(Method::Equal)
            ))
            .is_err()
    );
}

#[test]
fn removed_manual_targets_are_reported_and_foreign_line_ids_are_rejected() {
    let purchase = purchase(
        vec![
            line(Some(1000), Known(10), Unknown),
            line(Some(2000), Known(10), Unknown),
        ],
        Some(1000),
    );
    let old = purchase.draft.lines[0].id;
    let remaining = purchase.draft.lines[1].id;
    let (_dir, _path, mut store, id) = setup(purchase);
    store
        .execute_allocation(command(
            "shares",
            id,
            0,
            1,
            Action::SetManualShares(BTreeMap::from([(old, Some(300)), (remaining, Some(700))])),
        ))
        .unwrap();
    store
        .execute_allocation(command(
            "method",
            id,
            1,
            1,
            Action::SetMethod(Method::Manual),
        ))
        .unwrap();
    let mut draft = store.purchase(id).unwrap().unwrap().draft;
    draft.lines.remove(0);
    store
        .execute_purchase(PurchaseCommand {
            id: Id::new(),
            request_key: "remove-line".into(),
            actor: "local-user".into(),
            reason: String::new(),
            body: PurchaseAction::Save {
                purchase_id: id,
                expected_revision: 1,
                status: PurchaseStatus::Draft,
                draft,
            },
        })
        .unwrap();
    let current = store.purchase_costs(id).unwrap();
    assert!(
        current
            .allocation
            .issues
            .contains(&Issue::ObsoleteManualTargets(vec![old]))
    );
    assert_eq!(current.state.manual_shares[&old], Some(300));
    assert!(
        store
            .execute_allocation(command(
                "foreign",
                id,
                2,
                2,
                Action::SetManualShares(BTreeMap::from([(Id::new(), Some(1000))]))
            ))
            .is_err()
    );
    store
        .execute_allocation(command(
            "explicit-reset",
            id,
            2,
            2,
            Action::SetManualShares(BTreeMap::from([(remaining, Some(1000))])),
        ))
        .unwrap();
    assert_eq!(
        store.purchase_costs(id).unwrap().allocation.status,
        Status::Complete
    );
}

#[test]
fn failed_audit_rolls_back_config_and_paid_prices_and_history_are_protected() {
    let (_dir, path, mut store, id) = setup(purchase(
        vec![line(Some(1000), Known(10), Known(1000))],
        Some(500),
    ));
    let raw = Connection::open(&path).unwrap();
    raw.execute_batch("CREATE TRIGGER fail_shipping BEFORE INSERT ON shipping_command BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
    let request = command("equal", id, 0, 1, Action::SetMethod(Method::Equal));
    assert!(store.execute_allocation(request.clone()).is_err());
    assert_eq!(store.purchase_costs(id).unwrap().state.revision, 0);
    assert_eq!(
        store.purchase(id).unwrap().unwrap().draft.lines[0].paid_total_cents,
        Some(1000)
    );
    raw.execute_batch("DROP TRIGGER fail_shipping").unwrap();
    store.execute_allocation(request).unwrap();
    assert!(raw.execute("DELETE FROM shipping_command", []).is_err());
    raw.execute("UPDATE shipping_allocation SET method='Weight'", [])
        .unwrap();
    assert!(Store::open(path).is_err());
}

#[test]
fn concurrent_method_changes_cannot_overwrite_the_same_revision() {
    let (_dir, path, store, id) = setup(purchase(
        vec![line(Some(1000), Known(10), Known(1000))],
        Some(500),
    ));
    drop(store);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let workers: Vec<_> = [Method::Equal, Method::Weight]
        .into_iter()
        .enumerate()
        .map(|(i, method)| {
            let path = path.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut store = Store::open(path).unwrap();
                barrier.wait();
                store
                    .execute_allocation(command(
                        &format!("method-{i}"),
                        id,
                        0,
                        1,
                        Action::SetMethod(method),
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
        Store::open(path)
            .unwrap()
            .allocation_history(id)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn received_purchase_costs_use_acquisition_measures_after_stock_removal() {
    let (_dir, _path, mut store, id) = setup(purchase(
        vec![
            line(Some(6000), Known(3500), Known(5_000_000)),
            line(Some(1000), Known(100), Known(1_000_000)),
        ],
        Some(600),
    ));
    let receipt = store
        .execute_purchase(PurchaseCommand {
            id: Id::new(),
            request_key: "receive".into(),
            actor: "local-user".into(),
            reason: String::new(),
            body: PurchaseAction::Receive {
                purchase_id: id,
                expected_revision: 1,
            },
        })
        .unwrap();
    let line_id = store.purchase(id).unwrap().unwrap().draft.lines[0].id;
    let ledger = store.rebuild().unwrap();
    let holding = *ledger
        .provenance()
        .iter()
        .find(|(_, line)| **line == line_id)
        .unwrap()
        .0;
    assert_eq!(receipt.receipt.unwrap().holdings.len(), 2);
    store
        .execute_allocation(command(
            "weight",
            id,
            0,
            2,
            Action::SetMethod(Method::Weight),
        ))
        .unwrap();
    assert_eq!(store.rebuild().unwrap().operations().len(), 1);
    store
        .execute_stock(brickxinator_core::stock::Command {
            id: Id::new(),
            request_key: "sold-weight".into(),
            actor: "local-user".into(),
            reason: "Sold one kilogram".into(),
            holding_id: holding,
            expected_revision: 1,
            action: brickxinator_core::stock::Action::Adjust(
                brickxinator_core::stock::Adjustment::Remove {
                    kind: brickxinator_core::stock::Removal::Sold,
                    pieces: None,
                    weight_mg: Some(1_000_000),
                    damaged_pieces: 0,
                    damaged_weight_mg: 0,
                },
            ),
        })
        .unwrap();
    let costs = store.purchase_costs(id).unwrap();
    assert_eq!(costs.lines[0].shipping_share_cents, Some(500));
    assert_eq!(costs.lines[1].shipping_share_cents, Some(100));
    assert_eq!(
        costs.lines[0]
            .paid_unit_costs
            .as_ref()
            .unwrap()
            .eur_per_kg
            .unwrap()
            .display(2)
            .unwrap(),
        "12.00"
    );
    assert_eq!(
        store.current_inventory().unwrap()[&holding]
            .weight_mg
            .owned(),
        Known(4_000_000)
    );
    assert_eq!(store.purchase(id).unwrap().unwrap().revision, 2);
    assert_eq!(store.rebuild().unwrap().operations().len(), 2);
}

#[test]
fn allocation_keys_share_the_global_namespace_and_negative_shares_are_invalid() {
    let source = purchase(vec![line(Some(1000), Known(10), Unknown)], Some(1000));
    let line = source.draft.lines[0].id;
    let (_dir, _path, mut store, id) = setup(source);
    let mut invalid = serde_json::to_value(command(
        "negative",
        id,
        0,
        1,
        Action::SetManualShares(BTreeMap::from([(line, Some(1000))])),
    ))
    .unwrap();
    invalid["action"]["SetManualShares"][line.to_string()] = serde_json::Value::from(-1);
    assert!(serde_json::from_value::<Command>(invalid).is_err());
    assert!(
        store
            .execute_allocation(command("draft", id, 0, 1, Action::SetMethod(Method::Equal)))
            .is_err()
    );
    store
        .execute_allocation(command(
            "shared",
            id,
            0,
            1,
            Action::SetMethod(Method::Equal),
        ))
        .unwrap();
    let current = store.purchase(id).unwrap().unwrap();
    assert!(
        store
            .execute_purchase(PurchaseCommand {
                id: Id::new(),
                request_key: "shared".into(),
                actor: "local-user".into(),
                reason: String::new(),
                body: PurchaseAction::Save {
                    purchase_id: id,
                    expected_revision: 1,
                    status: PurchaseStatus::Ordered,
                    draft: current.draft
                }
            })
            .is_err()
    );
    assert!(
        store
            .execute_settings(brickxinator_core::settings::Command {
                id: Id::new(),
                request_key: "shared".into(),
                actor: "local-user".into(),
                reason: String::new(),
                accept_name_warnings: false,
                action: brickxinator_core::settings::Action::CreateLocation {
                    id: Id::new(),
                    parent_id: None,
                    name: "Drawer".into()
                }
            })
            .is_err()
    );
    assert_eq!(store.purchase(id).unwrap().unwrap().revision, 1);
}

#[test]
fn schema_seven_keeps_search_without_silently_installing_finance() {
    use sha2::{Digest, Sha256};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("schema-seven.sqlite");
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
        (
            "migration_006.sql",
            include_str!("../../../migrations/migration_006.sql"),
        ),
        (
            "migration_007.sql",
            include_str!("../../../migrations/migration_007.sql"),
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
    assert_eq!(store.dashboard().unwrap().inventory.all.holdings, 0);
    assert!(store.purchase_costs(Id::new()).is_err());
    let raw = Connection::open(path).unwrap();
    assert_eq!(
        raw.query_row("SELECT count(*) FROM schema_migration", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        7
    );
}
