//! Cost layers are derived independently from physical ownership and acquisition history.
use crate::sqlite::{self, Result, Store};
use brickxinator_core::{
    child_costs::{self, Basis, Command, Component, Plan},
    inventory::Measure,
    ledger::{Body, Id, Ledger},
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::collections::BTreeMap;
fn enabled(c: &Connection) -> Result<bool> {
    Ok(c.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migration WHERE version=16)",
        [],
        |r| r.get(0),
    )?)
}
pub(crate) fn key_exists(c: &Connection, key: &str) -> Result<bool> {
    Ok(enabled(c)?
        && c.query_row(
            "SELECT EXISTS(SELECT 1 FROM child_cost_command WHERE request_key=?1)",
            [key],
            |r| r.get(0),
        )?)
}
type Key = (Id, Option<Id>, Component);
fn key(c: &Command) -> Key {
    (c.purchase_id, c.invoice_line_id, c.component)
}
pub(crate) fn current_basis(c: &Connection, p: Id, l: &Ledger) -> Result<(Basis, u64)> {
    let purchase =
        crate::purchase_store::read_purchase(c, p)?.ok_or("cost source purchase does not exist")?;
    let financial = crate::fulfilment_store::states(c, l)?
        .remove(&p)
        .unwrap_or_default();
    let shipping = crate::finance_store::read_state(c, p)?
        .0
        .unwrap_or_else(|| brickxinator_core::finance::State::for_purchase(&purchase));
    Ok((
        Basis {
            purchase,
            financial: financial.financial,
            shipping,
        },
        financial.revision,
    ))
}
fn valid_basis(c: &Connection, b: &Basis, fulfilment_revision: u64) -> Result<()> {
    crate::fulfilment_store::canonical_source(c, &b.purchase)?;
    if fulfilment_revision == 0 {
        if b.financial != Default::default() {
            return Err("invalid initial financial source".into());
        }
    } else {
        let mut found = false;
        let mut q = c.prepare("SELECT result_json FROM fulfilment_command WHERE purchase_id=?1")?;
        for row in q.query_map([b.purchase.id.to_string()], |r| r.get::<_, String>(0))? {
            let state: brickxinator_core::fulfilment::State = serde_json::from_str(&row?)?;
            if state.revision == fulfilment_revision && state.financial == b.financial {
                found = true;
            }
        }
        if !found {
            return Err("cost financial basis is not canonical history".into());
        }
    }
    if b.shipping.revision == 0 {
        if b.shipping != brickxinator_core::finance::State::for_purchase(&b.purchase) {
            return Err("invalid initial shipping basis".into());
        }
    } else {
        let mut found = false;
        let mut q = c.prepare("SELECT result_json FROM shipping_command WHERE purchase_id=?1")?;
        for row in q.query_map([b.purchase.id.to_string()], |r| r.get::<_, String>(0))? {
            let result: crate::finance_store::AllocationResult = serde_json::from_str(&row?)?;
            if result.change.after == b.shipping {
                found = true;
            }
        }
        if !found {
            return Err("cost shipping basis is not canonical history".into());
        }
    }
    Ok(())
}
pub(crate) fn history(c: &Connection, ledger: &Ledger) -> Result<Vec<(Command, Plan)>> {
    if !enabled(c)? {
        return Ok(vec![]);
    }
    let mut q=c.prepare("SELECT id,request_key,payload_json,basis_json,result_json FROM child_cost_command ORDER BY sequence")?;
    let mut records: Vec<(Command, Plan)> = vec![];
    let mut revisions = BTreeMap::<Key, u64>::new();
    let mut prefix = Ledger::default();
    let mut position = 0;
    for row in q.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
        ))
    })? {
        let (id, request, payload, basis, result) = row?;
        let command: Command = serde_json::from_str(&payload)?;
        let basis: Basis = serde_json::from_str(&basis)?;
        if command.id.to_string() != id
            || command.request_key != request
            || command.expected_inventory_sequence < position
            || command.expected_inventory_sequence > ledger.operations().len()
        {
            return Err("invalid cost history envelope or inventory prefix".into());
        }
        while position < command.expected_inventory_sequence {
            prefix.append(ledger.operations()[position].clone())?;
            position += 1;
        }
        valid_basis(c, &basis, command.expected_fulfilment_revision)?;
        let disposed = prior_disposed(&records, &command, &prefix)?;
        let plan = child_costs::prepare(
            &command,
            &basis,
            &prefix,
            *revisions.get(&key(&command)).unwrap_or(&0),
            disposed,
        )?;
        if plan != serde_json::from_str::<Plan>(&result)? {
            return Err("cost result differs from immutable source replay".into());
        }
        revisions.insert(key(&command), plan.revision);
        records.push((command, plan));
    }
    Ok(records)
}
fn prior_disposed(records: &[(Command, Plan)], command: &Command, ledger: &Ledger) -> Result<u64> {
    if let Some((c, p)) = records.iter().rev().find(|(c, _)| key(c) == key(command)) {
        Ok(layers(c, p, ledger)?["disposed_cents"]
            .as_u64()
            .ok_or("disposed cost is not known")?)
    } else {
        Ok(0)
    }
}
pub(crate) fn validate(c: &Connection, l: &Ledger) -> Result<()> {
    history(c, l)?;
    Ok(())
}
fn known(m: Measure) -> Option<u64> {
    if let Measure::Known(n) = m {
        Some(n)
    } else {
        None
    }
}
fn proportion(
    cost: u64,
    before: brickxinator_core::journal::Holding,
    after: brickxinator_core::journal::Holding,
    prefer_retained: bool,
) -> Option<u64> {
    if before.pieces == after.pieces && before.weight_mg == after.weight_mg {
        return Some(cost);
    }
    let ratio = known(before.pieces.owned())
        .zip(known(after.pieces.owned()))
        .filter(|(old, _)| *old > 0)
        .or_else(|| known(before.weight_mg.owned()).zip(known(after.weight_mg.owned())));
    match ratio {
        Some((old, new)) if old > 0 && new <= old => {
            let product = u128::from(cost) * u128::from(new);
            let denominator = u128::from(old);
            let remainder = product % denominator;
            let extra = u128::from(
                remainder * 2 > denominator || (remainder * 2 == denominator && prefer_retained),
            );
            u64::try_from(product / denominator + extra).ok()
        }
        Some((0, 0)) => Some(cost),
        _ => None,
    }
}
/// Replay the physical effects after each reviewed cost plan. Layers stay separate
/// across purchases; unknown quantity bases remain explicitly incomplete.
pub(crate) fn layers(command: &Command, plan: &Plan, ledger: &Ledger) -> Result<serde_json::Value> {
    let mut prefix = Ledger::default();
    let mut shares: BTreeMap<Id, Option<u64>> = plan
        .shares
        .iter()
        .chain(&plan.copy_shares)
        .map(|(id, n)| (*id, Some(*n)))
        .collect();
    let mut disposed = plan.disposed_cents;
    let mut issues: Vec<String> = vec![];
    let mut effects = BTreeMap::<Id, (BTreeMap<Id, Option<u64>>, u64, u64)>::new();
    for (index, op) in ledger.operations().iter().enumerate() {
        let before = prefix.holdings().clone();
        let provenance = if matches!(&op.body,Body::SupplierReturn{sources,..} if !sources.is_empty())
        {
            Some(brickxinator_core::source_layers::project(&prefix))
        } else {
            None
        };
        let before_copies: BTreeMap<_, _> = plan
            .copy_shares
            .keys()
            .map(|id| (*id, prefix.lifecycle(*id).status))
            .collect();
        let outcome = prefix.append(op.clone())?;
        if index < command.expected_inventory_sequence {
            continue;
        }
        let changed_copies: Vec<_> = before_copies
            .iter()
            .filter(|(id, status)| prefix.lifecycle(**id).status != **status)
            .map(|(id, _)| *id)
            .collect();
        let previous_shares = shares.clone();
        let previous_disposed = disposed;
        let correction=match &op.body{
            Body::Lifecycle(e)=>e.corrects,
            Body::SupplierReturnCorrection{dispatch_operation_id}=>Some(*dispatch_operation_id),
            Body::ReceiptCorrection{receipt_operation_id,reverse:false,..}=>prefix.operations().iter().rev().find(|old|matches!(old.body,Body::ReceiptCorrection{receipt_operation_id:id,reverse:true,..} if id==*receipt_operation_id)).map(|old|old.id),
            _=>None
        };
        if let Some(target) = correction
            && let Some((old, disposed_before, disposed_after)) = effects.get(&target)
        {
            for id in outcome.holdings.keys().chain(changed_copies.iter()) {
                if let Some(n) = old.get(id) {
                    shares.insert(*id, *n);
                } else {
                    shares.remove(id);
                }
            }
            disposed = disposed
                .checked_sub(disposed_after - disposed_before)
                .ok_or("cost correction exceeds disposed cost")?;
            effects.insert(op.id, (previous_shares, previous_disposed, disposed));
            continue;
        }
        match &op.body {
            Body::Merge { sources, target_id } => {
                let mut sum = Some(0u64);
                let mut any = false;
                for source in sources {
                    if let Some(amount) = shares.remove(&source.holding_id) {
                        any = true;
                        sum = sum.zip(amount).and_then(|(a, b)| a.checked_add(b));
                    }
                }
                if any {
                    shares.insert(*target_id, sum);
                }
            }
            Body::Identify {
                source_id,
                child_id,
                ..
            }
            | Body::Split {
                source_id,
                child_id,
                ..
            } => {
                if let Some(amount) = shares.get(source_id).copied() {
                    let remaining = amount.and_then(|cost| {
                        proportion(
                            cost,
                            before[source_id],
                            outcome.holdings[source_id],
                            source_id < child_id,
                        )
                    });
                    let child = amount.zip(remaining).map(|(a, b)| a - b);
                    shares.insert(*source_id, remaining);
                    shares.insert(*child_id, child);
                }
            }
            Body::Lifecycle(e) if e.corrects.is_none() => {
                for disposition in &e.parts {
                    let id = disposition.holding_id;
                    if let Some(amount) = shares.get(&id).copied() {
                        match disposition.handling {
                            brickxinator_core::physical::Handling::KeepAll => {}
                            brickxinator_core::physical::Handling::TransferAll => {
                                if let Some(n) = amount {
                                    disposed =
                                        disposed.checked_add(n).ok_or("disposed cost overflow")?;
                                }
                                shares.insert(id, Some(0));
                            }
                            brickxinator_core::physical::Handling::Custom { child_id, .. } => {
                                let kept = amount.and_then(|n| {
                                    proportion(
                                        n,
                                        before[&id],
                                        outcome.holdings[&child_id],
                                        child_id < op.id,
                                    )
                                });
                                if let (Some(n), Some(k)) = (amount, kept) {
                                    disposed = disposed
                                        .checked_add(n - k)
                                        .ok_or("disposed cost overflow")?;
                                }
                                shares.insert(id, Some(0));
                                shares.insert(child_id, kept);
                            }
                        }
                    }
                }
            }
            Body::SupplierReturn {
                holding_id,
                sources,
                ..
            } if !sources.is_empty() => {
                if let Some(amount) = shares.get(holding_id).copied() {
                    let belongs = |id: Id| {
                        command.invoice_line_id.map_or_else(
                            || plan.basis.purchase.draft.lines.iter().any(|l| l.id == id),
                            |line| line == id,
                        )
                    };
                    let selected: Vec<_> = sources
                        .iter()
                        .filter(|s| belongs(s.invoice_line_id))
                        .collect();
                    if !selected.is_empty() {
                        let all = provenance.as_ref().and_then(|p| p.get(holding_id));
                        let sum = |weight: bool| {
                            all?.iter().filter(|(id, _)| belongs(**id)).try_fold(
                                0u64,
                                |a, (_, l)| {
                                    a.checked_add(if weight { l.weight_mg? } else { l.pieces? })
                                },
                            )
                        };
                        let removed = |weight: bool| {
                            selected.iter().try_fold(0u64, |a, l| {
                                a.checked_add(if weight { l.weight_mg? } else { l.pieces? })
                            })
                        };
                        let ratio = sum(false)
                            .zip(removed(false))
                            .filter(|(old, _)| *old > 0)
                            .or_else(|| sum(true).zip(removed(true)).filter(|(old, _)| *old > 0));
                        let next = amount.zip(ratio).and_then(|(cost, (old, n))| {
                            let retained = old.checked_sub(n)?;
                            let product = u128::from(cost) * u128::from(retained);
                            let denom = u128::from(old);
                            let rem = product % denom;
                            u64::try_from(
                                product / denom
                                    + u128::from(
                                        rem * 2 > denom
                                            || (rem * 2 == denom && *holding_id < op.id),
                                    ),
                            )
                            .ok()
                        });
                        if let (Some(old), Some(new)) = (amount, next) {
                            disposed = disposed
                                .checked_add(old - new)
                                .ok_or("disposed cost overflow")?;
                        }
                        shares.insert(*holding_id, next);
                    }
                }
            }
            Body::Detach { .. } => {}
            _ => {
                for (id, after) in &outcome.holdings {
                    if let Some(amount) = shares.get(id).copied() {
                        let next = amount.and_then(|cost| {
                            before
                                .get(id)
                                .and_then(|old| proportion(cost, *old, *after, *id < op.id))
                        });
                        if let (Some(old), Some(new)) = (amount, next) {
                            disposed = disposed
                                .checked_add(old - new)
                                .ok_or("disposed cost overflow")?;
                        }
                        shares.insert(*id, next);
                    }
                }
            }
        }
        for id in &changed_copies {
            if prefix.lifecycle(*id).status != brickxinator_core::physical::Status::Active {
                if let Some(Some(n)) = shares.get(id) {
                    disposed = disposed
                        .checked_add(*n)
                        .ok_or("disposed copy cost overflow")?;
                    shares.insert(*id, Some(0));
                }
            } else {
                shares.insert(*id, None);
            }
        }
        effects.insert(op.id, (previous_shares, previous_disposed, disposed));
    }
    if shares.values().any(Option::is_none) {
        issues.push(
            "physical changes lack a safe proportional cost basis; review a new allocation".into(),
        );
    }
    Ok(
        serde_json::json!({"copy_shares":shares.iter().filter(|(id,_)|plan.copy_shares.contains_key(id)).collect::<BTreeMap<_,_>>(),"shares":shares.iter().filter(|(id,_)|!plan.copy_shares.contains_key(id)).collect::<BTreeMap<_,_>>(),"disposed_cents":disposed,"issues":issues,"residual_cents":plan.residual_cents}),
    )
}
impl Store {
    pub fn child_costs(&self, purchase: Id) -> Result<serde_json::Value> {
        let tx = self.connection.unchecked_transaction()?;
        let ledger = sqlite::load(&tx)?;
        let (basis, fulfilment_revision) = current_basis(&tx, purchase, &ledger)?;
        let records = history(&tx, &ledger)?;
        let mut latest = BTreeMap::new();
        for (c, p) in &records {
            if c.purchase_id == purchase {
                latest.insert(key(c), (c, p));
            }
        }
        let mut plans = vec![];
        for (c, p) in latest.values() {
            plans.push(serde_json::json!({"command":c,"plan":p,"current":layers(c,p,&ledger)?,"requires_reallocation":p.basis.amount(c.invoice_line_id,c.component).ok()!=basis.amount(c.invoice_line_id,c.component).ok()}));
        }
        Ok(
            serde_json::json!({"basis":basis,"fulfilment_revision":fulfilment_revision,"inventory_sequence":ledger.operations().len(),"plans":plans,"history":records.iter().filter(|(c,_)|c.purchase_id==purchase).collect::<Vec<_>>()}),
        )
    }
    pub fn preview_child_cost(&self, command: Command) -> Result<Plan> {
        let tx = self.connection.unchecked_transaction()?;
        let ledger = sqlite::load(&tx)?;
        let (basis, revision) = current_basis(&tx, command.purchase_id, &ledger)?;
        if revision != command.expected_fulfilment_revision {
            return Err("financial source changed since cost review".into());
        }
        let records = history(&tx, &ledger)?;
        let disposed = prior_disposed(&records, &command, &ledger)?;
        let old = records
            .into_iter()
            .rev()
            .find(|(c, _)| key(c) == key(&command))
            .map_or(0, |(_, p)| p.revision);
        Ok(child_costs::prepare(
            &command, &basis, &ledger, old, disposed,
        )?)
    }
    pub fn execute_child_cost(&mut self, command: Command) -> Result<Plan> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let cached: Option<(String, String)> = tx
            .query_row(
                "SELECT payload_json,result_json FROM child_cost_command WHERE request_key=?1",
                [&command.request_key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((payload, result)) = cached {
            if serde_json::from_str::<Command>(&payload)? != command {
                return Err("cost request key conflict".into());
            }
            return Ok(serde_json::from_str(&result)?);
        }
        if sqlite::metadata_key_exists(&tx,&command.request_key)?||tx.query_row("SELECT EXISTS(SELECT 1 FROM inventory_operation WHERE request_key=?1) OR EXISTS(SELECT 1 FROM purchase_command WHERE request_key=?1)",[&command.request_key],|r|r.get::<_,bool>(0))?{return Err("request key belongs to another command".into());}
        let ledger = sqlite::load(&tx)?;
        let (basis, revision) = current_basis(&tx, command.purchase_id, &ledger)?;
        if revision != command.expected_fulfilment_revision {
            return Err("financial source changed since cost review".into());
        }
        let records = history(&tx, &ledger)?;
        let disposed = prior_disposed(&records, &command, &ledger)?;
        let old = records
            .into_iter()
            .rev()
            .find(|(c, _)| key(c) == key(&command))
            .map_or(0, |(_, p)| p.revision);
        let plan = child_costs::prepare(&command, &basis, &ledger, old, disposed)?;
        tx.execute("INSERT INTO child_cost_command(id,request_key,payload_json,basis_json,result_json) VALUES(?1,?2,?3,?4,?5)",params![command.id.to_string(),command.request_key,serde_json::to_string(&command)?,serde_json::to_string(&basis)?,serde_json::to_string(&plan)?])?;
        tx.commit()?;
        Ok(plan)
    }
}
