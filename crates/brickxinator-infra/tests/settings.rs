use brickxinator_core::{
    ledger::Id,
    settings::{self, Action, Command, Kind, SettingsError, Vocabulary, Warning},
};
use brickxinator_infra::sqlite::Store;
use rusqlite::Connection;

fn command(key: &str, action: Action) -> Command {
    Command {
        id: Id::new(),
        request_key: key.into(),
        actor: "local-user".into(),
        reason: String::new(),
        accept_name_warnings: false,
        action,
    }
}
fn setup() -> (tempfile::TempDir, std::path::PathBuf, Store) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.sqlite");
    let store = Store::create(&path).unwrap();
    (dir, path, store)
}
fn location(store: &mut Store, key: &str, name: &str, parent_id: Option<Id>) -> Id {
    let id = Id::new();
    store
        .execute_settings(command(
            key,
            Action::CreateLocation {
                id,
                parent_id,
                name: name.into(),
            },
        ))
        .unwrap();
    id
}

#[test]
fn seeds_provide_one_protected_unknown_per_vocabulary_and_initial_conditions() {
    let (_dir, path, store) = setup();
    let records = store.settings().unwrap();
    assert_eq!(records.len(), 6);
    for vocabulary in [
        Vocabulary::Manufacturer,
        Vocabulary::Color,
        Vocabulary::Condition,
    ] {
        assert_eq!(records.values().filter(|r| matches!(r.kind,Kind::Vocabulary {vocabulary:v,system_unknown:true} if v==vocabulary)).count(),1);
    }
    for name in ["New", "Used", "Mixed"] {
        assert!(records.values().any(|r| r.name == name));
    }
    drop(store);
    assert_eq!(Store::open(path).unwrap().settings().unwrap(), records);
}

#[test]
fn system_unknown_cannot_be_changed_archived_or_duplicated() {
    let (_dir, path, mut store) = setup();
    let records = store.settings().unwrap();
    let unknown = records
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
    for action in [
        Action::Rename {
            id: unknown,
            expected_revision: 1,
            name: "Other".into(),
        },
        Action::Archive {
            id: unknown,
            expected_revision: 1,
            archived: true,
        },
    ] {
        assert!(
            store
                .execute_settings(command("protected", action))
                .is_err()
        );
    }
    let mut duplicate = command(
        "duplicate",
        Action::CreateVocabulary {
            id: Id::new(),
            vocabulary: Vocabulary::Color,
            name: " unknown ".into(),
        },
    );
    duplicate.accept_name_warnings = true;
    assert!(store.execute_settings(duplicate).is_err());
    let raw = Connection::open(path).unwrap();
    assert!(
        raw.execute(
            "UPDATE settings_record SET display_name='Other' WHERE id=?1",
            [unknown.to_string()]
        )
        .is_err()
    );
    assert!(
        raw.execute(
            "DELETE FROM settings_record WHERE id=?1",
            [unknown.to_string()]
        )
        .is_err()
    );
    assert_eq!(store.settings().unwrap(), records);
}

#[test]
fn names_preserve_user_input_and_warn_before_duplicates_or_typos() {
    let (_dir, _path, mut store) = setup();
    let drawer = location(&mut store, "drawer", "Drawer", None);
    let mut near = command(
        "near",
        Action::CreateLocation {
            id: Id::new(),
            parent_id: None,
            name: "Drawr".into(),
        },
    );
    let error = store.execute_settings(near.clone()).unwrap_err();
    assert!(
        matches!(error.downcast_ref::<SettingsError>(),Some(SettingsError::NameWarnings(warnings)) if warnings.iter().any(|w| matches!(w,Warning::Similar {id,..} if *id==drawer)))
    );
    near.accept_name_warnings = true;
    assert_eq!(store.execute_settings(near).unwrap().after.name, "Drawr");
    let mut duplicate = command(
        "duplicate",
        Action::CreateLocation {
            id: Id::new(),
            parent_id: None,
            name: "  DRAWER  ".into(),
        },
    );
    assert!(store.execute_settings(duplicate.clone()).is_err());
    duplicate.accept_name_warnings = true;
    let change = store.execute_settings(duplicate).unwrap();
    assert_eq!(change.after.name, "  DRAWER  ");
    assert!(matches!(change.warnings[0], Warning::Duplicate { .. }));
    // Identical names under different parents need no acknowledgement.
    location(&mut store, "child", "Drawer", Some(drawer));
}

