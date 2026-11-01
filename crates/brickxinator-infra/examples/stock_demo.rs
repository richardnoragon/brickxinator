//! Synthetic lot closure/reopen and damage/removal workflow.
use brickxinator_core::{
    inventory::{AssemblyState, Measure},
    ledger::{Body, Id, Operation},
    purchases::{Draft, InvoiceLine, LineKind, PurchaseAction, PurchaseCommand},
    stock::{Action, Adjustment, Command, Removal},
};
use brickxinator_infra::sqlite::{Result, Store};
fn command(key: &str, holding_id: Id, expected_revision: u64, action: Action) -> Command {
    Command {
        id: Id::new(),
        request_key: key.into(),
        actor: "demo".into(),
        reason: "Synthetic stock workflow".into(),
        holding_id,
        expected_revision,
        action,
    }
}
fn main() -> Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: stock_demo <NEW-database-path>")?;
    let mut store = Store::create(&path)?;
    let purchase = Id::new();
    store.execute_purchase(PurchaseCommand {
        id: Id::new(),
        request_key: "draft".into(),
        actor: "demo".into(),
        reason: String::new(),
        body: PurchaseAction::Create {
            purchase_id: purchase,
            draft: Draft {
                lines: vec![InvoiceLine {
                    source_url: None,
                    notes: String::new(),
                    id: Id::new(),
                    kind: LineKind::MixedLot,
                    description: Some("Synthetic mixed lot".into()),
                    set_number: None,
                    copy_count: None,
                    pieces: Measure::Unknown,
                    weight_mg: Measure::Known(5_000_000),
                    paid_total_cents: Some(8000),
                    assembly: AssemblyState::Loose,
                }],
                ..Draft::default()
            },
        },
    })?;
    let receipt = store.execute_purchase(PurchaseCommand {
        id: Id::new(),
        request_key: "receive".into(),
        actor: "demo".into(),
        reason: String::new(),
        body: PurchaseAction::Receive {
            purchase_id: purchase,
            expected_revision: 1,
        },
    })?;
    let source = *receipt
        .receipt
        .ok_or("missing receipt")?
        .holdings
        .keys()
        .next()
        .ok_or("missing holding")?;
    let child = Id::new();
    store.commit(Operation {
        id: Id::new(),
        request_key: "identify".into(),
        actor: "demo".into(),
        reason: "Identify 100 unweighed pieces".into(),
        body: Body::Identify {
            source_id: source,
            child_id: child,
            expected_revision: 1,
            pieces: Some(100),
            weight_mg: None,
        },
    })?;
    let close = command(
        "confirm-empty",
        source,
        2,
        Action::Complete {
            confirm_physical_empty: true,
        },
    );
    store.execute_stock(close.clone())?;
    println!(
        "After explicit empty confirmation: {} current holding",
        store.current_inventory()?.len()
    );
    drop(store);
    let mut store = Store::open(path)?;
    store.execute_stock(close)?;
    store.execute_stock(command("reopen", source, 3, Action::Reopen))?;
    println!(
        "Reopened source measurements: pieces {:?}, weight_mg {:?}",
        store.current_inventory()?[&source].pieces.owned(),
        store.current_inventory()?[&source].weight_mg.owned()
    );
    store.execute_stock(command(
        "broken",
        child,
        1,
        Action::Adjust(Adjustment::Broken {
            pieces: Some(2),
            weight_mg: None,
        }),
    ))?;
    store.execute_stock(command(
        "donated",
        child,
        2,
        Action::Adjust(Adjustment::Remove {
            kind: Removal::Donated,
            pieces: Some(5),
            weight_mg: None,
            damaged_pieces: 0,
            damaged_weight_mg: 0,
        }),
    ))?;
    let holding = store.current_inventory()?[&child];
    println!(
        "Child after damage/donation: owned {:?}, damaged {:?}, available {:?}",
        holding.pieces.owned(),
        holding.pieces.damaged(),
        holding.pieces.available(holding.assembly)
    );
    println!(
        "Original acquisition weight stays {:?}; source history has {} entries",
        store
            .purchase(purchase)?
            .ok_or("missing purchase")?
            .draft
            .lines[0]
            .weight_mg,
        store.holding_history(source)?.len()
    );
    Ok(())
}
