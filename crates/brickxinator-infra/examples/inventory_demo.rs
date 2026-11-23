//! Synthetic demonstration; creates a new database and refuses existing paths.
use brickxinator_core::{
    inventory::Measure,
    ledger::{Body, Id, Operation},
};
use brickxinator_infra::sqlite::{Result, Store};

fn main() -> Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: inventory_demo <NEW-database-path>")?;
    let source = Id::new();
    let child = Id::new();
    let mut store = Store::create(&path)?;
    store.commit(Operation {
        id: Id::new(),
        request_key: "demo-observation".into(),
        actor: "demo".into(),
        reason: "Synthetic mixed-lot observation".into(),
        body: Body::Create {
            holding_id: source,
            pieces: Measure::Known(3500),
            weight_mg: Measure::Known(5_000_000),
        },
    })?;
    store.commit(Operation {
        id: Id::new(),
        request_key: "demo-identification".into(),
        actor: "demo".into(),
        reason: "Identify 2500 synthetic pieces".into(),
        body: Body::Identify {
            source_id: source,
            child_id: child,
            expected_revision: 1,
            pieces: Some(2500),
            weight_mg: Some(3_000_000),
        },
    })?;
    drop(store);
    let store = Store::open(path)?;
    let rebuilt = store.rebuild()?;
    println!(
        "SQLite {}; reopened and replayed {} operations",
        store.sqlite_version(),
        rebuilt.operations().len()
    );
    for (id, holding) in rebuilt.holdings() {
        println!(
            "{id}: pieces={:?}, weight_mg={:?}, revision={}",
            holding.pieces.owned(),
            holding.weight_mg.owned(),
            holding.revision
        );
    }
    Ok(())
}