#[test]
fn nested_locations_reject_self_parent_descendant_cycles_and_vocabulary_parents() {
    let (_dir, _path, mut store) = setup();
    let room = location(&mut store, "room", "Workshop", None);
    let cabinet = location(&mut store, "cabinet", "Cabinet", Some(room));
    let bin = location(&mut store, "bin", "Bin", Some(cabinet));
    for parent_id in [Some(room), Some(bin)] {
        assert!(
            store
                .execute_settings(command(
                    "cycle",
                    Action::Reparent {
                        id: room,
                        expected_revision: 1,
                        parent_id
                    }
                ))
                .is_err()
        );
    }
    let vocab = *store
        .settings()
        .unwrap()
        .values()
        .find(|r| matches!(r.kind, Kind::Vocabulary { .. }))
        .map(|r| &r.id)
        .unwrap();
    assert!(
        store
            .execute_settings(command(
                "invalid-parent",
                Action::CreateLocation {
                    id: Id::new(),
                    parent_id: Some(vocab),
                    name: "Shelf".into()
                }
            ))
            .is_err()
    );
    store
        .execute_settings(command(
            "move",
            Action::Reparent {
                id: bin,
                expected_revision: 1,
                parent_id: Some(room),
            },
        ))
        .unwrap();
    assert_eq!(
        store.settings().unwrap()[&bin].kind,
        Kind::Location {
            parent_id: Some(room)
        }
    );
    assert!(store.rebuild().unwrap().holdings().is_empty());
}

#[test]
fn archiving_preserves_tree_and_ids_but_blocks_new_selection_under_parent() {
    let (_dir, path, mut store) = setup();
    let room = location(&mut store, "room", "Workshop", None);
    let bin = location(&mut store, "bin", "Bin", Some(room));
    store
        .execute_settings(command(
            "archive",
            Action::Archive {
                id: room,
                expected_revision: 1,
                archived: true,
            },
        ))
        .unwrap();
    let records = store.settings().unwrap();
    assert!(records.contains_key(&bin));
    assert!(!settings::selectable(&records, bin));
    assert!(
        !store
            .selectable_settings()
            .unwrap()
            .iter()
            .any(|r| r.id == bin)
    );
    assert!(
        store
            .execute_settings(command(
                "new-child",
                Action::CreateLocation {
                    id: Id::new(),
                    parent_id: Some(bin),
                    name: "Box".into()
                }
            ))
            .is_err()
    );
    store
        .execute_settings(command(
            "unarchive",
            Action::Archive {
                id: room,
                expected_revision: 2,
                archived: false,
            },
        ))
        .unwrap();
    drop(store);
    let store = Store::open(path).unwrap();
    assert!(settings::selectable(&store.settings().unwrap(), bin));
}

#[test]
fn revisions_and_retries_preserve_original_results_after_restart() {
    let (_dir, path, mut store) = setup();
    let id = Id::new();
    let create = command(
        "create",
        Action::CreateVocabulary {
            id,
            vocabulary: Vocabulary::Manufacturer,
            name: "LEGO".into(),
        },
    );
    let original = store.execute_settings(create.clone()).unwrap();
    store
        .execute_settings(command(
            "rename",
            Action::Rename {
                id,
                expected_revision: 1,
                name: "LEGO Group".into(),
            },
        ))
        .unwrap();
    assert!(
        store
            .execute_settings(command(
                "stale",
                Action::Archive {
                    id,
                    expected_revision: 1,
                    archived: true
                }
            ))
            .is_err()
    );
    drop(store);
    let mut store = Store::open(path).unwrap();
    assert_eq!(store.execute_settings(create.clone()).unwrap(), original);
    assert_eq!(store.settings().unwrap()[&id].name, "LEGO Group");
    let history = store.settings_history(id).unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[1].change.before.as_ref().unwrap().name, "LEGO");
    assert!(
        history
            .iter()
            .all(|r| r.committed_at.ends_with('Z') && r.actor == "local-user")
    );
    let mut conflict = create;
    conflict.reason = "different payload".into();
    assert!(store.execute_settings(conflict).is_err());
}

