use brickxinator_core::{ledger::Id, projects::*};
use brickxinator_infra::{exchange, sqlite::Store};
fn cmd(revision: u64, action: Action) -> Command {
    Command {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "project regression".into(),
        expected_revision: revision,
        action,
    }
}
#[test]
fn project_history_reopens_retries_and_round_trips_native_exchange() {
    let root = tempfile::tempdir().unwrap();
    let db = root.path().join("collection.sqlite");
    let mut store = Store::create(&db).unwrap();
    let v = Version {
        id: Id::new(),
        design_id: Id::new(),
        label: "MOC v1".into(),
        author: "Author".into(),
        notes: "Unbuilt".into(),
        source: "design.ldr".into(),
        requirements: vec![],
    };
    let create = cmd(0, Action::AddVersion(v.clone()));
    let first = store.execute_project(create.clone()).unwrap();
    assert_eq!(store.execute_project(create.clone()).unwrap(), first);
    let mut changed = create.clone();
    changed.reason = "different".into();
    assert!(store.execute_project(changed).is_err());
    let p = Project {
        id: Id::new(),
        name: "Diorama".into(),
        kind: Kind::Diorama,
        version_id: v.id,
        copy_id: None,
        progress: Progress::Planned,
        notes: String::new(),
    };
    store
        .execute_project(cmd(1, Action::Create(p.clone())))
        .unwrap();
    let snapshot = store.projects().unwrap();
    drop(store);
    assert_eq!(Store::open(&db).unwrap().projects().unwrap(), snapshot);
    let export = root.path().join("native");
    exchange::export(&db, &export).unwrap();
    let destination = root.path().join("import.sqlite");
    drop(Store::create(&destination).unwrap());
    exchange::import(&destination, &export, root.path().join("safety.zip")).unwrap();
    assert_eq!(
        Store::open(&destination).unwrap().projects().unwrap(),
        snapshot
    );
}
#[test]
fn project_mutations_remain_immutable_and_failed_commands_leave_no_audit() {
    let root = tempfile::tempdir().unwrap();
    let db = root.path().join("collection.sqlite");
    let mut s = Store::create(&db).unwrap();
    let bad = cmd(
        0,
        Action::Create(Project {
            id: Id::new(),
            name: "Bad".into(),
            kind: Kind::Moc,
            version_id: Id::new(),
            copy_id: None,
            progress: Progress::Planned,
            notes: String::new(),
        }),
    );
    assert!(s.execute_project(bad).is_err());
    assert_eq!(s.projects().unwrap()["state"]["revision"], 0);
    let connection = rusqlite::Connection::open(&db).unwrap();
    s.execute_project(cmd(
        0,
        Action::AddVersion(Version {
            id: Id::new(),
            design_id: Id::new(),
            label: "v1".into(),
            author: String::new(),
            notes: String::new(),
            source: String::new(),
            requirements: vec![],
        }),
    ))
    .unwrap();
    assert!(
        connection
            .execute("UPDATE project_command SET result_json='{}'", [])
            .is_err()
    );
    assert!(
        connection
            .execute("DELETE FROM project_command", [])
            .is_err()
    );
}

