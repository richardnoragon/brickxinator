//! Derived costs retain their immutable acquisition basis and explicit residual.
use crate::{
    adjustments,
    finance::{self, Method},
    inventory::Measure,
    ledger::{Id, Ledger},
    purchases::Purchase,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Component {
    Item,
    Shipping,
    Fees,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AllocationMethod {
    Manual,
    PieceCount,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Basis {
    pub purchase: Purchase,
    pub financial: adjustments::State,
    pub shipping: finance::State,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Command {
    pub id: Id,
    pub request_key: String,
    pub actor: String,
    pub reason: String,
    pub effective_on: String,
    pub purchase_id: Id,
    pub invoice_line_id: Option<Id>,
    pub component: Component,
    pub expected_revision: u64,
    pub expected_inventory_sequence: usize,
    pub expected_purchase_revision: u64,
    pub expected_fulfilment_revision: u64,
    pub expected_shipping_revision: u64,
    pub method: AllocationMethod,
    pub allocated_cents: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_disposed_cents: Option<u64>,
    pub targets: BTreeMap<Id, Option<u64>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub copy_targets: BTreeMap<Id, Option<u64>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub copy_piece_bases: BTreeMap<Id, u64>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub revision: u64,
    pub command_id: Id,
    pub basis: Basis,
    pub total_cents: u64,
    pub shares: BTreeMap<Id, u64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub copy_shares: BTreeMap<Id, u64>,
    pub residual_cents: u64,
    pub disposed_cents: u64,
}
impl Basis {
    pub fn amount(&self, line: Option<Id>, component: Component) -> Result<u64, String> {
        match component {
            Component::Fees => {
                if line.is_some() {
                    return Err("fees have a purchase-level source; do not allocate them once per invoice line".into());
                }
                self.purchase
                    .draft
                    .fees_cents
                    .ok_or("source fees are unknown".into())
            }
            Component::Item => {
                let id = line.ok_or("item cost requires an invoice line")?;
                let summary = self.financial.summary(&self.purchase)?;
                summary
                    .net_lines
                    .get(&id)
                    .copied()
                    .flatten()
                    .ok_or("source item cost is unknown".to_string())?
                    .checked_sub(summary.refunded_lines.get(&id).copied().unwrap_or(0))
                    .ok_or("refund exceeds source item cost".into())
            }
            Component::Shipping => {
                let id = line.ok_or("shipping cost requires an invoice line")?;
                finance::allocate(
                    &{
                        let mut p = self.purchase.clone();
                        if self.shipping.method == finance::Method::PriceProportional {
                            let summary = self.financial.summary(&p)?;
                            for line in &mut p.draft.lines {
                                line.paid_total_cents =
                                    summary.net_lines.get(&line.id).copied().flatten();
                            }
                        }
                        p
                    },
                    &self.shipping,
                )
                .map_err(str::to_owned)?
                .shares
                .get(&id)
                .copied()
                .flatten()
                .ok_or("shipping allocation incomplete; explicitly allocate shipping first".into())
            }
        }
    }
}
pub fn prepare(
    command: &Command,
    basis: &Basis,
    ledger: &Ledger,
    revision: u64,
    disposed_cents: u64,
) -> Result<Plan, String> {
    if command.actor.trim().is_empty()
        || command.reason.trim().is_empty()
        || command.request_key.trim().is_empty()
        || !crate::purchases::valid_date(&command.effective_on)
    {
        return Err("cost allocation needs audit context and a valid date".into());
    }
    if command.purchase_id != basis.purchase.id
        || command.expected_purchase_revision != basis.purchase.revision
        || command.expected_shipping_revision != basis.shipping.revision
        || command.expected_inventory_sequence != ledger.operations().len()
        || command.expected_revision != revision
    {
        return Err("cost source, stock or allocation revision changed".into());
    }
    let disposed_cents = command.reviewed_disposed_cents.unwrap_or(disposed_cents);
    let total = basis.amount(command.invoice_line_id, command.component)?;
    if command
        .allocated_cents
        .checked_add(disposed_cents)
        .is_none_or(|n| n > total)
        || (command.targets.is_empty()
            && command.copy_targets.is_empty()
            && !(revision > 0 && command.allocated_cents == 0))
    {
        return Err("allocate no more than the source basis to explicit target holdings".into());
    }
    for id in command.targets.keys() {
        let h = ledger
            .current_holdings()
            .get(id)
            .copied()
            .ok_or("cost target is not an active holding")?;
        if h.pieces.owned() == Measure::Known(0) && h.weight_mg.owned() == Measure::Known(0) {
            return Err("cannot assign cost to empty physical stock".into());
        }
        let mut ancestors = vec![*id];
        let mut found = false;
        let mut visited = std::collections::BTreeSet::new();
        while let Some(id) = ancestors.pop() {
            if !visited.insert(id) {
                continue;
            }
            if let Some(line) = ledger.provenance().get(&id)
                && basis.purchase.draft.lines.iter().any(|l| l.id == *line)
                && command
                    .invoice_line_id
                    .is_none_or(|expected| expected == *line)
            {
                found = true;
            }
            for op in ledger.operations() {
                match &op.body {
                    crate::ledger::Body::Merge { sources, target_id } if *target_id == id => {
                        ancestors.extend(sources.iter().map(|s| s.holding_id))
                    }
                    crate::ledger::Body::Split {
                        source_id,
                        child_id,
                        ..
                    }
                    | crate::ledger::Body::Identify {
                        source_id,
                        child_id,
                        ..
                    } if *child_id == id => ancestors.push(*source_id),
                    _ => {}
                }
            }
        }
        if !found {
            return Err("cost target has no provenance from this acquisition source".into());
        }
    }
    for id in command.copy_targets.keys() {
        let copy = ledger
            .set_copies()
            .get(id)
            .ok_or("cost target is not a physical copy")?;
        if command.targets.contains_key(id)
            || ledger.lifecycle(*id).status != crate::physical::Status::Active
        {
            return Err("direct costs require an active copy and distinct targets".into());
        }
        if !basis
            .purchase
            .draft
            .lines
            .iter()
            .any(|line| line.id == copy.invoice_line_id)
            && !command.copy_targets.is_empty()
        {
            return Err("copy cost target does not belong to this acquisition source".into());
        }
        if command
            .invoice_line_id
            .is_some_and(|line| line != copy.invoice_line_id)
        {
            return Err("copy cost must use its acquisition invoice line".into());
        }
    }
    if command
        .copy_piece_bases
        .keys()
        .any(|id| !command.copy_targets.contains_key(id))
    {
        return Err("copy piece-count basis has an unrelated target".into());
    }
    let mut all_targets = command.targets.clone();
    all_targets.extend(command.copy_targets.clone());
    let shares = match command.method {
        AllocationMethod::Manual => {
            let shares = all_targets
                .iter()
                .map(|(id, n)| {
                    n.map(|n| (*id, n))
                        .ok_or("enter every manual target share".to_string())
                })
                .collect::<Result<BTreeMap<_, _>, _>>()?;
            if shares
                .values()
                .try_fold(0u64, |a, n| a.checked_add(*n))
                .ok_or("cost share overflow")?
                != command.allocated_cents
            {
                return Err(
                    "manual shares must exactly match the explicitly allocated amount".into(),
                );
            }
            shares
        }
        AllocationMethod::PieceCount => {
            let template = basis
                .purchase
                .draft
                .lines
                .first()
                .ok_or("source has no lines")?;
            let mut p = basis.purchase.clone();
            p.draft.shipping_cents = Some(command.allocated_cents);
            p.draft.lines = all_targets
                .keys()
                .map(|id| {
                    let mut line = template.clone();
                    line.id = *id;
                    line.pieces = ledger
                        .holdings()
                        .get(id)
                        .map(|h| h.pieces.owned())
                        .unwrap_or_else(|| {
                            command
                                .copy_piece_bases
                                .get(id)
                                .copied()
                                .map_or(Measure::Unknown, Measure::Known)
                        });
                    line
                })
                .collect();
            let mut state = finance::State::new(p.id);
            state.method = Method::PieceCount;
            let allocation = finance::allocate(&p, &state).map_err(str::to_owned)?;
            if allocation.status != finance::Status::Complete {
                return Err(format!(
                    "piece-count allocation incomplete: {:?}",
                    allocation.issues
                ));
            }
            allocation
                .shares
                .into_iter()
                .map(|(id, n)| n.map(|n| (id, n)).ok_or("missing share".to_string()))
                .collect::<Result<_, _>>()?
        }
    };
    Ok(Plan {
        revision: revision.checked_add(1).ok_or("cost revision overflow")?,
        command_id: command.id,
        basis: basis.clone(),
        total_cents: total,
        copy_shares: shares
            .iter()
            .filter(|(id, _)| command.copy_targets.contains_key(id))
            .map(|(id, n)| (*id, *n))
            .collect(),
        shares: shares
            .into_iter()
            .filter(|(id, _)| !command.copy_targets.contains_key(id))
            .collect(),
        residual_cents: total - command.allocated_cents - disposed_cents,
        disposed_cents,
    })
}
