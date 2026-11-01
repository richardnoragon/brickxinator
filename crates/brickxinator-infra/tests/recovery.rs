use brickxinator_core::{
    ledger::Id,
    purchases::{Draft, PurchaseAction, PurchaseCommand, Status},
};
use brickxinator_infra::{
    attachments::{Command, Target},
    maintenance::Maintenance,
    recovery, snapshots,
    sqlite::Store,
};
use rusqlite::Connection;
use std::fs;

fn setup() -> (tempfile::TempDir, std::path::PathBuf, Id) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("database.sqlite");
    let mut store = Store::create(&db).unwrap();
    let id = Id::new();
    store
        .execute_purchase(PurchaseCommand {
            id: Id::new(),
            request_key: "draft".into(),
            actor: "local-user".into(),
            reason: String::new(),
            body: PurchaseAction::Create {
                purchase_id: id,
                draft: Draft {
                    seller: Some("Before restore".into()),
                    ..Draft::default()
                },
            },
        })
        .unwrap();
    drop(store);
    (dir, db, id)
}
#[test]
fn attachment_upload_is_immutable_idempotent_and_original_name_cannot_escape() {
    let (_dir, db, purchase) = setup();
    let mut store = Store::open(&db).unwrap();
    let command = Command {
        id: Id::new(),
        request_key: "image".into(),
        actor: "local-user".into(),
        target: Target::Purchase(purchase),
        original_name: "../../picture.png".into(),
    };
    let bytes = b"\x89PNG\r\n\x1a\nsynthetic image";
    let record = store.attach(command.clone(), bytes).unwrap();
    assert_eq!(store.attach(command.clone(), bytes).unwrap(), record);
    assert_eq!(store.attachment_bytes(record.id).unwrap().1, bytes);
    assert_eq!(store.attachments().unwrap().len(), 1);
    assert!(store.attach(command, b"%PDF-changed").is_err());
    assert!(
        store
            .attach(
                Command {
                    id: Id::new(),
                    request_key: "html".into(),
                    actor: "local-user".into(),
                    target: Target::Purchase(purchase),
                    original_name: "image.png".into()
                },
                b"<svg onload='x'></svg>"
            )
            .is_err()
    );
    assert!(
        !db.parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("picture.png")
            .exists()
    );
}
#[test]
fn live_leases_exclude_maintenance_without_expiration() {
    let (_dir, db, _) = setup();
    let store = Store::open(&db).unwrap();
    assert!(Maintenance::acquire(&db).is_err());
    drop(store);
    let maintenance = Maintenance::acquire(&db).unwrap();
    assert!(Store::open(&db).is_err());
    drop(maintenance);
    Store::open(&db).unwrap();
}

#[test]
fn restored_machine_configuration_is_separate_and_never_reactivates_old_paths() {
    let (dir, db, _purchase) = setup();
    let original = brickxinator_infra::backups::initialize(dir.path(), dir.path()).unwrap();
    let config_path = brickxinator_infra::backups::config_path(dir.path());
    let archive = dir.path().join("with-config.zip");
    let manifest = snapshots::snapshot(&db, &archive, "manual", Some(&config_path)).unwrap();
    assert!(manifest.files.contains_key("config.json"));
    assert!(
        !manifest
            .files
            .keys()
            .any(|name| name.contains("restic-password"))
    );
    let mut current = original;
    current.destinations[0].daily = dir.path().join("new-machine/daily");
    current.destinations[0].monthly = dir.path().join("new-machine/monthly");
    current.destinations[0].enabled = true;
    brickxinator_infra::backups::save(dir.path(), &current).unwrap();
    let active = fs::read(&config_path).unwrap();
    let report = recovery::restore(&db, archive, dir.path().join("safety")).unwrap();
    assert_eq!(fs::read(config_path).unwrap(), active);
    let restored: brickxinator_infra::backups::Config =
        serde_json::from_slice(&fs::read(report.config_for_review.unwrap()).unwrap()).unwrap();
    assert!(!restored.destinations[0].enabled);
    assert_ne!(
        restored.destinations[0].daily,
        current.destinations[0].daily
    );
}

#[test]
fn interrupted_restore_rolls_back_at_each_file_install_stage() {
    for phase in 0..4 {
        let (dir, db, purchase) = setup();
        let rollback_name = format!(".brickxinator-rollback-{}", Id::new());
        let rollback = dir.path().join(&rollback_name);
        fs::create_dir(&rollback).unwrap();
        let journal = std::path::PathBuf::from(format!("{}.restore.json", db.display()));
        fs::write(&journal,serde_json::to_vec(&serde_json::json!({"rollback_directory":rollback_name,"had_assets":false,"committed":phase==3})).unwrap()).unwrap();
        if phase > 0 {
            fs::rename(&db, rollback.join("database.sqlite")).unwrap();
        }
        if phase > 1 {
            fs::copy(rollback.join("database.sqlite"), &db).unwrap();
        }
        assert!(Store::open(&db).is_err());
        assert!(recovery::recover_interrupted(&db).unwrap());
        assert!(!journal.exists());
        assert!(!rollback.exists());
        assert!(
            Store::open(&db)
                .unwrap()
                .purchase(purchase)
                .unwrap()
                .is_some()
        );
        assert!(!recovery::recover_interrupted(&db).unwrap());
    }
}

