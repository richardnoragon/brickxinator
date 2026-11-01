use brickxinator_core::{
    ledger::Id,
    purchases::{Draft, PurchaseAction, PurchaseCommand},
};
use brickxinator_infra::{attachments, purge, sqlite::Store};

#[test]
fn entered_metadata_can_be_purged_but_retries_cannot_resurrect_it() {
    let root = tempfile::tempdir().unwrap();
    let db = root.path().join("database.sqlite");
    let mut store = Store::create(&db).unwrap();
    let id = Id::new();
    let create = PurchaseCommand {
        id: Id::new(),
        request_key: "create".into(),
        actor: "test".into(),
        reason: String::new(),
        body: PurchaseAction::Create {
            purchase_id: id,
            draft: Draft {
                seller: Some("private seller details".into()),
                notes: "private notes".into(),
                shipping_cents: Some(500),
                ..Default::default()
            },
        },
    };
    store.execute_purchase(create.clone()).unwrap();
    let command = purge::Command {
        id: Id::new(),
        purchase_id: id,
        expected_revision: 1,
        request_key: "purge".into(),
        actor: "test".into(),
    };
    store.purge_draft(command.clone()).unwrap();
    store.purge_draft(command.clone()).unwrap();
    assert!(store.purchase(id).unwrap().is_none());
    assert!(store.purchase_history(id).unwrap().is_empty());
    assert!(store.execute_purchase(create.clone()).is_err());
    let mut recycled = create;
    recycled.request_key = "new-request".into();
    assert!(store.execute_purchase(recycled).is_err());
    drop(store);
    let mut store = Store::open(&db).unwrap();
    store.purge_draft(command).unwrap();
    let connection = rusqlite::Connection::open(&db).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM purchase_command", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM purchase_deletion", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn attached_drafts_and_stale_purge_are_rejected_atomically() {
    let root = tempfile::tempdir().unwrap();
    let db = root.path().join("database.sqlite");
    let mut store = Store::create(&db).unwrap();
    let id = Id::new();
    store
        .execute_purchase(PurchaseCommand {
            id: Id::new(),
            request_key: "create".into(),
            actor: "test".into(),
            reason: String::new(),
            body: PurchaseAction::Create {
                purchase_id: id,
                draft: Draft::default(),
            },
        })
        .unwrap();
    let command = purge::Command {
        id: Id::new(),
        purchase_id: id,
        expected_revision: 0,
        request_key: "purge".into(),
        actor: "test".into(),
    };
    assert!(store.purge_draft(command.clone()).is_err());
    store
        .attach(
            attachments::Command {
                id: Id::new(),
                request_key: "attach".into(),
                actor: "test".into(),
                target: attachments::Target::Purchase(id),
                original_name: "invoice.pdf".into(),
            },
            b"%PDF-1.4 fixture",
        )
        .unwrap();
    assert!(
        store
            .purge_draft(purge::Command {
                expected_revision: 1,
                ..command
            })
            .is_err()
    );
    assert!(store.purchase(id).unwrap().is_some());
    assert_eq!(store.attachments().unwrap().len(), 1);
    drop(store);
    Store::open(db).unwrap();
}
