//! Synthetic catalog linking, classification and independent child relocation.
use brickxinator_core::{
    catalog,
    inventory::Measure,
    ledger::{Body, Id, Operation},
    profiles::{self, Target},
    settings::{self, Kind, Vocabulary},
};
use brickxinator_infra::sqlite::{Result, Store};

fn main() -> Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: catalog_demo <NEW-database-path>")?;
    let mut store = Store::create(&path)?;
    let manufacturer = store
        .settings()?
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
        .ok_or("missing Unknown manufacturer")?
        .id;
    let part = Id::new();
    store.execute_catalog(catalog::Command {
        id: Id::new(),
        request_key: "part".into(),
        actor: "demo".into(),
        reason: String::new(),
        action: catalog::Action::Create {
            id: part,
            kind: catalog::Kind::Part,
            details: catalog::Details {
                expected_contents: Vec::new(),
                manufacturer_id: manufacturer,
                number: Some("3001".into()),
                name: Some("Synthetic 2 x 4 brick definition".into()),
                description: None,
                release_year: None,
                theme: None,
                stated_piece_count: None,
            },
        },
    })?;
    println!(
        "After catalog creation: {} holdings",
        store.rebuild()?.holdings().len()
    );
    let drawer = Id::new();
    let shelf = Id::new();
    for (key, id, name) in [("drawer", drawer, "Drawer 14"), ("shelf", shelf, "Shelf")] {
        store.execute_settings(settings::Command {
            id: Id::new(),
            request_key: key.into(),
            actor: "demo".into(),
            reason: String::new(),
            accept_name_warnings: false,
            action: settings::Action::CreateLocation {
                id,
                parent_id: None,
                name: name.into(),
            },
        })?;
    }
    let source = Id::new();
    let child = Id::new();
    store.commit(Operation {
        id: Id::new(),
        request_key: "opening".into(),
        actor: "demo".into(),
        reason: "Synthetic opening observation".into(),
        body: Body::Create {
            holding_id: source,
            pieces: Measure::Known(100),
            weight_mg: Measure::Known(1000),
        },
    })?;
    let mut links = store.default_inventory_links()?;
    links.location_id = Some(drawer);
    store.execute_profile(profiles::Command {
        id: Id::new(),
        request_key: "store-source".into(),
        actor: "demo".into(),
        reason: String::new(),
        target: Target::Holding(source),
        expected_revision: 0,
        links,
    })?;
    store.commit(Operation {
        id: Id::new(),
        request_key: "identify".into(),
        actor: "demo".into(),
        reason: "Identify 20 synthetic pieces".into(),
        body: Body::Identify {
            source_id: source,
            child_id: child,
            expected_revision: 1,
            pieces: Some(20),
            weight_mg: None,
        },
    })?;
    let inherited = store.inventory_profiles()?[&Target::Holding(child)].clone();
    println!(
        "Child inherited source drawer: {}",
        inherited.links.location_id == Some(drawer)
    );
    let mut links = inherited.links;
    links.definition_id = Some(part);
    links.location_id = Some(shelf);
    store.execute_profile(profiles::Command {
        id: Id::new(),
        request_key: "link-and-move".into(),
        actor: "demo".into(),
        reason: "Identify child and move whole holding".into(),
        target: Target::Holding(child),
        expected_revision: 1,
        links,
    })?;
    drop(store);
    let store = Store::open(path)?;
    let ledger = store.rebuild()?;
    println!(
        "After reopening: source {:?} pieces, child {:?} pieces",
        ledger.holdings()[&source].pieces.owned(),
        ledger.holdings()[&child].pieces.owned()
    );
    println!(
        "Catalog definitions: {}; child profile revisions: {}",
        store.catalog()?.len(),
        store.profile_history(Target::Holding(child))?.len()
    );
    Ok(())
}
