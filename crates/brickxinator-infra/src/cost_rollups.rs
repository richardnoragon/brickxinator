//! Cost rollups are views, never new allocations or additions to owned cost.
use crate::sqlite::{self, Result, Store};
use brickxinator_core::{finance::Exact, inventory::Measure, ledger::Id, projects::ClaimKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Amount {
    pub known_cents: Exact,
    pub incomplete: bool,
}
impl Amount {
    fn add(&mut self, n: Option<u64>) -> Result<()> {
        if let Some(n) = n {
            self.known_cents.0 = self
                .known_cents
                .0
                .checked_add(u128::from(n))
                .ok_or("cost rollup overflow")?;
        } else {
            self.incomplete = true;
        }
        Ok(())
    }
}
#[derive(Debug, Default, Serialize)]
struct CopyRow {
    label: String,
    status: brickxinator_core::physical::Status,
    direct: Amount,
    allocated_parts: Amount,
    total: Amount,
}
#[derive(Debug, Default, Serialize)]
struct ProjectRow {
    name: String,
    copy_id: Option<Id>,
    allocated_parts: Amount,
}
fn value(v: &serde_json::Value) -> Result<BTreeMap<Id, Option<u64>>> {
    Ok(serde_json::from_value(v.clone())?)
}
fn divide(cost: u64, mut buckets: Vec<(Id, u64)>, owned: u64) -> Result<Vec<(Id, u64)>> {
    if owned == 0 {
        return Err("cannot roll up allocated costs without known positive holding count".into());
    }
    buckets.sort_by_key(|(id, _)| *id);
    let claimed = buckets
        .iter()
        .try_fold(0u64, |a, (_, n)| a.checked_add(*n))
        .ok_or("allocated quantity overflow")?;
    if claimed > owned {
        return Err("cost rollup allocations exceed owned stock".into());
    }
    // Include unassigned stock as a rounding bucket so project views never consume
    // cents belonging to loose stock. None sorts after stable UUID target ties.
    let mut rows: Vec<(Option<Id>, u64, u128)> = buckets
        .into_iter()
        .map(|(id, n)| {
            let product = u128::from(cost) * u128::from(n);
            (
                Some(id),
                (product / u128::from(owned)) as u64,
                product % u128::from(owned),
            )
        })
        .collect();
    let remaining = owned - claimed;
    let product = u128::from(cost) * u128::from(remaining);
    rows.push((
        None,
        (product / u128::from(owned)) as u64,
        product % u128::from(owned),
    ));
    let sum = rows
        .iter()
        .try_fold(0u64, |a, (_, n, _)| a.checked_add(*n))
        .ok_or("cost rounding overflow")?;
    let mut order: Vec<usize> = (0..rows.len()).collect();
    order.sort_by(|a, b| {
        rows[*b]
            .2
            .cmp(&rows[*a].2)
            .then_with(|| rows[*a].0.is_none().cmp(&rows[*b].0.is_none()))
            .then_with(|| rows[*a].0.cmp(&rows[*b].0))
    });
    for index in order.into_iter().take((cost - sum) as usize) {
        rows[index].1 += 1;
    }
    Ok(rows
        .into_iter()
        .filter_map(|(id, n, _)| id.map(|id| (id, n)))
        .collect())
}
impl Store {
    pub fn cost_rollups(&self) -> Result<serde_json::Value> {
        let tx = self.connection.unchecked_transaction()?;
        let ledger = sqlite::load(&tx)?;
        let projects = crate::project_store::history(&tx, &ledger)?;
        let mut copies: BTreeMap<Id, CopyRow> = ledger
            .set_copies()
            .keys()
            .map(|id| {
                (
                    *id,
                    CopyRow {
                        label: ledger.copy_label(*id).unwrap_or("Set copy").into(),
                        status: ledger.lifecycle(*id).status,
                        ..Default::default()
                    },
                )
            })
            .collect();
        let mut project_rows: BTreeMap<Id, ProjectRow> = projects
            .projects
            .values()
            .map(|p| {
                (
                    p.id,
                    ProjectRow {
                        name: p.name.clone(),
                        copy_id: p.copy_id,
                        ..Default::default()
                    },
                )
            })
            .collect();
        let mut latest = BTreeMap::new();
        for (c, p) in crate::child_cost_store::history(&tx, &ledger)? {
            latest.insert((c.purchase_id, c.invoice_line_id, c.component), (c, p));
        }
        let mut owned = Amount::default();
        let mut residual = Amount::default();
        let mut disposed = Amount::default();
        let mut holdings = BTreeMap::<Id, Amount>::new();
        let mut item_coverage = BTreeMap::<Id, std::collections::BTreeSet<Id>>::new();
        let mut direct_seen = std::collections::BTreeSet::new();
        let mut stale = vec![];
        for (c, p) in latest.values() {
            let current = crate::child_cost_store::layers(c, p, &ledger)?;
            residual.add(Some(p.residual_cents))?;
            disposed.add(current["disposed_cents"].as_u64())?;
            let (current_basis, _) =
                crate::child_cost_store::current_basis(&tx, c.purchase_id, &ledger)?;
            if p.basis.amount(c.invoice_line_id, c.component).ok()
                != current_basis.amount(c.invoice_line_id, c.component).ok()
            {
                stale.push(c.id);
            }

            for (id, n) in value(&current["copy_shares"])? {
                direct_seen.insert(id);
                if let Some(row) = copies.get_mut(&id) {
                    row.direct.add(n)?;
                }
                owned.add(n)?;
            }
            for (id, n) in value(&current["shares"])? {
                if c.component == brickxinator_core::child_costs::Component::Item
                    && let Some(line) = c.invoice_line_id
                {
                    item_coverage.entry(id).or_default().insert(line);
                }
                holdings.entry(id).or_default().add(n)?;
                owned.add(n)?;
                if let Some(copy) = ledger.set_contents().get(&id) {
                    if let Some(row) = copies.get_mut(copy) {
                        row.allocated_parts.add(n)?;
                    }
                    continue;
                }
                let mut buckets = BTreeMap::<Id, u64>::new();
                for claim in projects
                    .claims
                    .values()
                    .filter(|claim| claim.holding_id == id && claim.kind == ClaimKind::Allocated)
                {
                    let count = buckets.entry(claim.project_id).or_default();
                    *count = count
                        .checked_add(claim.quantity)
                        .ok_or("project allocation count overflow")?;
                }
                if buckets.is_empty() {
                    continue;
                }
                let denominator = ledger.holdings().get(&id).and_then(|h| {
                    if let Measure::Known(n) = h.pieces.owned() {
                        Some(n)
                    } else {
                        None
                    }
                });
                let split = n
                    .zip(denominator)
                    .map(|(n, qty)| {
                        divide(n, buckets.iter().map(|(id, n)| (*id, *n)).collect(), qty)
                    })
                    .transpose()?;
                for project in buckets.keys() {
                    let share = split.as_ref().and_then(|rows| {
                        rows.iter().find(|(id, _)| id == project).map(|(_, n)| *n)
                    });
                    if let Some(row) = project_rows.get_mut(project) {
                        row.allocated_parts.add(share)?;
                        if let Some(copy) = row.copy_id
                            && let Some(copy) = copies.get_mut(&copy)
                        {
                            copy.allocated_parts.add(share)?;
                        }
                    }
                }
            }
        }
        let active = ledger.current_holdings();
        let mut uncosted = vec![];
        let sources = brickxinator_core::source_layers::project(&ledger);
        for (id, h) in &active {
            if h.pieces.owned() == Measure::Known(0) && h.weight_mg.owned() == Measure::Known(0) {
                continue;
            }
            let missing_source = sources.get(id).is_some_and(|layers| {
                layers.iter().any(|(line, qty)| {
                    !(qty.pieces == Some(0) && qty.weight_mg == Some(0))
                        && !item_coverage
                            .get(id)
                            .is_some_and(|lines| lines.contains(line))
                })
            });
            if !holdings.contains_key(id) || missing_source {
                holdings.entry(*id).or_default().incomplete = true;
                uncosted.push(*id);
                for claim in projects
                    .claims
                    .values()
                    .filter(|r| r.holding_id == *id && r.kind == ClaimKind::Allocated)
                {
                    if let Some(row) = project_rows.get_mut(&claim.project_id) {
                        row.allocated_parts.incomplete = true;
                        if let Some(copy) = row.copy_id
                            && let Some(copy) = copies.get_mut(&copy)
                        {
                            copy.allocated_parts.incomplete = true;
                        }
                    }
                }
                if let Some(copy) = ledger.set_contents().get(id)
                    && let Some(copy) = copies.get_mut(copy)
                {
                    copy.allocated_parts.incomplete = true;
                }
            }
        }
        for row in copies.values_mut() {
            row.total.known_cents.0 = row
                .direct
                .known_cents
                .0
                .checked_add(row.allocated_parts.known_cents.0)
                .ok_or("copy cost total overflow")?;
            row.total.incomplete = row.direct.incomplete || row.allocated_parts.incomplete;
        }
        let uncosted_copies: Vec<_> = ledger
            .set_copies()
            .keys()
            .filter(|id| {
                ledger.lifecycle(**id).status == brickxinator_core::physical::Status::Active
                    && !direct_seen.contains(id)
            })
            .copied()
            .collect();
        Ok(
            serde_json::json!({"copies":copies,"projects":project_rows,"holdings":holdings,"reviewed_owned":owned,"unallocated_source":residual,"disposed":disposed,"uncosted_holdings":uncosted,"copies_without_direct_assignment":uncosted_copies,"requires_reallocation":stale,"inventory_sequence":ledger.operations().len(),"project_revision":projects.revision}),
        )
    }
}
