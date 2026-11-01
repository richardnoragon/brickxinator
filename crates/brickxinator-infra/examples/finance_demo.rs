//! Exact invoice-line allocation and durable incomplete manual drafts.
use brickxinator_core::{
    finance::{Action, Command, Method},
    inventory::{AssemblyState, Measure},
    ledger::Id,
    purchases::{Draft, InvoiceLine, LineKind, PurchaseAction, PurchaseCommand},
};
use brickxinator_infra::sqlite::{Result, Store};
use std::collections::BTreeMap;
fn command(key: &str, purchase_id: Id, expected_revision: u64, action: Action) -> Command {
    Command {
        id: Id::new(),
        request_key: key.into(),
        actor: "demo".into(),
        reason: String::new(),
        purchase_id,
        expected_revision,
        expected_purchase_revision: 1,
        action,
    }
}
fn main() -> Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: finance_demo <NEW-database-path>")?;
    let mut store = Store::create(&path)?;
    let purchase = Id::new();
    let lines: Vec<_> = [
        (2000, 100, 1000),
        (3000, 500, 2000),
        (6000, 3500, 5_000_000),
    ]
    .into_iter()
    .map(|(price, pieces, weight)| InvoiceLine {
        source_url: None,
        notes: String::new(),
        id: Id::new(),
        kind: LineKind::LooseBatch,
        description: Some("Synthetic invoice line".into()),
        set_number: None,
        copy_count: None,
        pieces: Measure::Known(pieces),
        weight_mg: Measure::Known(weight),
        paid_total_cents: Some(price),
        assembly: AssemblyState::Loose,
    })
    .collect();
    let ids: Vec<_> = lines.iter().map(|line| line.id).collect();
    store.execute_purchase(PurchaseCommand {
        id: Id::new(),
        request_key: "draft".into(),
        actor: "demo".into(),
        reason: String::new(),
        body: PurchaseAction::Create {
            purchase_id: purchase,
            draft: Draft {
                seller: Some("Demo seller".into()),
                shipping_cents: Some(100),
                lines,
                ..Draft::default()
            },
        },
    })?;
    println!(
        "Default: {:?}",
        store.purchase_costs(purchase)?.allocation.status
    );
    for (revision, method) in [
        Method::Equal,
        Method::PriceProportional,
        Method::PieceCount,
        Method::Weight,
    ]
    .into_iter()
    .enumerate()
    {
        let result = store.execute_allocation(command(
            &format!("method-{revision}"),
            purchase,
            revision as u64,
            Action::SetMethod(method),
        ))?;
        println!(
            "{method:?} shares (cents): {:?}",
            result
                .costs
                .lines
                .iter()
                .map(|line| line.shipping_share_cents)
                .collect::<Vec<_>>()
        );
    }
    let manual = BTreeMap::from([(ids[0], Some(20)), (ids[1], Some(30)), (ids[2], None)]);
    store.execute_allocation(command(
        "manual-draft",
        purchase,
        4,
        Action::SetManualShares(manual.clone()),
    ))?;
    let manual_request = command("manual", purchase, 5, Action::SetMethod(Method::Manual));
    let original = store.execute_allocation(manual_request.clone())?;
    println!(
        "Manual draft: {:?}, remainder {:?}",
        original.costs.allocation.status, original.costs.allocation.manual_remainder
    );
    store.execute_allocation(command(
        "equal-again",
        purchase,
        6,
        Action::SetMethod(Method::Equal),
    ))?;
    store.execute_allocation(command(
        "manual-again",
        purchase,
        7,
        Action::SetMethod(Method::Manual),
    ))?;
    drop(store);
    let mut store = Store::open(path)?;
    if store.execute_allocation(manual_request)? != original {
        return Err("retry changed its original result".into());
    }
    let costs = store.purchase_costs(purchase)?;
    if costs.state.manual_shares != manual {
        return Err("manual draft changed on method switch/restart".into());
    }
    let rates = costs.lines[2]
        .paid_unit_costs
        .as_ref()
        .ok_or("missing demo rates")?;
    println!(
        "EUR/kg: {}; EUR/piece: {}",
        rates.eur_per_kg.ok_or("missing weight rate")?.display(2)?,
        rates
            .eur_per_piece
            .ok_or("missing piece rate")?
            .display(9)?
    );
    let total = costs.purchase_total_cents.ok_or("missing demo total")?.0;
    println!(
        "Unchanged purchase total: EUR {}.{:02}",
        total / 100,
        total % 100
    );
    println!(
        "Shipping revisions: {}; inventory operations: {}",
        store.allocation_history(purchase)?.len(),
        store.rebuild()?.operations().len()
    );
    Ok(())
}
