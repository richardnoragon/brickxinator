use brickxinator_core::{
    catalog,
    inventory::{
        AssemblyState,
        Measure::{self, Known, Unknown},
    },
    ledger::{Body, Id, Operation},
    profiles::{self, Target as ProfileTarget},
    purchases::{Draft, InvoiceLine, LineKind, PurchaseAction, PurchaseCommand, Status},
    settings::{self, Kind as SettingKind, Vocabulary},
    stock, verification,
    views::{Availability, Kind, Search, Target},
};
use brickxinator_infra::sqlite::Store;
use rusqlite::Connection;

fn setup() -> (tempfile::TempDir, std::path::PathBuf, Store) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("search.sqlite");
    let store = Store::create(&path).unwrap();
    (dir, path, store)
}

#[test]
fn price_history_is_per_invoice_line_and_never_prices_mixed_lot_children_as_parts() {
    let (_dir, _path, mut store) = setup();
    let manufacturer = vocabulary(
        &mut store,
        "brand",
        "Example manufacturer",
        Vocabulary::Manufacturer,
    );
    let part = definition(&mut store, "part", manufacturer, "3001", "Brick");
    let (_, one) = purchase(
        &mut store,
        "one",
        LineKind::LooseBatch,
        Known(10),
        Known(1000),
        None,
        true,
    );
    let (_, two) = purchase(
        &mut store,
        "two",
        LineKind::LooseBatch,
        Known(20),
        Unknown,
        None,
        true,
    );
    for (key, receipt) in [("link-one", one), ("link-two", two)] {
        assign(
            &mut store,
            key,
            ProfileTarget::Holding(*receipt.unwrap().holdings.keys().next().unwrap()),
            0,
            Some(part),
            (None, None),
            None,
        );
    }
    let history = store.price_history(part).unwrap();
    assert_eq!(history.len(), 2);
    assert!(
        history
            .iter()
            .all(|price| price.paid_total_cents == Some(8000)
                && price.attributable
                && price.current)
    );
    let (_, lot) = purchase(
        &mut store,
        "lot",
        LineKind::MixedLot,
        Known(30),
        Unknown,
        None,
        true,
    );
    let source = *lot.unwrap().holdings.keys().next().unwrap();
    let child = Id::new();
    store
        .commit(Operation {
            id: Id::new(),
            request_key: "identify-price".into(),
            actor: "test".into(),
            reason: "classify".into(),
            body: Body::Identify {
                source_id: source,
                child_id: child,
                expected_revision: 1,
                pieces: Some(5),
                weight_mg: None,
            },
        })
        .unwrap();
    assign(
        &mut store,
        "link-child",
        ProfileTarget::Holding(child),
        0,
        Some(part),
        (None, None),
        None,
    );
    let history = store.price_history(part).unwrap();
    assert_eq!(history.len(), 3);
    let source_price = history
        .iter()
        .find(|price| price.kind == LineKind::MixedLot)
        .unwrap();
    assert!(!source_price.attributable);
    assert!(source_price.paid_unit_costs.is_none());
}

