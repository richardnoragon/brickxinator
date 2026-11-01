//! Synthetic purchase entry and receipt, with restart-safe retry demonstration.
use brickxinator_core::{
    inventory::{AssemblyState, Measure},
    ledger::Id,
    purchases::{Draft, InvoiceLine, LineKind, PurchaseAction, PurchaseCommand},
};
use brickxinator_infra::sqlite::{Result, Store};

fn main() -> Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: purchase_demo <NEW-database-path>")?;
    let mut store = Store::create(&path)?;
    let id = Id::new();
    let sets = InvoiceLine {
        source_url: None,
        notes: String::new(),
        id: Id::new(),
        kind: LineKind::Set,
        description: Some("Synthetic three-set invoice line".into()),
        set_number: Some("7838".into()),
        copy_count: Some(3),
        pieces: Measure::Known(1000),
        weight_mg: Measure::Unknown,
        paid_total_cents: Some(30_000),
        assembly: AssemblyState::Assembled,
    };
    let lot = InvoiceLine {
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
    };
    let draft = Draft {
        seller: Some("Demo seller".into()),
        shipping_cents: Some(699),
        lines: vec![sets, lot],
        ..Draft::default()
    };
    store.execute_purchase(PurchaseCommand {
        id: Id::new(),
        request_key: "demo-purchase".into(),
        actor: "demo".into(),
        reason: String::new(),
        body: PurchaseAction::Create {
            purchase_id: id,
            draft,
        },
    })?;
    let before = store.rebuild()?;
    println!(
        "Before receipt: {} holdings, {} set copies",
        before.holdings().len(),
        before.set_copies().len()
    );
    let receive = PurchaseCommand {
        id: Id::new(),
        request_key: "demo-receive".into(),
        actor: "demo".into(),
        reason: "Whole synthetic purchase arrived".into(),
        body: PurchaseAction::Receive {
            purchase_id: id,
            expected_revision: 1,
        },
    };
    let original = store.execute_purchase(receive.clone())?;
    drop(store);
    let mut store = Store::open(path)?;
    if store.execute_purchase(receive)? != original {
        return Err("receipt retry changed its result".into());
    }
    let ledger = store.rebuild()?;
    let purchase = store
        .purchase(id)?
        .ok_or("purchase missing after reopening")?;
    println!(
        "After reopening and retry: {} receipt, {} mixed-lot holding, {} independent set copies",
        ledger.receipts().len(),
        ledger.holdings().len(),
        ledger.set_copies().len()
    );
    let cents = purchase
        .draft
        .total_cents()
        .ok_or("demo total should be known")?;
    println!(
        "Purchase total: EUR {}.{:02}; set line retains EUR 300.00 once",
        cents / 100,
        cents % 100
    );
    println!(
        "Audited purchase revisions: {}",
        store.purchase_history(id)?.len()
    );
    Ok(())
}
