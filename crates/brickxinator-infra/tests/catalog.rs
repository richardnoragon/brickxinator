use brickxinator_core::{
    catalog::{self, Details},
    inventory::Measure::{Known, Unknown},
    ledger::{Body, Id, Operation},
    profiles::{self, Target},
    settings::{self, Kind, Vocabulary},
};
use brickxinator_infra::sqlite::Store;
use rusqlite::Connection;

fn setup() -> (tempfile::TempDir, std::path::PathBuf, Store) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("catalog.sqlite");
    let store = Store::create(&path).unwrap();
    (dir, path, store)
}
fn unknown(store: &Store, vocabulary: Vocabulary) -> Id {
    store.settings().unwrap().values().find(|r|matches!(r.kind,Kind::Vocabulary {vocabulary:v,system_unknown:true} if v==vocabulary)).unwrap().id
}
fn setting(store: &mut Store, key: &str, action: settings::Action) {
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
fn brand(store: &mut Store) -> Id {
    let id = Id::new();
    setting(
        store,
        "brand",
        settings::Action::CreateVocabulary {
            id,
            vocabulary: Vocabulary::Manufacturer,
            name: "LEGO".into(),
        },
    );
    id
}
fn location(store: &mut Store, key: &str, name: &str) -> Id {
    let id = Id::new();
    setting(
        store,
        key,
        settings::Action::CreateLocation {
            id,
            parent_id: None,
            name: name.into(),
        },
    );
    id
}
fn details(manufacturer_id: Id, number: Option<&str>) -> Details {
    Details {
        expected_contents: Vec::new(),
        manufacturer_id,
        number: number.map(str::to_string),
        name: None,
        description: None,
        release_year: None,
        theme: None,
        stated_piece_count: None,
    }
}
fn command(key: &str, action: catalog::Action) -> catalog::Command {
    catalog::Command {
        id: Id::new(),
        request_key: key.into(),
        actor: "local-user".into(),
        reason: String::new(),
        action,
    }
}
fn define(store: &mut Store, key: &str, kind: catalog::Kind, details: Details) -> Id {
    let id = Id::new();
    store
        .execute_catalog(command(key, catalog::Action::Create { id, kind, details }))
        .unwrap();
    id
}
fn holding(store: &mut Store, key: &str) -> Id {
    let id = Id::new();
    store
        .commit(Operation {
            id: Id::new(),
            request_key: key.into(),
            actor: "local-user".into(),
            reason: "Synthetic receipt".into(),
            body: Body::Create {
                holding_id: id,
                pieces: Known(100),
                weight_mg: Known(1000),
            },
        })
        .unwrap();
    id
}
fn profile(store: &Store, key: &str, target: Target, revision: u64) -> profiles::Command {
    profiles::Command {
        id: Id::new(),
        request_key: key.into(),
        actor: "local-user".into(),
        reason: String::new(),
        target,
        expected_revision: revision,
        links: store.default_inventory_links().unwrap(),
    }
}

#[test]
fn definitions_create_no_stock_and_known_keys_are_unique_per_namespace() {
    let (_dir, path, mut store) = setup();
    let brand = brand(&mut store);
    let part = define(
        &mut store,
        "part",
        catalog::Kind::Part,
        details(brand, Some(" 3001 ")),
    );
    assert!(
        store
            .execute_catalog(command(
                "duplicate",
                catalog::Action::Create {
                    id: Id::new(),
                    kind: catalog::Kind::Part,
                    details: details(brand, Some("3001"))
                }
            ))
            .is_err()
    );
    define(
        &mut store,
        "set",
        catalog::Kind::Set,
        details(brand, Some("3001")),
    );
    assert!(store.rebuild().unwrap().holdings().is_empty());
    assert!(store.rebuild().unwrap().set_copies().is_empty());
    drop(store);
    let store = Store::open(path).unwrap();
    assert_eq!(
        store.catalog().unwrap()[&part].details.number.as_deref(),
        Some(" 3001 ")
    );
    assert_eq!(store.catalog().unwrap().len(), 2);
}

#[test]
fn unknown_and_incomplete_definitions_stay_separate() {
    let (_dir, _path, mut store) = setup();
    let unknown = unknown(&store, Vocabulary::Manufacturer);
    let brand = brand(&mut store);
    for (i, manufacturer, number) in [
        (0, unknown, Some("3001")),
        (1, unknown, Some("3001")),
        (2, brand, None),
        (3, brand, None),
    ] {
        define(
            &mut store,
            &format!("part-{i}"),
            catalog::Kind::Part,
            details(manufacturer, number),
        );
    }
    assert_eq!(store.catalog().unwrap().len(), 4);
    assert!(store.rebuild().unwrap().operations().is_empty());
}

#[test]
fn catalog_edits_are_idempotent_revision_checked_and_do_not_change_balances() {
    let (_dir, path, mut store) = setup();
    let brand = brand(&mut store);
    let id = Id::new();
    let create = command(
        "create",
        catalog::Action::Create {
            id,
            kind: catalog::Kind::Part,
            details: details(brand, Some("3001")),
        },
    );
    let original = store.execute_catalog(create.clone()).unwrap();
    let mut edit = original.after.details.clone();
    edit.name = Some("Brick 2 x 4".into());
    store
        .execute_catalog(command(
            "edit",
            catalog::Action::Edit {
                id,
                expected_revision: 1,
                details: edit.clone(),
            },
        ))
        .unwrap();
    assert!(
        store
            .execute_catalog(command(
                "stale",
                catalog::Action::Edit {
                    id,
                    expected_revision: 1,
                    details: edit
                }
            ))
            .is_err()
    );
    drop(store);
    let mut store = Store::open(path).unwrap();
    assert_eq!(store.execute_catalog(create.clone()).unwrap(), original);
    assert_eq!(store.catalog().unwrap()[&id].revision, 2);
    let history = store.catalog_history(id).unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(
        history[1].change.before.as_ref().unwrap().details.name,
        None
    );
    assert_eq!(
        history[1].change.after.details.name.as_deref(),
        Some("Brick 2 x 4")
    );
    assert!(history.iter().all(|r| r.committed_at.ends_with('Z')));
    let mut conflict = create;
    conflict.reason = "different".into();
    assert!(store.execute_catalog(conflict).is_err());
    assert!(store.rebuild().unwrap().operations().is_empty());
}

#[test]
fn links_and_whole_holding_moves_preserve_stock_and_survive_archive() {
    let (_dir, path, mut store) = setup();
    let brand = brand(&mut store);
    let part = define(
        &mut store,
        "part",
        catalog::Kind::Part,
        details(brand, Some("3001")),
    );
    let source = holding(&mut store, "receipt");
    let storage = location(&mut store, "storage", "Drawer 14");
    let mut assign = profile(&store, "assign", Target::Holding(source), 0);
    assign.links.definition_id = Some(part);
    assign.links.location_id = Some(storage);
    let original = store.execute_profile(assign.clone()).unwrap();
    store
        .execute_catalog(command(
            "archive-part",
            catalog::Action::Archive {
                id: part,
                expected_revision: 1,
                archived: true,
            },
        ))
        .unwrap();
    setting(
        &mut store,
        "archive-storage",
        settings::Action::Archive {
            id: storage,
            expected_revision: 1,
            archived: true,
        },
    );
    // Retaining archived links while changing another field is valid.
    let mut retained = assign.clone();
    retained.id = Id::new();
    retained.request_key = "retain".into();
    retained.expected_revision = 1;
    store.execute_profile(retained).unwrap();
    let other = holding(&mut store, "another-receipt");
    let mut new = profile(&store, "new-archived", Target::Holding(other), 0);
    new.links = original.after.links.clone();
    assert!(store.execute_profile(new).is_err());
    drop(store);
    let mut store = Store::open(path).unwrap();
    assert_eq!(store.execute_profile(assign).unwrap(), original);
    assert_eq!(
        store.inventory_profiles().unwrap()[&Target::Holding(source)]
            .links
            .location_id,
        Some(storage)
    );
    let shelf = location(&mut store, "shelf", "Shelf");
    let mut move_whole = profile(&store, "move", Target::Holding(source), 2);
    move_whole.links = original.after.links;
    move_whole.links.location_id = Some(shelf);
    store.execute_profile(move_whole).unwrap();
    assert_eq!(
        store.rebuild().unwrap().holdings()[&source].pieces.owned(),
        Known(100)
    );
    assert_eq!(store.rebuild().unwrap().operations().len(), 2);
}

#[test]
fn classification_inherits_location_once_and_child_can_move_independently() {
    let (_dir, path, mut store) = setup();
    let source = holding(&mut store, "receive");
    let drawer = location(&mut store, "drawer", "Drawer");
    let mut assign = profile(&store, "assign", Target::Holding(source), 0);
    assign.links.location_id = Some(drawer);
    store.execute_profile(assign).unwrap();
    setting(
        &mut store,
        "archive-drawer",
        settings::Action::Archive {
            id: drawer,
            expected_revision: 1,
            archived: true,
        },
    );
    let child = Id::new();
    let identify = Operation {
        id: Id::new(),
        request_key: "identify".into(),
        actor: "local-user".into(),
        reason: "Identify 20 pieces".into(),
        body: Body::Identify {
            source_id: source,
            child_id: child,
            expected_revision: 1,
            pieces: Some(20),
            weight_mg: None,
        },
    };
    let outcome = store.commit(identify.clone()).unwrap();
    let child_profile = store.inventory_profiles().unwrap()[&Target::Holding(child)].clone();
    assert_eq!(child_profile.links.location_id, Some(drawer));
    assert_eq!(child_profile.links.definition_id, None);
    assert_eq!(
        store.profile_history(Target::Holding(child)).unwrap()[0].related_operation_id,
        Some(identify.id)
    );
    let shelf = location(&mut store, "shelf", "Shelf");
    let mut move_child = profile(&store, "move-child", Target::Holding(child), 1);
    move_child.links.location_id = Some(shelf);
    store.execute_profile(move_child).unwrap();
    assert_eq!(store.commit(identify).unwrap(), outcome);
    drop(store);
    let store = Store::open(path).unwrap();
    let links = store.inventory_profiles().unwrap();
    assert_eq!(
        links[&Target::Holding(source)].links.location_id,
        Some(drawer)
    );
    assert_eq!(
        links[&Target::Holding(child)].links.location_id,
        Some(shelf)
    );
    let ledger = store.rebuild().unwrap();
    assert_eq!(ledger.holdings()[&source].pieces.owned(), Known(80));
    assert_eq!(ledger.holdings()[&child].pieces.owned(), Known(20));
    assert_eq!(ledger.holdings()[&source].weight_mg.owned(), Unknown);
}

#[test]
fn inherited_profile_failure_rolls_back_stock_transfer() {
    let (_dir, path, mut store) = setup();
    let source = holding(&mut store, "receive");
    store
        .execute_profile(profile(&store, "assign", Target::Holding(source), 0))
        .unwrap();
    let raw = Connection::open(&path).unwrap();
    raw.execute_batch("CREATE TRIGGER fail_profile BEFORE INSERT ON profile_command WHEN NEW.related_operation_id IS NOT NULL BEGIN SELECT RAISE(ABORT,'injected audit failure'); END;").unwrap();
    let child = Id::new();
    let request = Operation {
        id: Id::new(),
        request_key: "identify".into(),
        actor: "local-user".into(),
        reason: "Identify".into(),
        body: Body::Identify {
            source_id: source,
            child_id: child,
            expected_revision: 1,
            pieces: Some(10),
            weight_mg: None,
        },
    };
    assert!(store.commit(request.clone()).is_err());
    assert_eq!(
        store.rebuild().unwrap().holdings()[&source].pieces.owned(),
        Known(100)
    );
    assert_eq!(store.rebuild().unwrap().holdings().len(), 1);
    raw.execute_batch("DROP TRIGGER fail_profile").unwrap();
    store.commit(request).unwrap();
    assert_eq!(store.inventory_profiles().unwrap().len(), 2);
}

#[test]
fn wrong_vocabularies_missing_targets_and_wrong_namespaces_are_rejected() {
    let (_dir, _path, mut store) = setup();
    let color = unknown(&store, Vocabulary::Color);
    assert!(
        store
            .execute_catalog(command(
                "bad-manufacturer",
                catalog::Action::Create {
                    id: Id::new(),
                    kind: catalog::Kind::Part,
                    details: details(color, None)
                }
            ))
            .is_err()
    );
    let manufacturer = unknown(&store, Vocabulary::Manufacturer);
    let set = define(
        &mut store,
        "set",
        catalog::Kind::Set,
        details(manufacturer, None),
    );
    let source = holding(&mut store, "receive");
    let mut assign = profile(&store, "wrong-kind", Target::Holding(source), 0);
    assign.links.definition_id = Some(set);
    assert!(store.execute_profile(assign).is_err());
    let mut assign = profile(&store, "wrong-color", Target::Holding(source), 0);
    assign.links.color_id = manufacturer;
    assert!(store.execute_profile(assign).is_err());
    assert!(
        store
            .execute_profile(profile(
                &store,
                "missing-target",
                Target::Holding(Id::new()),
                0
            ))
            .is_err()
    );
    assert!(store.inventory_profiles().unwrap().is_empty());
}

#[test]
fn catalog_audit_failure_rolls_back_and_tampering_is_detected() {
    let (_dir, path, mut store) = setup();
    let manufacturer = unknown(&store, Vocabulary::Manufacturer);
    let raw = Connection::open(&path).unwrap();
    raw.execute_batch("CREATE TRIGGER fail_catalog BEFORE INSERT ON catalog_command BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
    let id = Id::new();
    let create = command(
        "create",
        catalog::Action::Create {
            id,
            kind: catalog::Kind::Part,
            details: details(manufacturer, None),
        },
    );
    assert!(store.execute_catalog(create.clone()).is_err());
    assert!(store.catalog().unwrap().is_empty());
    raw.execute_batch("DROP TRIGGER fail_catalog").unwrap();
    store.execute_catalog(create).unwrap();
    assert!(raw.execute("DELETE FROM catalog_command", []).is_err());
    raw.execute(
        "UPDATE catalog_definition SET archived=1 WHERE id=?1",
        [id.to_string()],
    )
    .unwrap();
    assert!(Store::open(path).is_err());
}

#[test]
fn concurrent_profile_edits_reject_stale_revision() {
    let (_dir, path, mut store) = setup();
    let source = holding(&mut store, "receive");
    let original = profile(&store, "assign", Target::Holding(source), 0);
    store.execute_profile(original).unwrap();
    let a = location(&mut store, "room", "Workshop");
    let b = location(&mut store, "shelf", "Shelf");
    let mut first = profile(&store, "first", Target::Holding(source), 1);
    first.links.location_id = Some(a);
    let mut second = profile(&store, "second", Target::Holding(source), 1);
    second.links.location_id = Some(b);
    drop(store);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let workers: Vec<_> = [first, second]
        .into_iter()
        .map(|command| {
            let path = path.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut store = Store::open(path).unwrap();
                barrier.wait();
                store.execute_profile(command).is_ok()
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
        Store::open(path).unwrap().rebuild().unwrap().holdings()[&source]
            .pieces
            .owned(),
        Known(100)
    );
}

#[test]
fn received_set_copies_link_to_set_definitions_and_move_without_creating_parts() {
    use brickxinator_core::{
        inventory::AssemblyState,
        purchases::{Draft, InvoiceLine, LineKind, PurchaseAction, PurchaseCommand},
    };
    let (_dir, path, mut store) = setup();
    let manufacturer = unknown(&store, Vocabulary::Manufacturer);
    let part = define(
        &mut store,
        "part",
        catalog::Kind::Part,
        details(manufacturer, Some("7838")),
    );
    let mut set_details = details(manufacturer, Some("7838"));
    set_details.stated_piece_count = Some(451);
    let set = define(&mut store, "set", catalog::Kind::Set, set_details);
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
                        kind: LineKind::Set,
                        description: None,
                        set_number: Some("7838".into()),
                        copy_count: Some(1),
                        pieces: Known(451),
                        weight_mg: Unknown,
                        paid_total_cents: None,
                        assembly: AssemblyState::Assembled,
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
    let shelf = location(&mut store, "shelf", "Shelf");
    let mut assign = profile(&store, "assign", Target::SetCopy(copy), 0);
    assign.links.definition_id = Some(part);
    assert!(store.execute_profile(assign.clone()).is_err());
    assign.links.definition_id = Some(set);
    assign.links.location_id = Some(shelf);
    store.execute_profile(assign).unwrap();
    drop(store);
    let store = Store::open(path).unwrap();
    assert_eq!(
        store.inventory_profiles().unwrap()[&Target::SetCopy(copy)]
            .links
            .definition_id,
        Some(set)
    );
    assert!(store.rebuild().unwrap().holdings().is_empty());
    assert_eq!(store.rebuild().unwrap().set_copies().len(), 1);
}

#[test]
fn schema_three_keeps_settings_without_silently_installing_catalog() {
    use sha2::{Digest, Sha256};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("schema-three.sqlite");
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
    assert_eq!(store.settings().unwrap().len(), 6);
    assert!(store.catalog().is_err());
    assert!(store.inventory_profiles().is_err());
    let raw = Connection::open(path).unwrap();
    assert_eq!(
        raw.query_row("SELECT count(*) FROM schema_migration", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        3
    );
}
