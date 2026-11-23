use brickxinator_core::{ledger::Id, settings};
use brickxinator_infra::{
    enrichment_store::*,
    exchange,
    integration_store::{Format, Source},
    sqlite::Store,
};
fn cmd(s: &Store, format: Format, text: &str, mappings: Vec<Mapping>) -> Command {
    Command {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "reviewed offline catalog enrichment".into(),
        source: Source {
            name: "provider.csv".into(),
            format,
            text: text.into(),
            delimiter: ',',
            columns: None,
        },
        expected_settings_sequence: s.enrichments().unwrap()["settings_sequence"]
            .as_i64()
            .unwrap(),
        accept_name_warnings: false,
        mappings,
    }
}
fn mapping(row: usize, target_id: Id, fields: &[&str]) -> Mapping {
    Mapping {
        row,
        target_id,
        expected_revision: 0,
        expected_settings_revision: 0,
        fields: fields.iter().map(|s| s.to_string()).collect(),
        local_name: None,
    }
}
#[test]
fn color_import_refresh_preserves_local_override_rgb_unknowns_and_native_history() {
    let root = tempfile::tempdir().unwrap();
    let db = root.path().join("collection.sqlite");
    let mut s = Store::create(&db).unwrap();
    let color = Id::new();
    let first = cmd(
        &s,
        Format::RebrickableColors,
        "id,name,rgb,is_trans\n0,Black,05131D,f",
        vec![mapping(
            0,
            color,
            &["name", "external_id", "rgb", "is_trans"],
        )],
    );
    let p = s.preview_enrichment(first.clone()).unwrap();
    assert_eq!(p.records[0].rgb.as_deref(), Some("05131D"));
    assert!(!s.settings().unwrap().contains_key(&color));
    let result = s.execute_enrichment(first.clone()).unwrap();
    assert_eq!(s.execute_enrichment(first).unwrap(), result);
    assert_eq!(s.settings().unwrap()[&color].name, "Black");
    s.execute_settings(settings::Command {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "collector local name".into(),
        accept_name_warnings: false,
        action: settings::Action::Rename {
            id: color,
            expected_revision: 1,
            name: "My black".into(),
        },
    })
    .unwrap();
    let mut m = mapping(0, color, &["rgb"]);
    m.expected_revision = 1;
    m.expected_settings_revision = 2;
    let refresh = cmd(
        &s,
        Format::RebrickableColors,
        "id,name,rgb,is_trans\n0,Provider changed name,000000,t",
        vec![m],
    );
    s.execute_enrichment(refresh.clone()).unwrap();
    assert_eq!(s.settings().unwrap()[&color].name, "My black");
    let record = &s.enrichments().unwrap()["records"][color.to_string()];
    assert_eq!(record["rgb"], "000000");
    assert_eq!(record["is_trans"], false);
    assert!(
        s.execute_enrichment(Command {
            id: Id::new(),
            request_key: Id::new().to_string(),
            ..refresh
        })
        .is_err()
    );
    assert_eq!(s.rebuild().unwrap().operations().len(), 0);
    let before = s.enrichments().unwrap();
    drop(s);
    let native = root.path().join("native");
    exchange::export(&db, &native).unwrap();
    let target = root.path().join("restored.sqlite");
    drop(Store::create(&target).unwrap());
    exchange::import(&target, &native, root.path().join("safe.zip")).unwrap();
    let s = Store::open(target).unwrap();
    assert_eq!(s.enrichments().unwrap(), before);
    assert_eq!(s.settings().unwrap()[&color].name, "My black");
}
#[test]
fn theme_batch_resolves_local_parents_and_rejects_cycles_and_missing_parents_atomically() {
    let root = tempfile::tempdir().unwrap();
    let mut s = Store::create(root.path().join("db.sqlite")).unwrap();
    let parent = Id::new();
    let child = Id::new();
    let first = cmd(
        &s,
        Format::RebrickableThemes,
        "id,name,parent_id\n1,Trains,\n2,Cargo,1",
        vec![
            mapping(0, parent, &["name", "parent"]),
            mapping(1, child, &["name", "parent"]),
        ],
    );
    s.execute_enrichment(first).unwrap();
    let before = s.enrichments().unwrap();
    assert_eq!(
        before["records"][child.to_string()]["parent_id"],
        parent.to_string()
    );
    let mut m = mapping(0, parent, &["parent"]);
    m.expected_revision = 1;
    let cycle = cmd(
        &s,
        Format::RebrickableThemes,
        "id,name,parent_id\n1,Trains,2",
        vec![m],
    );
    assert!(s.execute_enrichment(cycle).is_err());
    let missing = cmd(
        &s,
        Format::RebrickableThemes,
        "id,name,parent_id\n3,Unknown parent,999",
        vec![mapping(0, Id::new(), &["name", "parent"])],
    );
    assert!(s.execute_enrichment(missing).is_err());
    assert_eq!(s.enrichments().unwrap(), before);
}
#[test]
fn color_data_validation_and_protected_unknown_cannot_be_bypassed() {
    let root = tempfile::tempdir().unwrap();
    let mut s = Store::create(root.path().join("db.sqlite")).unwrap();
    let unknown = s.default_inventory_links().unwrap().color_id;
    let mut protected = cmd(
        &s,
        Format::RebrickableColors,
        "id,name,rgb,is_trans\n0,Black,000000,f",
        vec![mapping(0, unknown, &["rgb"])],
    );
    protected.mappings[0].expected_settings_revision = 1;
    assert!(s.execute_enrichment(protected).is_err());
    let invalid = cmd(
        &s,
        Format::RebrickableColors,
        "id,name,rgb,is_trans\n0,Black,nothex,f",
        vec![mapping(0, Id::new(), &["name", "rgb"])],
    );
    assert!(s.preview_enrichment(invalid).is_err());
    let blank = cmd(
        &s,
        Format::RebrickableColors,
        "id,name,rgb,is_trans\n99,Unknown RGB,,",
        vec![mapping(0, Id::new(), &["name", "rgb", "is_trans"])],
    );
    let result = s.execute_enrichment(blank).unwrap();
    assert_eq!(result.records[0].rgb, None);
    assert_eq!(result.records[0].is_trans, None);
    let next_id = Id::new();
    let stale = cmd(
        &s,
        Format::RebrickableColors,
        "id,name,rgb,is_trans\n-1,Provider unknown,,",
        vec![mapping(0, next_id, &["name", "rgb"])],
    );
    s.preview_enrichment(stale.clone()).unwrap();
    s.execute_settings(settings::Command {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "independent local rename".into(),
        accept_name_warnings: false,
        action: settings::Action::Rename {
            id: result.records[0].id,
            expected_revision: 1,
            name: "Local RGB name".into(),
        },
    })
    .unwrap();
    assert!(s.execute_enrichment(stale).is_err());
    assert!(!s.settings().unwrap().contains_key(&next_id));
}
#[test]
fn catalog_definitions_link_stable_theme_ids_and_reject_unmapped_links() {
    let root = tempfile::tempdir().unwrap();
    let mut s = Store::create(root.path().join("db.sqlite")).unwrap();
    let theme = Id::new();
    let c = cmd(
        &s,
        Format::RebrickableThemes,
        "id,name,parent_id\n7,Trains,",
        vec![mapping(0, theme, &["name", "parent"])],
    );
    s.execute_enrichment(c).unwrap();
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
    let id = Id::new();
    let command = |theme| {
        serde_json::from_value(serde_json::json!({"id":Id::new(),"request_key":Id::new().to_string(),"actor":"test","reason":"explicit theme identity","action":{"Create":{"id":id,"kind":"Set","details":{"manufacturer_id":manufacturer,"number":"60198","name":"Cargo Train","theme_id":theme}}}})).unwrap()
    };
    assert!(s.execute_catalog(command(Id::new())).is_err());
    s.execute_catalog(command(theme)).unwrap();
    assert_eq!(s.catalog().unwrap()[&id].details.theme_id, Some(theme));
    assert_eq!(s.rebuild().unwrap().operations().len(), 0);
    drop(s);
    let native = root.path().join("theme-native");
    exchange::export(root.path().join("db.sqlite"), &native).unwrap();
    let target = root.path().join("theme-restored.sqlite");
    drop(Store::create(&target).unwrap());
    exchange::import(&target, &native, root.path().join("theme-safe.zip")).unwrap();
    assert_eq!(
        Store::open(target).unwrap().catalog().unwrap()[&id]
            .details
            .theme_id,
        Some(theme)
    );
}
