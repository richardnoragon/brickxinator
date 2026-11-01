use brickxinator_infra::{backups, snapshots, sqlite::Store};

fn fixture() -> (tempfile::TempDir, std::path::PathBuf, backups::Config) {
    let root = tempfile::tempdir().unwrap();
    let db = root.path().join("database.sqlite");
    drop(Store::create(&db).unwrap());
    let config = backups::initialize(root.path(), root.path()).unwrap();
    (root, db, config)
}

#[test]
fn defaults_are_disabled_and_password_is_private() {
    let (root, _, config) = fixture();
    assert!(!config.destinations[0].enabled);
    assert_eq!(
        config.destinations[0].monthly,
        root.path().join("backups/montly")
    );
    assert_eq!((config.hour, config.minute), (3, 0));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(config.password_file)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}

#[test]
fn local_schedule_boundaries_keep_daily_and_monthly_independent() {
    let (_root, _db, config) = fixture();
    for (time, day, month) in [
        ("2026-11-01T02:59:00", "2026-10-31", "2026-10"),
        ("2026-11-01T03:00:00", "2026-11-01", "2026-11"),
        ("2026-11-02T02:00:00", "2026-11-01", "2026-11"),
        ("2027-03-15T12:00:00", "2027-03-15", "2027-03"),
    ] {
        let now = chrono::NaiveDateTime::parse_from_str(time, "%Y-%m-%dT%H:%M:%S").unwrap();
        assert_eq!(
            backups::due_periods(&config, now).unwrap(),
            (day.into(), month.into())
        );
    }
}

#[test]
fn never_completed_backups_are_visible_and_external_types_require_mounts() {
    let (root, _db, mut config) = fixture();
    config.destinations[0].enabled = true;
    let view = backups::status_view(root.path(), &config).unwrap();
    assert!(view[0].daily_error.is_some());
    assert!(view[0].monthly_error.is_some());
    config.destinations[0].kind = backups::DestinationKind::ExternalDrive;
    assert!(backups::save(root.path(), &config).is_err());
    config.destinations[0].required_mount = Some(root.path().join("absent"));
    backups::save(root.path(), &config).unwrap();
}

#[test]
fn an_unavailable_destination_does_not_skip_other_monthly_destinations() {
    let (root, db, mut config) = fixture();
    let mut second = config.destinations[0].clone();
    second.id = brickxinator_core::ledger::Id::new();
    second.enabled = true;
    second.daily = root.path().join("second/daily");
    second.monthly = root.path().join("second/monthly");
    config.destinations[0].enabled = true;
    config.destinations[0].required_mount = Some(root.path().join("missing-mount"));
    config.destinations.push(second.clone());
    config.restic = root.path().join("missing-restic");
    backups::save(root.path(), &config).unwrap();
    assert!(backups::run(root.path(), &db, true).is_err());
    let states = backups::status(root.path()).unwrap();
    assert_eq!(states.len(), 2);
    assert!(
        states
            .iter()
            .find(|s| s.destination == Some(second.id))
            .unwrap()
            .last_monthly_success
            .is_some()
    );
    assert!(!config.destinations[0].monthly.exists());
}

#[test]
fn missing_mount_creates_no_destination_and_records_both_failures() {
    let (root, db, mut config) = fixture();
    config.destinations[0].enabled = true;
    config.destinations[0].required_mount = Some(root.path().join("absent-mount"));
    backups::save(root.path(), &config).unwrap();
    assert!(backups::run(root.path(), &db, true).is_err());
    assert!(!config.destinations[0].daily.exists());
    assert!(!config.destinations[0].monthly.exists());
    let status = backups::status(root.path()).unwrap();
    assert!(status[0].daily_error.is_some());
    assert!(status[0].monthly_error.is_some());
    assert!(status[0].last_daily_success.is_none());
}

#[test]
fn monthly_is_independent_of_missing_restic_and_has_valid_config_snapshot() {
    let (root, db, mut config) = fixture();
    // Exercise a genuinely mounted directory without mounting/unmounting a drive.
    config.destinations[0].required_mount = Some(std::path::PathBuf::from("/"));
    config.destinations[0].enabled = true;
    config.restic = root.path().join("missing-restic");
    backups::save(root.path(), &config).unwrap();
    assert!(backups::run(root.path(), &db, true).is_err());
    let status = backups::status(root.path()).unwrap();
    assert!(status[0].daily_error.is_some());
    assert!(status[0].last_monthly_success.is_some());
    let files: Vec<_> = std::fs::read_dir(&config.destinations[0].monthly)
        .unwrap()
        .map(|f| f.unwrap().path())
        .collect();
    assert_eq!(files.len(), 1);
    let checked = snapshots::validate(&files[0]).unwrap();
    assert_eq!(checked.manifest.kind, "monthly");
    assert!(checked.config().is_some());
    // Failed daily retries must not manufacture a second monthly point.
    assert!(backups::run(root.path(), &db, false).is_err());
    assert_eq!(
        std::fs::read_dir(&config.destinations[0].monthly)
            .unwrap()
            .count(),
        1
    );
}