#[test]
fn failed_audit_rolls_back_the_settings_change() {
    let (_dir, path, mut store) = setup();
    let raw = Connection::open(&path).unwrap();
    raw.execute_batch("CREATE TRIGGER injected_failure BEFORE INSERT ON settings_command BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
    let id = Id::new();
    let create = command(
        "create",
        Action::CreateVocabulary {
            id,
            vocabulary: Vocabulary::Color,
            name: "Red".into(),
        },
    );
    assert!(store.execute_settings(create.clone()).is_err());
    assert!(!store.settings().unwrap().contains_key(&id));
    raw.execute_batch("DROP TRIGGER injected_failure").unwrap();
    store.execute_settings(create).unwrap();
    assert!(store.settings().unwrap().contains_key(&id));
    assert!(raw.execute("DELETE FROM settings_command", []).is_err());
    raw.execute(
        "UPDATE settings_record SET display_name='Unrecorded edit' WHERE id=?1",
        [id.to_string()],
    )
    .unwrap();
    assert!(Store::open(path).is_err());
}

#[test]
fn concurrent_tree_moves_cannot_create_a_cycle() {
    let (_dir, path, mut store) = setup();
    let a = location(&mut store, "a", "Room A", None);
    let mut create_b = command(
        "b",
        Action::CreateLocation {
            id: Id::new(),
            name: "Room B".into(),
            parent_id: None,
        },
    );
    create_b.accept_name_warnings = true;
    let b = store.execute_settings(create_b).unwrap().after.id;
    drop(store);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let workers: Vec<_> = [(a, b), (b, a)]
        .into_iter()
        .enumerate()
        .map(|(i, (id, parent))| {
            let path = path.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut store = Store::open(path).unwrap();
                barrier.wait();
                store
                    .execute_settings(command(
                        &format!("move-{i}"),
                        Action::Reparent {
                            id,
                            expected_revision: 1,
                            parent_id: Some(parent),
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
    settings::validate_tree(&Store::open(path).unwrap().settings().unwrap()).unwrap();
}

#[test]
fn invalid_names_fail_without_consuming_request_keys() {
    let (_dir, _path, mut store) = setup();
    for name in ["", "  ", "Drawer\n14"] {
        assert!(
            store
                .execute_settings(command(
                    "new",
                    Action::CreateLocation {
                        id: Id::new(),
                        parent_id: None,
                        name: name.into()
                    }
                ))
                .is_err()
        );
    }
    location(&mut store, "new", "Drawer 14", None);
    assert_eq!(store.settings().unwrap().len(), 7);
}

#[test]
fn schema_two_opens_without_installing_settings() {
    use sha2::{Digest, Sha256};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("schema-two.sqlite");
    let raw = Connection::open(&path).unwrap();
    for (version, name, sql) in [
        (
            1,
            "migration_001.sql",
            include_str!("../../../migrations/migration_001.sql"),
        ),
        (
            2,
            "migration_002.sql",
            include_str!("../../../migrations/migration_002.sql"),
        ),
    ] {
        raw.execute_batch(sql).unwrap();
        raw.execute(
            "INSERT INTO schema_migration(version,name,checksum) VALUES(?1,?2,?3)",
            rusqlite::params![
                version,
                name,
                format!("{:x}", Sha256::digest(sql.as_bytes()))
            ],
        )
        .unwrap();
    }
    drop(raw);
    let store = Store::open(&path).unwrap();
    assert!(store.settings().is_err());
    assert!(store.rebuild().unwrap().holdings().is_empty());
    let raw = Connection::open(path).unwrap();
    let versions: i64 = raw
        .query_row("SELECT count(*) FROM schema_migration", [], |r| r.get(0))
        .unwrap();
    assert_eq!(versions, 2);
}

#[test]
fn request_keys_cannot_be_reused_between_settings_and_purchases() {
    use brickxinator_core::purchases::{Draft, PurchaseAction, PurchaseCommand};
    let (_dir, _path, mut store) = setup();
    location(&mut store, "shared", "Workshop", None);
    let mut purchase = PurchaseCommand {
        id: Id::new(),
        request_key: "shared".into(),
        actor: "local-user".into(),
        reason: String::new(),
        body: PurchaseAction::Create {
            purchase_id: Id::new(),
            draft: Draft::default(),
        },
    };
    assert!(store.execute_purchase(purchase.clone()).is_err());
    purchase.request_key = "purchase".into();
    store.execute_purchase(purchase).unwrap();
    assert!(
        store
            .execute_settings(command(
                "purchase",
                Action::CreateLocation {
                    id: Id::new(),
                    parent_id: None,
                    name: "Other room".into()
                }
            ))
            .is_err()
    );
}
