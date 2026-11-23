use brickxinator_core::{
    ledger::Id,
    settings::{Kind, Vocabulary},
};
use brickxinator_infra::{exchange, integration_store::*, sqlite::Store};
#[test]
fn reviewed_catalog_refresh_preserves_uuid_local_fields_and_cached_sources() {
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
                Kind::Vocabulary {
                    vocabulary: Vocabulary::Manufacturer,
                    system_unknown: true
                }
            )
        })
        .unwrap()
        .id;
    let part = Id::new();
    let command = Command {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "reviewed Rebrickable subset".into(),
        source: Source {
            name: "parts.csv".into(),
            format: Format::RebrickableParts,
            text: "part_num,name,part_cat_id\n3001,Brick,11".into(),
            delimiter: ',',
            columns: None,
        },
        mappings: vec![Mapping {
            row: 0,
            target_id: part,
            expected_revision: 0,
            manufacturer_id: manufacturer,
            theme_id: None,
            fields: vec!["number".into(), "name".into()],
        }],
    };
    assert_eq!(s.catalog().unwrap().len(), 0);
    let preview = s.preview_integration(command.clone()).unwrap();
    assert_eq!(preview.changes.len(), 1);
    assert_eq!(s.catalog().unwrap().len(), 0);
    let result = s.execute_integration(command.clone()).unwrap();
    assert_eq!(s.execute_integration(command.clone()).unwrap(), result);
    let mut refresh = command.clone();
    refresh.id = Id::new();
    refresh.request_key = Id::new().to_string();
    refresh.source.text = "part_num,name,part_cat_id\n3001,Updated name,11".into();
    refresh.mappings[0].expected_revision = 1;
    refresh.mappings[0].fields = vec!["name".into()];
    s.execute_integration(refresh.clone()).unwrap();
    let d = &s.catalog().unwrap()[&part];
    assert_eq!(d.details.number.as_deref(), Some("3001"));
    assert_eq!(d.details.name.as_deref(), Some("Updated name"));
    assert_eq!(d.revision, 2);
    assert!(
        s.execute_integration(Command {
            id: Id::new(),
            request_key: Id::new().to_string(),
            ..refresh
        })
        .is_err()
    );
    assert_eq!(s.rebuild().unwrap().operations().len(), 0);
    let mut collision = command.clone();
    collision.id = Id::new();
    collision.request_key = format!("import:{}:0", collision.id);
    collision.mappings[0].target_id = Id::new();
    collision.source.text = "part_num,name,part_cat_id\n3002,Collision test,11".into();
    assert!(s.execute_integration(collision).is_err());
    assert_eq!(s.catalog().unwrap().len(), 1);
    let cache = s.integrations().unwrap();
    assert_eq!(cache.as_array().unwrap().len(), 2);
    drop(s);
    let native = root.path().join("native");
    exchange::export(&db, &native).unwrap();
    let target = root.path().join("restored.sqlite");
    drop(Store::create(&target).unwrap());
    exchange::import(&target, &native, root.path().join("safe.zip")).unwrap();
    assert_eq!(Store::open(target).unwrap().integrations().unwrap(), cache);
}