#[test]
#[ignore = "requires BRICKXINATOR_TEST_RESTIC pointing to a real Restic executable"]
fn real_restic_restores_validated_daily_snapshot() {
    use brickxinator_core::{
        inventory::Measure,
        ledger::{Body, Id, Operation},
        purchases::{Draft, PurchaseAction, PurchaseCommand},
    };
    let (root, db, mut config) = fixture();
    config.destinations[0].enabled = true;
    config.restic = std::env::var_os("BRICKXINATOR_TEST_RESTIC")
        .expect("Restic path")
        .into();
    backups::save(root.path(), &config).unwrap();
    let purchase = Id::new();
    let mut store = Store::open(&db).unwrap();
    store
        .execute_purchase(PurchaseCommand {
            id: Id::new(),
            request_key: "backup-purchase".into(),
            actor: "test".into(),
            reason: String::new(),
            body: PurchaseAction::Create {
                purchase_id: purchase,
                draft: Draft::default(),
            },
        })
        .unwrap();
    store
        .attach(
            brickxinator_infra::attachments::Command {
                id: Id::new(),
                request_key: "backup-attachment".into(),
                actor: "test".into(),
                target: brickxinator_infra::attachments::Target::Purchase(purchase),
                original_name: "invoice.pdf".into(),
            },
            b"%PDF-1.4 backup baseline",
        )
        .unwrap();
    drop(store);
    // More than the retention horizon: prove both policies with real points.
    for point in 1..=31 {
        let mut store = Store::open(&db).unwrap();
        store
            .commit(Operation {
                id: Id::new(),
                request_key: format!("backup-point-{point}"),
                actor: "test".into(),
                reason: "New measured stock between backup points".into(),
                body: Body::Create {
                    holding_id: Id::new(),
                    pieces: Measure::Known(point),
                    weight_mg: Measure::Unknown,
                },
            })
            .unwrap();
        drop(store);
        backups::run(root.path(), &db, true).unwrap();
    }
    let destination = &config.destinations[0];
    assert_eq!(std::fs::read_dir(&destination.monthly).unwrap().count(), 12);
    let points = std::process::Command::new(&config.restic)
        .arg("--repo")
        .arg(destination.daily.join("restic"))
        .arg("--password-file")
        .arg(&config.password_file)
        .args(["--no-cache", "snapshots", "--json"])
        .output()
        .unwrap();
    assert!(points.status.success());
    let points: serde_json::Value = serde_json::from_slice(&points.stdout).unwrap();
    assert_eq!(points.as_array().unwrap().len(), 30);
    let original = root
        .path()
        .join("backup-staging")
        .join(destination.id.to_string())
        .join("daily.zip");
    let mut retained = points.as_array().unwrap().iter().collect::<Vec<_>>();
    retained.sort_by_key(|point| point["time"].as_str().unwrap());
    let input_bytes: u64 = retained
        .iter()
        .map(|point| {
            point["summary"]["total_bytes_processed"]
                .as_u64()
                .unwrap_or(0)
        })
        .sum();
    for (index, point) in retained.into_iter().enumerate() {
        let restored = root.path().join(format!("restored-{index}"));
        let output = std::process::Command::new(&config.restic)
            .arg("--repo")
            .arg(destination.daily.join("restic"))
            .arg("--password-file")
            .arg(&config.password_file)
            .args([
                "--no-cache",
                "restore",
                point["id"].as_str().unwrap(),
                "--target",
            ])
            .arg(&restored)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let checked =
            snapshots::validate(restored.join(original.strip_prefix("/").unwrap())).unwrap();
        assert_eq!(checked.manifest.kind, "daily");
        assert!(checked.config().is_some());
        let store = Store::open(checked.database()).unwrap();
        assert_eq!(store.rebuild().unwrap().holdings().len(), index + 2);
        assert_eq!(store.attachments().unwrap().len(), 1);
        for attachment in store.attachments().unwrap() {
            store.attachment_bytes(attachment.id).unwrap();
        }
    }
    let monthly_database = root.path().join("monthly-check.sqlite");
    drop(Store::create(&monthly_database).unwrap());
    for archive in std::fs::read_dir(&destination.monthly).unwrap() {
        let archive = archive.unwrap().path();
        let checked = snapshots::validate(&archive).unwrap();
        let expected = checked.manifest.last_operation_sequence as usize;
        brickxinator_infra::recovery::restore(
            &monthly_database,
            &archive,
            root.path().join("monthly-safety"),
        )
        .unwrap();
        let store = Store::open(&monthly_database).unwrap();
        assert_eq!(store.rebuild().unwrap().holdings().len(), expected);
        for attachment in store.attachments().unwrap() {
            store.attachment_bytes(attachment.id).unwrap();
        }
    }
    fn size(path: &std::path::Path) -> u64 {
        std::fs::read_dir(path)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_dir() {
                    size(&entry.path())
                } else {
                    entry.metadata().unwrap().len()
                }
            })
            .sum()
    }
    println!(
        "Restic measurement: retained compressed-ZIP input bytes={input_bytes}; repository file bytes={}",
        size(&destination.daily.join("restic"))
    );
    assert!(!original.exists());
    let status = backups::status(root.path()).unwrap();
    assert!(status[0].last_daily_success.is_some());
    assert!(status[0].last_monthly_success.is_some());
}