#[test]
fn archive_hides_daily_rows_but_preserves_stock_and_unarchives_after_restart() {
    let (_dir, path, mut store) = setup();
    let (id, receipt) = purchase(
        &mut store,
        "archive",
        LineKind::LooseBatch,
        Known(14),
        Unknown,
        None,
        true,
    );
    let holding = *receipt.unwrap().holdings.keys().next().unwrap();
    let balances = store.rebuild().unwrap().holdings().clone();
    let archive = PurchaseCommand {
        id: Id::new(),
        request_key: "archive-purchase".into(),
        actor: "test".into(),
        reason: String::new(),
        body: PurchaseAction::Archive {
            purchase_id: id,
            expected_revision: 2,
            archived: true,
        },
    };
    store.execute_purchase(archive.clone()).unwrap();
    store.execute_purchase(archive).unwrap();
    let daily = Search {
        archived: Some(false),
        ..Default::default()
    };
    assert_eq!(store.search_inventory(&daily).unwrap().total_matches, 0);
    assert_eq!(
        store
            .search_inventory(&Search::default())
            .unwrap()
            .total_matches,
        1
    );
    assert_eq!(
        store
            .dashboard()
            .unwrap()
            .inventory
            .all
            .owned_pieces
            .known_sum,
        14
    );
    assert!(store.dashboard().unwrap().recent_purchases.is_empty());
    assert_eq!(store.rebuild().unwrap().holdings(), &balances);
    store
        .execute_purchase(PurchaseCommand {
            id: Id::new(),
            request_key: "unarchive-purchase".into(),
            actor: "test".into(),
            reason: String::new(),
            body: PurchaseAction::Archive {
                purchase_id: id,
                expected_revision: 3,
                archived: false,
            },
        })
        .unwrap();
    let mut links = store.default_inventory_links().unwrap();
    links.archived = true;
    store
        .execute_profile(profiles::Command {
            id: Id::new(),
            request_key: "archive-holding".into(),
            actor: "test".into(),
            reason: String::new(),
            target: ProfileTarget::Holding(holding),
            expected_revision: 0,
            links: links.clone(),
        })
        .unwrap();
    drop(store);
    let mut store = Store::open(path).unwrap();
    assert_eq!(store.search_inventory(&daily).unwrap().total_matches, 0);
    assert_eq!(
        store
            .dashboard()
            .unwrap()
            .inventory
            .all
            .owned_pieces
            .known_sum,
        14
    );
    links.archived = false;
    store
        .execute_profile(profiles::Command {
            id: Id::new(),
            request_key: "unarchive-holding".into(),
            actor: "test".into(),
            reason: String::new(),
            target: ProfileTarget::Holding(holding),
            expected_revision: 1,
            links,
        })
        .unwrap();
    assert_eq!(store.search_inventory(&daily).unwrap().total_matches, 1);
    assert_eq!(store.rebuild().unwrap().holdings(), &balances);
}