#[test]
fn physical_stock_commands_cannot_consume_reserved_stock_and_views_agree() {
    use brickxinator_core::{
        catalog,
        inventory::Measure,
        ledger::{Body, Operation},
        profiles, settings, stock,
    };
    let root = tempfile::tempdir().unwrap();
    let db = root.path().join("collection.sqlite");
    let mut s = Store::create(&db).unwrap();
    let manufacturer = s
        .settings()
        .unwrap()
        .values()
        .find(|r| {
            matches!(
                r.kind,
                settings::Kind::Vocabulary {
                    vocabulary: settings::Vocabulary::Manufacturer,
                    system_unknown: true
                }
            )
        })
        .unwrap()
        .id;
    let part = Id::new();
    let holding = Id::new();
    s.execute_catalog(catalog::Command {
        id: Id::new(),
        request_key: "part".into(),
        actor: "test".into(),
        reason: "fixture".into(),
        action: catalog::Action::Create {
            id: part,
            kind: catalog::Kind::Part,
            details: serde_json::from_value(
                serde_json::json!({"manufacturer_id":manufacturer,"number":"3001","name":"Brick"}),
            )
            .unwrap(),
        },
    })
    .unwrap();
    s.commit(Operation {
        id: Id::new(),
        request_key: "stock".into(),
        actor: "test".into(),
        reason: "counted physical stock".into(),
        body: Body::Create {
            holding_id: holding,
            pieces: Measure::Known(100),
            weight_mg: Measure::Unknown,
        },
    })
    .unwrap();
    let mut links = s.default_inventory_links().unwrap();
    links.definition_id = Some(part);
    let color = links.color_id;
    s.execute_profile(profiles::Command {
        id: Id::new(),
        request_key: "link".into(),
        actor: "test".into(),
        reason: "identified".into(),
        target: profiles::Target::Holding(holding),
        expected_revision: 0,
        links,
    })
    .unwrap();
    let v = Id::new();
    let p = Id::new();
    let r = Id::new();
    let claim = Id::new();
    s.execute_project(cmd(
        0,
        Action::AddVersion(Version {
            id: v,
            design_id: Id::new(),
            label: "v1".into(),
            author: String::new(),
            notes: String::new(),
            source: String::new(),
            requirements: vec![Requirement {
                id: r,
                definition_id: part,
                color_id: color,
                quantity: 60,
                unit_estimate_cents: None,
                price_snapshot_id: None,
            }],
        }),
    ))
    .unwrap();
    s.execute_project(cmd(
        1,
        Action::Create(Project {
            id: p,
            name: "MOC".into(),
            kind: Kind::Moc,
            version_id: v,
            copy_id: None,
            progress: Progress::Planned,
            notes: String::new(),
        }),
    ))
    .unwrap();
    s.execute_project(cmd(
        2,
        Action::Claim(Claim {
            id: claim,
            project_id: p,
            requirement_id: r,
            holding_id: holding,
            quantity: 60,
            kind: ClaimKind::Reserved,
            verified_revision: None,
        }),
    ))
    .unwrap();
    assert_eq!(
        s.dashboard()
            .unwrap()
            .inventory
            .all
            .available_pieces
            .known_sum,
        40
    );
    let before = s.rebuild().unwrap().operations().len();
    let revision = s.rebuild().unwrap().holdings()[&holding].revision;
    let adjustment = |count, key: &str| stock::Command {
        id: Id::new(),
        request_key: key.into(),
        actor: "test".into(),
        reason: "sold loose stock".into(),
        holding_id: holding,
        expected_revision: revision,
        action: stock::Action::Adjust(stock::Adjustment::Remove {
            kind: stock::Removal::Sold,
            pieces: Some(count),
            weight_mg: None,
            damaged_pieces: 0,
            damaged_weight_mg: 0,
        }),
    };
    assert!(s.execute_stock(adjustment(41, "over-sale")).is_err());
    assert_eq!(s.rebuild().unwrap().operations().len(), before);
    s.execute_stock(adjustment(40, "sale")).unwrap();
    assert_eq!(
        s.dashboard()
            .unwrap()
            .inventory
            .all
            .available_pieces
            .known_sum,
        0
    );
    assert_eq!(
        s.dashboard().unwrap().inventory.all.owned_pieces.known_sum,
        60
    );
    let snapshot = s.projects().unwrap();
    drop(s);
    assert_eq!(Store::open(&db).unwrap().projects().unwrap(), snapshot);
    let export = root.path().join("native");
    exchange::export(&db, &export).unwrap();
    let dest = root.path().join("import.sqlite");
    drop(Store::create(&dest).unwrap());
    exchange::import(&dest, &export, root.path().join("safe.zip")).unwrap();
    assert_eq!(Store::open(&dest).unwrap().projects().unwrap(), snapshot);
}
