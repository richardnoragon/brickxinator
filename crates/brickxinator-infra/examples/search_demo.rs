//! Synthetic identifier/text/location search and exact partial-coverage totals.
use brickxinator_core::{
    catalog,
    inventory::Measure,
    ledger::{Body, Id, Operation},
    profiles::{self, Target},
    settings::{self, Kind, Vocabulary},
    views::Search,
};
use brickxinator_infra::sqlite::{Result, Store};
fn main() -> Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: search_demo <NEW-database-path>")?;
    let mut store = Store::create(&path)?;
    let brand = Id::new();
    let drawer = Id::new();
    for (key, action) in [
        (
            "brand",
            settings::Action::CreateVocabulary {
                id: brand,
                vocabulary: Vocabulary::Manufacturer,
                name: "LEGO".into(),
            },
        ),
        (
            "drawer",
            settings::Action::CreateLocation {
                id: drawer,
                parent_id: None,
                name: "Drawer 14".into(),
            },
        ),
    ] {
        store.execute_settings(settings::Command {
            id: Id::new(),
            request_key: key.into(),
            actor: "demo".into(),
            reason: String::new(),
            accept_name_warnings: false,
            action,
        })?;
    }
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
                manufacturer_id: brand,
                number: Some("3001".into()),
                name: Some("Train brick".into()),
                description: None,
                release_year: None,
                theme: None,
                stated_piece_count: None,
            },
        },
    })?;
    let condition = store
        .settings()?
        .values()
        .find(|r| {
            r.name == "Used"
                && matches!(
                    r.kind,
                    Kind::Vocabulary {
                        vocabulary: Vocabulary::Condition,
                        ..
                    }
                )
        })
        .ok_or("missing Used condition")?
        .id;
    for (index, count) in [100, 50].into_iter().enumerate() {
        let holding = Id::new();
        store.commit(Operation {
            id: Id::new(),
            request_key: format!("holding-{index}"),
            actor: "demo".into(),
            reason: "Synthetic stock observation".into(),
            body: Body::Create {
                holding_id: holding,
                pieces: Measure::Known(count),
                weight_mg: Measure::Unknown,
            },
        })?;
        let mut links = store.default_inventory_links()?;
        links.definition_id = Some(part);
        links.condition_id = condition;
        links.location_id = Some(drawer);
        store.execute_profile(profiles::Command {
            id: Id::new(),
            request_key: format!("profile-{index}"),
            actor: "demo".into(),
            reason: String::new(),
            target: Target::Holding(holding),
            expected_revision: 0,
            links,
        })?;
    }
    let query = Search {
        text: "3001".into(),
        location_id: Some(drawer),
        limit: 1,
        ..Search::default()
    };
    let page = store.search_inventory(&query)?;
    println!(
        "Search 3001 in Drawer 14: {} matches, {} displayed",
        page.total_matches,
        page.rows.len()
    );
    println!(
        "All-match owned pieces: {}; holdings with unknown weight: {}",
        page.totals.all.owned_pieces.known_sum, page.totals.all.owned_weight_mg.unknown_holdings
    );
    drop(store);
    let mut store = Store::open(path)?;
    let before = store.dashboard()?;
    println!(
        "Rebuilt {} searchable documents",
        store.rebuild_inventory_search()?
    );
    if store.dashboard()? != before {
        return Err("cache rebuild changed dashboard totals".into());
    }
    println!(
        "FTS train brick matches: {}",
        store
            .search_inventory(&Search {
                text: "train brick".into(),
                ..Search::default()
            })?
            .total_matches
    );
    println!(
        "Reopening/rebuilding preserved {} pieces",
        store.dashboard()?.inventory.all.owned_pieces.known_sum
    );
    Ok(())
}
