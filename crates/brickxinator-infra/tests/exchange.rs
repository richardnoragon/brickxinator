use brickxinator_core::{
    ledger::Id,
    purchases::{Draft, PurchaseAction, PurchaseCommand},
};
use brickxinator_infra::{attachments, exchange, sqlite::Store};

#[test]
fn native_round_trip_preserves_audit_assets_unknowns_and_idempotency() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source.sqlite");
    let target = root.path().join("target.sqlite");
    let mut store = Store::create(&source).unwrap();
    let purchase = Id::new();
    let command = PurchaseCommand {
        id: Id::new(),
        request_key: "create".into(),
        actor: "test".into(),
        reason: "native fixture".into(),
        body: PurchaseAction::Create {
            purchase_id: purchase,
            draft: Draft {
                seller: Some("=formula,\"quoted\"".into()),
                lines: vec![brickxinator_core::purchases::InvoiceLine {
                    source_url: None,
                    notes: String::new(),
                    id: Id::new(),
                    kind: brickxinator_core::purchases::LineKind::LooseBatch,
                    description: Some(" =formula,\"quoted\"".into()),
                    set_number: None,
                    copy_count: None,
                    pieces: brickxinator_core::inventory::Measure::Known(7),
                    weight_mg: brickxinator_core::inventory::Measure::Unknown,
                    paid_total_cents: None,
                    assembly: brickxinator_core::inventory::AssemblyState::Loose,
                }],
                ..Default::default()
            },
        },
    };
    let expected = store.execute_purchase(command.clone()).unwrap();
    let receipt_command = PurchaseCommand {
        id: Id::new(),
        request_key: "receive".into(),
        actor: "test".into(),
        reason: String::new(),
        body: PurchaseAction::Receive {
            purchase_id: purchase,
            expected_revision: 1,
        },
    };
    let receipt = store.execute_purchase(receipt_command.clone()).unwrap();
    let balances = store.rebuild().unwrap().holdings().clone();
    let attachment = store
        .attach(
            attachments::Command {
                id: Id::new(),
                request_key: "pdf".into(),
                actor: "test".into(),
                target: attachments::Target::Purchase(purchase),
                original_name: "invoice.pdf".into(),
            },
            b"%PDF-1.4 fixture",
        )
        .unwrap();
    let history = store.purchase_history(purchase).unwrap();
    drop(store);
    let bundle = root.path().join("native");
    let exported = exchange::export(&source, &bundle).unwrap();
    drop(Store::create(&target).unwrap());
    assert!(exchange::import(&target, &bundle, root.path().join("safety")).unwrap());
    let mut store = Store::open(&target).unwrap();
    assert_eq!(store.execute_purchase(command).unwrap(), expected);
    assert_eq!(store.execute_purchase(receipt_command).unwrap(), receipt);
    assert_eq!(store.rebuild().unwrap().holdings(), &balances);
    assert_eq!(store.purchase_history(purchase).unwrap(), history);
    assert_eq!(
        store.attachment_bytes(attachment.id).unwrap().1,
        b"%PDF-1.4 fixture"
    );
    drop(store);
    assert!(!exchange::import(&target, &bundle, root.path().join("safety")).unwrap());
    let reexported = exchange::export(&target, root.path().join("again")).unwrap();
    assert_eq!(exported, reexported);
    let archive = root.path().join("native.zip");
    exchange::export_archive(&target, &archive).unwrap();
    let third = root.path().join("third.sqlite");
    drop(Store::create(&third).unwrap());
    assert!(exchange::import_archive(&third, &archive, root.path().join("safety-third")).unwrap());
    assert_eq!(
        Store::open(&third).unwrap().rebuild().unwrap().holdings(),
        &balances
    );
    let csv = root.path().join("inventory.csv");
    exchange::inventory_csv(&target, &csv).unwrap();
    let csv = std::fs::read_to_string(csv).unwrap();
    assert!(csv.contains("\"' =formula,\"\"quoted\"\"\""));
    assert!(csv.contains("\"7\",\"\"\r\n"));
    let asset = bundle.join("assets").join(&attachment.relative_path);
    std::fs::write(&asset, b"%PDF-corrupted!").unwrap();
    assert!(exchange::import(&target, &bundle, root.path().join("safety")).is_err());
    std::fs::write(&asset, b"%PDF-1.4 fixture").unwrap();
    let path = bundle.join("collection.json");
    let mut native: exchange::Native =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    native.version = 99;
    std::fs::write(&path, serde_json::to_vec(&native).unwrap()).unwrap();
    assert!(exchange::import(&target, &bundle, root.path().join("safety")).is_err());
    assert_eq!(
        Store::open(&target)
            .unwrap()
            .purchase_history(purchase)
            .unwrap(),
        history
    );
}

#[test]
fn conflicting_collection_leaves_target_unchanged() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source.sqlite");
    let target = root.path().join("target.sqlite");
    for path in [&source, &target] {
        let mut store = Store::create(path).unwrap();
        store
            .execute_purchase(PurchaseCommand {
                id: Id::new(),
                request_key: "create".into(),
                actor: "test".into(),
                reason: String::new(),
                body: PurchaseAction::Create {
                    purchase_id: Id::new(),
                    draft: Draft::default(),
                },
            })
            .unwrap();
    }
    let before = Store::open(&target).unwrap().purchases().unwrap();
    exchange::export(&source, root.path().join("bundle")).unwrap();
    assert!(
        exchange::import(
            &target,
            root.path().join("bundle"),
            root.path().join("safety")
        )
        .is_err()
    );
    assert_eq!(Store::open(&target).unwrap().purchases().unwrap(), before);
}