#[test]
fn archive_traversal_and_future_format_are_rejected() {
    use std::io::Write;
    let (dir, db, _) = setup();
    let original = dir.path().join("original.zip");
    snapshots::snapshot(&db, &original, "test", None).unwrap();
    for (name, future) in [("../escaped", false), ("database.sqlite", true)] {
        let path = dir.path().join(if future {
            "future.zip"
        } else {
            "traversal.zip"
        });
        let mut input = zip::ZipArchive::new(fs::File::open(&original).unwrap()).unwrap();
        let mut metadata: serde_json::Value =
            serde_json::from_reader(input.by_name("metadata.json").unwrap()).unwrap();
        if future {
            metadata["version"] = serde_json::json!(999);
        }
        let mut output = zip::ZipWriter::new(fs::File::create(&path).unwrap());
        output
            .start_file("metadata.json", zip::write::SimpleFileOptions::default())
            .unwrap();
        output
            .write_all(&serde_json::to_vec(&metadata).unwrap())
            .unwrap();
        output
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        output.write_all(b"invalid").unwrap();
        output.finish().unwrap();
        assert!(snapshots::validate(path).is_err());
    }
    assert!(!dir.path().parent().unwrap().join("escaped").exists());
    Store::open(db).unwrap();
}
#[test]
fn snapshot_restore_and_safety_undo_preserve_attachments_and_history() {
    let (dir, db, purchase) = setup();
    let mut store = Store::open(&db).unwrap();
    let record = store
        .attach(
            Command {
                id: Id::new(),
                request_key: "pdf".into(),
                actor: "local-user".into(),
                target: Target::Purchase(purchase),
                original_name: "invoice.pdf".into(),
            },
            b"%PDF-1.4 synthetic invoice",
        )
        .unwrap();
    drop(store);
    let archive = dir.path().join("monthly.zip");
    let manifest = snapshots::snapshot(&db, &archive, "monthly", None).unwrap();
    assert!(
        manifest
            .files
            .contains_key(&format!("attachments/{}", record.relative_path))
    );
    snapshots::validate(&archive).unwrap();
    let mut store = Store::open(&db).unwrap();
    let mut draft = store.purchase(purchase).unwrap().unwrap().draft;
    draft.seller = Some("After snapshot".into());
    store
        .execute_purchase(PurchaseCommand {
            id: Id::new(),
            request_key: "edit".into(),
            actor: "local-user".into(),
            reason: String::new(),
            body: PurchaseAction::Save {
                purchase_id: purchase,
                expected_revision: 1,
                status: Status::Ordered,
                draft,
            },
        })
        .unwrap();
    drop(store);
    let restored = recovery::restore(&db, &archive, dir.path().join("safety")).unwrap();
    let store = Store::open(&db).unwrap();
    assert_eq!(
        store
            .purchase(purchase)
            .unwrap()
            .unwrap()
            .draft
            .seller
            .as_deref(),
        Some("Before restore")
    );
    assert_eq!(
        store.attachment_bytes(record.id).unwrap().1,
        b"%PDF-1.4 synthetic invoice"
    );
    drop(store);
    recovery::restore(&db, &restored.safety_snapshot, dir.path().join("safety")).unwrap();
    let store = Store::open(&db).unwrap();
    assert_eq!(
        store
            .purchase(purchase)
            .unwrap()
            .unwrap()
            .draft
            .seller
            .as_deref(),
        Some("After snapshot")
    );
}
#[test]
fn corrupted_snapshot_is_rejected_before_live_data_changes() {
    let (dir, db, purchase) = setup();
    let bad = dir.path().join("bad.zip");
    fs::write(&bad, b"not a ZIP").unwrap();
    assert!(recovery::restore(&db, &bad, dir.path().join("safety")).is_err());
    assert_eq!(
        Store::open(&db)
            .unwrap()
            .purchase(purchase)
            .unwrap()
            .unwrap()
            .revision,
        1
    );
}
#[test]
fn old_schema_upgrade_creates_valid_safety_point_and_keeps_data() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("old.sqlite");
    let raw = Connection::open(&db).unwrap();
    let migrations = [
        (
            "migration_001.sql",
            include_str!("../../../migrations/migration_001.sql"),
        ),
        (
            "migration_002.sql",
            include_str!("../../../migrations/migration_002.sql"),
        ),
    ];
    use sha2::{Digest, Sha256};
    for (index, (name, sql)) in migrations.into_iter().enumerate() {
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
    let mut store = Store::open(&db).unwrap();
    let id = Id::new();
    store
        .execute_purchase(PurchaseCommand {
            id: Id::new(),
            request_key: "legacy".into(),
            actor: "local-user".into(),
            reason: String::new(),
            body: PurchaseAction::Create {
                purchase_id: id,
                draft: Draft::default(),
            },
        })
        .unwrap();
    drop(store);
    let raw = Connection::open(&db).unwrap();
    raw.execute_batch("CREATE TRIGGER injected_upgrade_failure BEFORE INSERT ON schema_migration WHEN NEW.version=9 BEGIN SELECT RAISE(ABORT,'injected upgrade failure'); END;").unwrap();
    drop(raw);
    assert!(recovery::upgrade(&db, dir.path().join("failed-upgrade-safety")).is_err());
    let raw = Connection::open(&db).unwrap();
    assert_eq!(
        raw.query_row("SELECT max(version) FROM schema_migration", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        raw.query_row(
            "SELECT count(*) FROM sqlite_master WHERE name='catalog_definition'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    raw.execute_batch("DROP TRIGGER injected_upgrade_failure")
        .unwrap();
    drop(raw);
    let report = recovery::upgrade(&db, dir.path().join("safety")).unwrap();
    assert_eq!(report.previous_schema, 2);
    assert_eq!(
        snapshots::validate(report.safety_snapshot.unwrap())
            .unwrap()
            .manifest
            .schema_version,
        2
    );
    assert!(Store::open(&db).unwrap().purchase(id).unwrap().is_some());
    assert!(
        recovery::upgrade(&db, dir.path().join("safety"))
            .unwrap()
            .safety_snapshot
            .is_none()
    );
}