#[test]
fn seller_and_inclusive_purchase_date_filters_preserve_unknown_dates() {
    let (_dir, _path, mut store) = setup();
    let (dated, _) = purchase(
        &mut store,
        "dated",
        LineKind::LooseBatch,
        Known(12),
        Unknown,
        None,
        false,
    );
    let mut draft = store.purchase(dated).unwrap().unwrap().draft;
    draft.purchased_on = Some("2026-11-01".into());
    store
        .execute_purchase(PurchaseCommand {
            id: Id::new(),
            request_key: "date-edit".into(),
            actor: "test".into(),
            reason: String::new(),
            body: PurchaseAction::Save {
                purchase_id: dated,
                expected_revision: 1,
                status: Status::Draft,
                draft,
            },
        })
        .unwrap();
    store
        .execute_purchase(PurchaseCommand {
            id: Id::new(),
            request_key: "date-receive".into(),
            actor: "test".into(),
            reason: String::new(),
            body: PurchaseAction::Receive {
                purchase_id: dated,
                expected_revision: 2,
            },
        })
        .unwrap();
    purchase(
        &mut store,
        "unknown-date",
        LineKind::LooseBatch,
        Known(3),
        Unknown,
        None,
        true,
    );
    let mut query = Search {
        seller: Some("example SELLER".into()),
        purchased_from: Some("2026-11-01".into()),
        purchased_through: Some("2026-11-01".into()),
        ..Default::default()
    };
    let page = store.search_inventory(&query).unwrap();
    assert_eq!(page.total_matches, 1);
    assert_eq!(page.totals.all.owned_pieces.known_sum, 12);
    query.purchased_from = None;
    query.purchased_through = None;
    assert_eq!(store.search_inventory(&query).unwrap().total_matches, 2);
    query.purchased_from = Some("2026-02-30".into());
    assert!(store.search_inventory(&query).is_err());
    query.purchased_from = Some("2026-11-02".into());
    query.purchased_through = Some("2026-11-01".into());
    assert!(store.search_inventory(&query).is_err());
}
fn settings(store: &mut Store, key: &str, action: settings::Action) {
    store
        .execute_settings(settings::Command {
            id: Id::new(),
            request_key: key.into(),
            actor: "local-user".into(),
            reason: String::new(),
            accept_name_warnings: false,
            action,
        })
        .unwrap();
}
fn vocabulary(store: &mut Store, key: &str, name: &str, kind: Vocabulary) -> Id {
    let id = Id::new();
    settings(
        store,
        key,
        settings::Action::CreateVocabulary {
            id,
            vocabulary: kind,
            name: name.into(),
        },
    );
    id
}
fn location(store: &mut Store, key: &str, name: &str, parent_id: Option<Id>) -> Id {
    let id = Id::new();
    settings(
        store,
        key,
        settings::Action::CreateLocation {
            id,
            name: name.into(),
            parent_id,
        },
    );
    id
}
fn definition(store: &mut Store, key: &str, manufacturer: Id, number: &str, name: &str) -> Id {
    let id = Id::new();
    store
        .execute_catalog(catalog::Command {
            id: Id::new(),
            request_key: key.into(),
            actor: "local-user".into(),
            reason: String::new(),
            action: catalog::Action::Create {
                id,
                kind: catalog::Kind::Part,
                details: catalog::Details {
                    expected_contents: Vec::new(),
                    manufacturer_id: manufacturer,
                    number: Some(number.into()),
                    name: Some(name.into()),
                    description: Some("Synthetic brick catalog".into()),
                    release_year: None,
                    theme: None,
                    stated_piece_count: None,
                },
            },
        })
        .unwrap();
    id
}
fn holding(store: &mut Store, key: &str, pieces: Measure, weight_mg: Measure) -> Id {
    let id = Id::new();
    store
        .commit(Operation {
            id: Id::new(),
            request_key: key.into(),
            actor: "local-user".into(),
            reason: "Synthetic observation".into(),
            body: Body::Create {
                holding_id: id,
                pieces,
                weight_mg,
            },
        })
        .unwrap();
    id
}
fn assign(
    store: &mut Store,
    key: &str,
    target: ProfileTarget,
    revision: u64,
    definition: Option<Id>,
    attributes: (Option<Id>, Option<Id>),
    location: Option<Id>,
) {
    let (color, condition) = attributes;
    let mut links = store.default_inventory_links().unwrap();
    links.definition_id = definition;
    links.location_id = location;
    if let Some(id) = color {
        links.color_id = id;
    }
    if let Some(id) = condition {
        links.condition_id = id;
    }
    store
        .execute_profile(profiles::Command {
            id: Id::new(),
            request_key: key.into(),
            actor: "local-user".into(),
            reason: String::new(),
            target,
            expected_revision: revision,
            links,
        })
        .unwrap();
}
fn purchase(
    store: &mut Store,
    key: &str,
    kind: LineKind,
    pieces: Measure,
    weight_mg: Measure,
    copies: Option<u32>,
    receive: bool,
) -> (Id, Option<brickxinator_core::ledger::Outcome>) {
    let id = Id::new();
    store
        .execute_purchase(PurchaseCommand {
            id: Id::new(),
            request_key: format!("{key}-draft"),
            actor: "local-user".into(),
            reason: String::new(),
            body: PurchaseAction::Create {
                purchase_id: id,
                draft: Draft {
                    seller: Some("Example Seller".into()),
                    shipping_cents: Some(0),
                    lines: vec![InvoiceLine {
                        source_url: None,
                        notes: String::new(),
                        id: Id::new(),
                        kind,
                        description: Some(format!("Synthetic {key}")),
                        set_number: if kind == LineKind::Set {
                            Some("7838".into())
                        } else {
                            None
                        },
                        copy_count: copies,
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
    let outcome = if receive {
        store
            .execute_purchase(PurchaseCommand {
                id: Id::new(),
                request_key: format!("{key}-receive"),
                actor: "local-user".into(),
                reason: String::new(),
                body: PurchaseAction::Receive {
                    purchase_id: id,
                    expected_revision: 1,
                },
            })
            .unwrap()
            .receipt
    } else {
        None
    };
    (id, outcome)
}

#[test]
fn exact_numbers_fts_filters_pagination_and_scope_totals() {
    let (_dir, path, mut store) = setup();
    let brand = vocabulary(&mut store, "brand", "LEGO", Vocabulary::Manufacturer);
    let red = vocabulary(&mut store, "red", "Red", Vocabulary::Color);
    let used = store
        .settings()
        .unwrap()
        .values()
        .find(|r| r.name == "Used")
        .unwrap()
        .id;
    let room = location(&mut store, "room", "Workshop", None);
    let drawer = location(&mut store, "drawer", "Drawer 14", Some(room));
    let exact = definition(&mut store, "exact", brand, "3001-1", "Red train brick");
    let related = definition(
        &mut store,
        "related",
        brand,
        "9999",
        "Spare 3001-1 train brick",
    );
    let a = holding(&mut store, "a", Known(100), Unknown);
    let b = holding(&mut store, "b", Known(50), Known(500));
    assign(
        &mut store,
        "assign-a",
        ProfileTarget::Holding(a),
        0,
        Some(exact),
        (Some(red), Some(used)),
        Some(drawer),
    );
    assign(
        &mut store,
        "assign-b",
        ProfileTarget::Holding(b),
        0,
        Some(related),
        (Some(red), Some(used)),
        Some(drawer),
    );
    let query = Search {
        text: "3001-1".into(),
        limit: 1,
        ..Search::default()
    };
    let first = store.search_inventory(&query).unwrap();
    assert_eq!(first.total_matches, 2);
    assert_eq!(first.rows.len(), 1);
    assert_eq!(first.rows[0].target, Target::Holding(a));
    assert_eq!(first.totals.all.owned_pieces.known_sum, 150);
    assert_eq!(first.totals.all.owned_weight_mg.unknown_holdings, 1);
    let next = store
        .search_inventory(&Search { offset: 1, ..query })
        .unwrap();
    assert_eq!(next.rows[0].target, Target::Holding(b));
    assert_eq!(next.totals, first.totals);
    assert_eq!(
        store
            .search_inventory(&Search {
                number: Some("3001-1".into()),
                ..Search::default()
            })
            .unwrap()
            .total_matches,
        1
    );
    assert_eq!(
        store
            .search_inventory(&Search {
                text: "train brick".into(),
                kind: Some(Kind::OpeningHolding),
                manufacturer_id: Some(brand),
                color_id: Some(red),
                condition_id: Some(used),
                location_id: Some(room),
                incomplete: Some(false),
                ..Search::default()
            })
            .unwrap()
            .total_matches,
        2
    );
    assert_eq!(
        store
            .search_inventory(&Search {
                location_id: Some(room),
                include_descendants: false,
                ..Search::default()
            })
            .unwrap()
            .total_matches,
        0
    );
    drop(store);
    let mut store = Store::open(path).unwrap();
    assert_eq!(
        store
            .search_inventory(&Search::default())
            .unwrap()
            .total_matches,
        2
    );
}

#[test]
fn cache_refreshes_after_renames_moves_archiving_and_stock_changes() {
    let (_dir, path, mut store) = setup();
    let brand = vocabulary(&mut store, "brand", "LEGO", Vocabulary::Manufacturer);
    let part = definition(&mut store, "part", brand, "3001", "Train brick");
    let shelf = location(&mut store, "shelf", "Shelf", None);
    let id = holding(&mut store, "receive", Known(100), Unknown);
    assign(
        &mut store,
        "assign",
        ProfileTarget::Holding(id),
        0,
        Some(part),
        (None, None),
        Some(shelf),
    );
    assert_eq!(
        store
            .search_inventory(&Search {
                text: "Shelf".into(),
                ..Search::default()
            })
            .unwrap()
            .total_matches,
        1
    );
    settings(
        &mut store,
        "rename",
        settings::Action::Rename {
            id: shelf,
            expected_revision: 1,
            name: "Display cabinet".into(),
        },
    );
    assert_eq!(
        store
            .search_inventory(&Search {
                text: "Shelf".into(),
                ..Search::default()
            })
            .unwrap()
            .total_matches,
        0
    );
    assert_eq!(
        store
            .search_inventory(&Search {
                text: "cabinet".into(),
                ..Search::default()
            })
            .unwrap()
            .total_matches,
        1
    );
    store
        .execute_catalog(catalog::Command {
            id: Id::new(),
            request_key: "archive".into(),
            actor: "local-user".into(),
            reason: String::new(),
            action: catalog::Action::Archive {
                id: part,
                expected_revision: 1,
                archived: true,
            },
        })
        .unwrap();
    assert_eq!(
        store
            .search_inventory(&Search {
                archived_references: Some(true),
                ..Search::default()
            })
            .unwrap()
            .total_matches,
        1
    );
    assert_eq!(
        store
            .dashboard()
            .unwrap()
            .inventory
            .all
            .owned_pieces
            .known_sum,
        100
    );
    store
        .execute_stock(stock::Command {
            id: Id::new(),
            request_key: "sold".into(),
            actor: "local-user".into(),
            reason: "Sold five".into(),
            holding_id: id,
            expected_revision: 1,
            action: stock::Action::Adjust(stock::Adjustment::Remove {
                kind: stock::Removal::Sold,
                pieces: Some(5),
                weight_mg: None,
                damaged_pieces: 0,
                damaged_weight_mg: 0,
            }),
        })
        .unwrap();
    assert_eq!(
        store
            .dashboard()
            .unwrap()
            .inventory
            .all
            .owned_pieces
            .known_sum,
        95
    );
    let raw = Connection::open(path).unwrap();
    let dirty: i64 = raw
        .query_row("SELECT dirty FROM inventory_search_state", [], |r| r.get(0))
        .unwrap();
    assert_eq!(dirty, 0);
}

#[test]
fn cache_rebuild_and_dropped_cache_repair_do_not_change_source_history() {
    let (_dir, path, mut store) = setup();
    holding(&mut store, "a", Known(10), Unknown);
    let original = store.search_inventory(&Search::default()).unwrap();
    let dashboard = store.dashboard().unwrap();
    let operation_count = store.rebuild().unwrap().operations().len();
    let raw = Connection::open(&path).unwrap();
    raw.execute("DELETE FROM inventory_search_fts", []).unwrap();
    assert_eq!(store.rebuild_inventory_search().unwrap(), 1);
    assert_eq!(
        store.search_inventory(&Search::default()).unwrap(),
        original
    );
    raw.execute_batch("DROP TABLE inventory_search_fts; DROP TABLE inventory_search_document;")
        .unwrap();
    assert_eq!(
        store.search_inventory(&Search::default()).unwrap(),
        original
    );
    assert_eq!(store.dashboard().unwrap(), dashboard);
    assert_eq!(store.rebuild().unwrap().operations().len(), operation_count);
}

#[test]
fn literal_queries_cannot_inject_sql_or_fts_operators_and_limits_are_checked() {
    let (_dir, _path, mut store) = setup();
    holding(&mut store, "a", Known(10), Unknown);
    for text in [
        "\" OR *",
        "'); DELETE FROM purchase; --",
        "NEAR(a b)",
        "*",
        "[]",
        "a:b",
        "\"\"",
        "😀",
    ] {
        assert_eq!(
            store
                .search_inventory(&Search {
                    text: text.into(),
                    ..Search::default()
                })
                .unwrap()
                .total_matches,
            0
        );
    }
    assert!(
        store
            .search_inventory(&Search {
                limit: 0,
                ..Search::default()
            })
            .is_err()
    );
    assert!(
        store
            .search_inventory(&Search {
                limit: 201,
                ..Search::default()
            })
            .is_err()
    );
    assert!(
        store
            .search_inventory(&Search {
                number: Some(" ".into()),
                ..Search::default()
            })
            .is_err()
    );
    assert_eq!(
        store
            .search_inventory(&Search::default())
            .unwrap()
            .total_matches,
        1
    );
}

#[test]
fn completed_residue_is_excluded_and_unidentified_measures_stay_separate() {
    let (_dir, _path, mut store) = setup();
    let (_, receipt) = purchase(
        &mut store,
        "mixed",
        LineKind::MixedLot,
        Unknown,
        Known(5_000_000),
        None,
        true,
    );
    let source = *receipt.unwrap().holdings.keys().next().unwrap();
    let child = Id::new();
    store
        .commit(Operation {
            id: Id::new(),
            request_key: "identify".into(),
            actor: "local-user".into(),
            reason: "Extract unweighed pieces".into(),
            body: Body::Identify {
                source_id: source,
                child_id: child,
                expected_revision: 1,
                pieces: Some(100),
                weight_mg: None,
            },
        })
        .unwrap();
    let before = store.dashboard().unwrap();
    assert_eq!(
        before.inventory.unidentified_lots.owned_weight_mg.known_sum,
        0
    );
    assert_eq!(
        before
            .inventory
            .unidentified_lots
            .owned_weight_mg
            .unknown_holdings,
        1
    );
    assert_eq!(before.inventory.all.owned_pieces.known_sum, 100);
    store
        .execute_stock(stock::Command {
            id: Id::new(),
            request_key: "complete".into(),
            actor: "local-user".into(),
            reason: "Confirmed empty".into(),
            holding_id: source,
            expected_revision: 2,
            action: stock::Action::Complete {
                confirm_physical_empty: true,
            },
        })
        .unwrap();
    let after = store.search_inventory(&Search::default()).unwrap();
    assert_eq!(after.total_matches, 1);
    assert_eq!(after.rows[0].target, Target::Holding(child));
    assert_eq!(after.totals.unidentified_lots.holdings, 0);
    assert_eq!(after.totals.all.owned_pieces.known_sum, 100);
    store
        .execute_stock(stock::Command {
            id: Id::new(),
            request_key: "reopen".into(),
            actor: "local-user".into(),
            reason: "Found more residue".into(),
            holding_id: source,
            expected_revision: 3,
            action: stock::Action::Reopen,
        })
        .unwrap();
    assert_eq!(
        store
            .search_inventory(&Search::default())
            .unwrap()
            .total_matches,
        2
    );
    assert_eq!(
        store
            .dashboard()
            .unwrap()
            .inventory
            .all
            .owned_weight_mg
            .known_sum,
        0
    );
}

#[test]
fn set_copies_and_unknown_copy_lines_never_inflate_part_totals() {
    let (_dir, _path, mut store) = setup();
    let (_, receipt) = purchase(
        &mut store,
        "known",
        LineKind::Set,
        Known(451),
        Unknown,
        Some(3),
        true,
    );
    let copy = *receipt.unwrap().set_copies.keys().next().unwrap();
    purchase(
        &mut store,
        "unknown",
        LineKind::Set,
        Known(999),
        Unknown,
        None,
        true,
    );
    let dashboard = store.dashboard().unwrap();
    assert_eq!(dashboard.inventory.known_set_copies, 3);
    assert_eq!(dashboard.inventory.unquantified_set_lines, 1);
    assert_eq!(dashboard.inventory.all.holdings, 0);
    assert_eq!(dashboard.inventory.all.owned_pieces.known_sum, 0);
    let color = store
        .settings()
        .unwrap()
        .values()
        .find(|r| {
            matches!(
                r.kind,
                SettingKind::Vocabulary {
                    vocabulary: Vocabulary::Color,
                    system_unknown: true
                }
            )
        })
        .unwrap()
        .id;
    let line = Id::new();
    store
        .execute_verification(verification::Command {
            id: Id::new(),
            request_key: "expected".into(),
            actor: "local-user".into(),
            reason: String::new(),
            copy_id: copy,
            expected_revision: 0,
            action: verification::Action::AddLine {
                line_id: line,
                holding_id: Id::new(),
                definition_id: None,
                color_id: color,
                expected: Some(10),
            },
        })
        .unwrap();
    assert_eq!(
        store
            .dashboard()
            .unwrap()
            .inventory
            .all
            .owned_pieces
            .known_sum,
        0
    );
    store
        .execute_verification(verification::Command {
            id: Id::new(),
            request_key: "count".into(),
            actor: "local-user".into(),
            reason: "Observed seven pieces".into(),
            copy_id: copy,
            expected_revision: 1,
            action: verification::Action::Count {
                line_id: line,
                total: 7,
            },
        })
        .unwrap();
    let result = store
        .search_inventory(&Search {
            text: "7838".into(),
            ..Search::default()
        })
        .unwrap();
    assert_eq!(result.total_matches, 5);
    assert_eq!(result.totals.all.owned_pieces.known_sum, 7);
    assert_eq!(result.totals.known_set_copies, 3);
}

#[test]
fn availability_facets_overlap_and_sums_remain_exact_above_u64() {
    let (_dir, _path, mut store) = setup();
    let a = holding(&mut store, "a", Known(u64::MAX), Unknown);
    holding(&mut store, "b", Known(u64::MAX), Known(0));
    store
        .execute_stock(stock::Command {
            id: Id::new(),
            request_key: "broken".into(),
            actor: "local-user".into(),
            reason: "Broken two".into(),
            holding_id: a,
            expected_revision: 1,
            action: stock::Action::Adjust(stock::Adjustment::Broken {
                pieces: Some(2),
                weight_mg: None,
            }),
        })
        .unwrap();
    assert_eq!(
        store
            .search_inventory(&Search {
                availability: Some(Availability::Available),
                ..Search::default()
            })
            .unwrap()
            .total_matches,
        2
    );
    assert_eq!(
        store
            .search_inventory(&Search {
                availability: Some(Availability::BrokenDamaged),
                ..Search::default()
            })
            .unwrap()
            .total_matches,
        1
    );
    assert_eq!(
        store
            .search_inventory(&Search {
                availability: Some(Availability::Unknown),
                ..Search::default()
            })
            .unwrap()
            .total_matches,
        1
    );
    let total = store.dashboard().unwrap();
    assert_eq!(
        total.inventory.all.owned_pieces.known_sum,
        u128::from(u64::MAX) * 2
    );
    assert_eq!(
        total.inventory.all.available_pieces.known_sum,
        u128::from(u64::MAX) * 2 - 2
    );
    let encoded = serde_json::to_string(&total).unwrap();
    assert!(encoded.contains("36893488147419103230"));
    assert_eq!(
        serde_json::from_str::<brickxinator_core::views::Dashboard>(&encoded).unwrap(),
        total
    );
}

#[test]
fn pending_delivery_and_recent_purchase_views_use_recorded_history() {
    let (_dir, _path, mut store) = setup();
    let (pending, _) = purchase(
        &mut store,
        "pending",
        LineKind::LooseBatch,
        Known(10),
        Unknown,
        None,
        false,
    );
    purchase(
        &mut store,
        "received",
        LineKind::LooseBatch,
        Known(5),
        Unknown,
        None,
        true,
    );
    let draft = store.purchase(pending).unwrap().unwrap().draft;
    store
        .execute_purchase(PurchaseCommand {
            id: Id::new(),
            request_key: "ordered".into(),
            actor: "local-user".into(),
            reason: String::new(),
            body: PurchaseAction::Save {
                purchase_id: pending,
                expected_revision: 1,
                status: Status::Ordered,
                draft,
            },
        })
        .unwrap();
    let dashboard = store.dashboard().unwrap();
    assert_eq!(dashboard.pending_deliveries, 1);
    assert_eq!(dashboard.inventory.all.owned_pieces.known_sum, 5);
    assert_eq!(dashboard.recent_purchases.len(), 2);
    assert!(
        dashboard
            .recent_purchases
            .iter()
            .all(|p| p.recorded_at.ends_with('Z'))
    );
}

#[test]
fn set_contents_follow_parent_storage_and_assembly_in_search() {
    let (_dir, _path, mut store) = setup();
    let (_, receipt) = purchase(
        &mut store,
        "set",
        LineKind::Set,
        Known(451),
        Unknown,
        Some(1),
        true,
    );
    let copy = *receipt.unwrap().set_copies.keys().next().unwrap();
    let room = location(&mut store, "room", "Workshop", None);
    let drawer = location(&mut store, "drawer", "Drawer", Some(room));
    let shelf = location(&mut store, "shelf", "Shelf", None);
    assign(
        &mut store,
        "copy-storage",
        ProfileTarget::SetCopy(copy),
        0,
        None,
        (None, None),
        Some(drawer),
    );
    let color = store.default_inventory_links().unwrap().color_id;
    let line = Id::new();
    for (key, revision, action) in [
        (
            "expected",
            0,
            verification::Action::AddLine {
                line_id: line,
                holding_id: Id::new(),
                definition_id: None,
                color_id: color,
                expected: Some(10),
            },
        ),
        (
            "count",
            1,
            verification::Action::Count {
                line_id: line,
                total: 7,
            },
        ),
        (
            "assembled",
            2,
            verification::Action::Assembly {
                state: AssemblyState::PartiallyAssembled,
            },
        ),
    ] {
        store
            .execute_verification(verification::Command {
                id: Id::new(),
                request_key: key.into(),
                actor: "local-user".into(),
                reason: "Synthetic set observation".into(),
                copy_id: copy,
                expected_revision: revision,
                action,
            })
            .unwrap();
    }
    let content = Search {
        kind: Some(Kind::SetContent),
        location_id: Some(room),
        availability: Some(Availability::InAssembledSet),
        ..Search::default()
    };
    assert_eq!(
        store
            .search_inventory(&Search {
                assembly: Some(AssemblyState::Loose),
                ..Default::default()
            })
            .unwrap()
            .total_matches,
        0
    );
    assert_eq!(
        store
            .search_inventory(&Search {
                assembly: Some(AssemblyState::Assembled),
                ..Default::default()
            })
            .unwrap()
            .total_matches,
        0
    );
    assert_eq!(
        store
            .search_inventory(&Search {
                assembly: Some(AssemblyState::PartiallyAssembled),
                ..Default::default()
            })
            .unwrap()
            .total_matches,
        2
    );
    let page = store.search_inventory(&content).unwrap();
    assert_eq!(page.total_matches, 1);
    assert_eq!(page.totals.all.owned_pieces.known_sum, 7);
    assert_eq!(page.totals.all.available_pieces.known_sum, 0);
    assign(
        &mut store,
        "copy-move",
        ProfileTarget::SetCopy(copy),
        1,
        None,
        (None, None),
        Some(shelf),
    );
    assert_eq!(store.search_inventory(&content).unwrap().total_matches, 0);
    store
        .execute_verification(verification::Command {
            id: Id::new(),
            request_key: "loose".into(),
            actor: "local-user".into(),
            reason: String::new(),
            copy_id: copy,
            expected_revision: 3,
            action: verification::Action::Assembly {
                state: AssemblyState::Loose,
            },
        })
        .unwrap();
    let moved = store
        .search_inventory(&Search {
            kind: Some(Kind::SetContent),
            location_id: Some(shelf),
            availability: Some(Availability::Available),
            ..Search::default()
        })
        .unwrap();
    assert_eq!(moved.total_matches, 1);
    assert_eq!(moved.totals.all.available_pieces.known_sum, 7);
}

#[test]
fn failed_projection_refresh_rolls_back_cache_and_preserves_source_data() {
    let (_dir, path, mut store) = setup();
    let id = holding(&mut store, "a", Known(10), Unknown);
    assert_eq!(
        store
            .dashboard()
            .unwrap()
            .inventory
            .all
            .owned_pieces
            .known_sum,
        10
    );
    store
        .execute_stock(stock::Command {
            id: Id::new(),
            request_key: "sold".into(),
            actor: "local-user".into(),
            reason: "Sold five".into(),
            holding_id: id,
            expected_revision: 1,
            action: stock::Action::Adjust(stock::Adjustment::Remove {
                kind: stock::Removal::Sold,
                pieces: Some(5),
                weight_mg: None,
                damaged_pieces: 0,
                damaged_weight_mg: 0,
            }),
        })
        .unwrap();
    let raw = Connection::open(path).unwrap();
    raw.execute_batch("CREATE TRIGGER fail_cache BEFORE INSERT ON inventory_search_document BEGIN SELECT RAISE(ABORT,'injected cache failure'); END;").unwrap();
    assert!(store.dashboard().is_err());
    assert_eq!(
        store.current_inventory().unwrap()[&id].pieces.owned(),
        Known(5)
    );
    assert_eq!(
        raw.query_row("SELECT dirty FROM inventory_search_state", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    raw.execute_batch("DROP TRIGGER fail_cache").unwrap();
    assert_eq!(
        store
            .dashboard()
            .unwrap()
            .inventory
            .all
            .owned_pieces
            .known_sum,
        5
    );
    assert_eq!(store.rebuild().unwrap().operations().len(), 2);
}

#[test]
fn schema_six_keeps_inventory_without_silently_installing_search() {
    use sha2::{Digest, Sha256};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("schema-six.sqlite");
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
    assert!(store.dashboard().is_err());
    assert!(store.search_inventory(&Search::default()).is_err());
    let raw = Connection::open(path).unwrap();
    assert_eq!(
        raw.query_row("SELECT count(*) FROM schema_migration", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        6
    );
}
