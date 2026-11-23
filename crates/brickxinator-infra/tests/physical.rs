use brickxinator_core::{
    catalog,
    inventory::Measure,
    ledger::{Body, Id, Operation},
    physical::*,
    profiles,
    purchases::{Draft, PurchaseAction, PurchaseCommand},
    settings::{Kind, Vocabulary},
    verification,
};
use brickxinator_infra::{exchange, sqlite::Store};
fn op(body: Body) -> Operation {
    Operation {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "reviewed physical workflow".into(),
        body,
    }
}
fn execute(s: &mut Store, o: Operation) {
    let p = s.preview_physical(o.clone()).unwrap();
    s.execute_physical(o, p["expected_profile_sequence"].as_i64().unwrap())
        .unwrap();
}
fn fixture() -> (tempfile::TempDir, std::path::PathBuf, Store, Id, Id) {
    let root = tempfile::tempdir().unwrap();
    let db = root.path().join("collection.sqlite");
    let mut s = Store::create(&db).unwrap();
    let part = Id::new();
    let manufacturer = s
        .settings()
        .unwrap()
        .values()
        .find(|r| {
            matches!(
                r.kind,
                Kind::Vocabulary {
                    vocabulary: Vocabulary::Manufacturer,
                    system_unknown: true
                }
            )
        })
        .unwrap()
        .id;
    s.execute_catalog(catalog::Command {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "part identity".into(),
        action: catalog::Action::Create {
            id: part,
            kind: catalog::Kind::Part,
            details: serde_json::from_value(
                serde_json::json!({"manufacturer_id":manufacturer,"name":"Brick","number":"3001"}),
            )
            .unwrap(),
        },
    })
    .unwrap();
    let purchase = Id::new();
    let line = Id::new();
    let draft:Draft=serde_json::from_value(serde_json::json!({"notes":"","lines":[{"id":line,"kind":"Set","description":"Test set","set_number":"10294","copy_count":1,"pieces":"Unknown","weight_mg":"Unknown","paid_total_cents":20000,"assembly":"Assembled"}]})).unwrap();
    s.execute_purchase(PurchaseCommand {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "purchase".into(),
        body: PurchaseAction::Create {
            purchase_id: purchase,
            draft,
        },
    })
    .unwrap();
    let receipt = s
        .execute_purchase(PurchaseCommand {
            id: Id::new(),
            request_key: Id::new().to_string(),
            actor: "test".into(),
            reason: "received".into(),
            body: PurchaseAction::Receive {
                purchase_id: purchase,
                expected_revision: 1,
            },
        })
        .unwrap();
    let copy = *receipt.receipt.unwrap().set_copies.keys().next().unwrap();
    let h = Id::new();
    let vline = Id::new();
    let color = s.default_inventory_links().unwrap().color_id;
    s.execute_verification(verification::Command {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "checklist".into(),
        copy_id: copy,
        expected_revision: 0,
        action: verification::Action::AddLine {
            line_id: vline,
            holding_id: h,
            definition_id: Some(part),
            color_id: color,
            expected: Some(100),
        },
    })
    .unwrap();
    s.execute_verification(verification::Command {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "physically counted".into(),
        copy_id: copy,
        expected_revision: 1,
        action: verification::Action::Count {
            line_id: vline,
            total: 100,
        },
    })
    .unwrap();
    (root, db, s, copy, h)
}
#[test]
fn detach_split_merge_and_native_restore_preserve_quantities_identity_and_provenance() {
    let (root, db, mut s, _copy, h) = fixture();
    let before = s.dashboard().unwrap().inventory.all.owned_pieces.known_sum;
    let rev = s.rebuild().unwrap().holdings()[&h].revision;
    execute(
        &mut s,
        op(Body::Detach {
            holding_id: h,
            expected_revision: rev,
        }),
    );
    let child = Id::new();
    let rev = s.rebuild().unwrap().holdings()[&h].revision;
    execute(
        &mut s,
        op(Body::Split {
            source_id: h,
            child_id: child,
            expected_revision: rev,
            portion: Portion {
                pieces: Some(40),
                weight_mg: None,
                damaged_pieces: Some(0),
                damaged_weight_mg: None,
            },
        }),
    );
    assert_eq!(
        s.dashboard().unwrap().inventory.all.owned_pieces.known_sum,
        before
    );
    let profile = s.inventory_profiles().unwrap();
    assert_eq!(
        profile[&profiles::Target::Holding(h)].links.definition_id,
        profile[&profiles::Target::Holding(child)]
            .links
            .definition_id
    );
    let ledger = s.rebuild().unwrap();
    let merged = Id::new();
    execute(
        &mut s,
        op(Body::Merge {
            sources: vec![
                Source {
                    holding_id: h,
                    expected_revision: ledger.holdings()[&h].revision,
                },
                Source {
                    holding_id: child,
                    expected_revision: ledger.holdings()[&child].revision,
                },
            ],
            target_id: merged,
        }),
    );
    assert_eq!(
        s.rebuild().unwrap().holdings()[&merged].pieces.owned(),
        Measure::Known(100)
    );
    assert_eq!(
        s.dashboard().unwrap().inventory.all.owned_pieces.known_sum,
        before
    );
    drop(s);
    let view = Store::open(&db).unwrap().physical_view().unwrap();
    assert_eq!(view["flow"].as_array().unwrap().len(), 3);
    let native = root.path().join("native");
    exchange::export(&db, &native).unwrap();
    let dest = root.path().join("import.sqlite");
    drop(Store::create(&dest).unwrap());
    exchange::import(&dest, &native, root.path().join("safe.zip")).unwrap();
    assert_eq!(Store::open(&dest).unwrap().physical_view().unwrap(), view);
}
#[test]
fn sold_copy_retains_identity_and_safe_correction_restores_stock_once() {
    let (_root, db, mut s, copy, h) = fixture();
    let rev = s.rebuild().unwrap().holdings()[&h].revision;
    let sale = op(Body::Lifecycle(Lifecycle {
        copy_id: copy,
        expected_revision: 0,
        status: Status::Sold,
        effective_on: "2026-11-23".into(),
        proceeds_cents: Some(45000),
        recipient: None,
        notes: "complete sale".into(),
        parts: vec![Disposition {
            holding_id: h,
            expected_revision: rev,
            handling: Handling::TransferAll,
        }],
        corrects: None,
    }));
    let preview = s.preview_physical(sale.clone()).unwrap();
    let result = s
        .execute_physical(
            sale.clone(),
            preview["expected_profile_sequence"].as_i64().unwrap(),
        )
        .unwrap();
    assert_eq!(s.execute_physical(sale.clone(), 0).unwrap(), result);
    assert_eq!(s.dashboard().unwrap().inventory.known_set_copies, 0);
    assert_eq!(
        s.dashboard().unwrap().inventory.all.owned_pieces.known_sum,
        0
    );
    let undo = op(Body::Lifecycle(Lifecycle {
        copy_id: copy,
        expected_revision: 1,
        status: Status::Active,
        effective_on: "2026-11-23".into(),
        proceeds_cents: None,
        recipient: None,
        notes: "mistaken sale".into(),
        parts: vec![],
        corrects: Some(sale.id),
    }));
    execute(&mut s, undo);
    assert_eq!(s.dashboard().unwrap().inventory.known_set_copies, 1);
    assert_eq!(
        s.dashboard().unwrap().inventory.all.owned_pieces.known_sum,
        100
    );
    drop(s);
    assert_eq!(
        Store::open(&db)
            .unwrap()
            .rebuild()
            .unwrap()
            .lifecycle(copy)
            .status,
        Status::Active
    );
}
#[test]
fn keep_all_remains_owned_and_later_metadata_changes_block_unsafe_reversal() {
    let (_root, _db, mut s, copy, h) = fixture();
    let rev = s.rebuild().unwrap().holdings()[&h].revision;
    let sale = op(Body::Lifecycle(Lifecycle {
        copy_id: copy,
        expected_revision: 0,
        status: Status::Void,
        effective_on: "2026-11-23".into(),
        proceeds_cents: None,
        recipient: None,
        notes: "erroneous copy".into(),
        parts: vec![Disposition {
            holding_id: h,
            expected_revision: rev,
            handling: Handling::KeepAll,
        }],
        corrects: None,
    }));
    execute(&mut s, sale.clone());
    assert_eq!(
        s.dashboard().unwrap().inventory.all.owned_pieces.known_sum,
        100
    );
    assert_eq!(
        s.dashboard()
            .unwrap()
            .inventory
            .all
            .available_pieces
            .known_sum,
        100
    );
    let before = s.inventory_profiles().unwrap()[&profiles::Target::Holding(h)].clone();
    let mut links = before.links;
    links.archived = true;
    s.execute_profile(profiles::Command {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "later profile change".into(),
        target: profiles::Target::Holding(h),
        expected_revision: before.revision,
        links,
    })
    .unwrap();
    let undo = op(Body::Lifecycle(Lifecycle {
        copy_id: copy,
        expected_revision: 1,
        status: Status::Active,
        effective_on: "2026-11-23".into(),
        proceeds_cents: None,
        recipient: None,
        notes: "undo".into(),
        parts: vec![],
        corrects: Some(sale.id),
    }));
    let seq = s.physical_view().unwrap()["profile_sequence"]
        .as_i64()
        .unwrap();
    assert!(s.preview_physical(undo.clone()).is_err());
    assert!(s.execute_physical(undo, seq).is_err());
    assert_eq!(s.rebuild().unwrap().lifecycle(copy).status, Status::Void);
}
