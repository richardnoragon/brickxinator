//! Historical paid invoice-line prices for currently linked catalog definitions.
use crate::sqlite::{Result, Store};
use brickxinator_core::{
    finance::{self, State, UnitCosts},
    ledger::Id,
    profiles::Target,
    purchases::{LineKind, Status},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Price {
    pub purchase_id: Id,
    pub invoice_line_id: Id,
    pub purchase_revision: u64,
    pub at: String,
    pub purchased_on: Option<String>,
    pub seller: Option<String>,
    pub kind: LineKind,
    pub paid_total_cents: Option<u64>,
    pub copy_count: Option<u32>,
    pub attributable: bool,
    pub paid_unit_costs: Option<UnitCosts>,
    pub current: bool,
}
impl Store {
    pub fn price_history(&self, definition: Id) -> Result<Vec<Price>> {
        if !self.catalog()?.contains_key(&definition) {
            return Err("catalog definition does not exist".into());
        }
        let ledger = self.rebuild()?;
        let profiles = self.inventory_profiles()?;
        let mut lines = BTreeSet::new();
        for profile in profiles
            .values()
            .filter(|p| p.links.definition_id == Some(definition))
        {
            let source = match profile.target {
                Target::Holding(id) => ledger.provenance().get(&id).copied(),
                Target::SetCopy(id) => ledger
                    .set_copies()
                    .get(&id)
                    .map(|copy| copy.invoice_line_id),
            };
            if let Some(source) = source {
                lines.insert(source);
            }
        }
        let mut result = vec![];
        for purchase in self
            .purchases()?
            .into_iter()
            .filter(|p| p.status == Status::Received)
        {
            if !purchase
                .draft
                .lines
                .iter()
                .any(|line| lines.contains(&line.id))
            {
                continue;
            }
            // Keep each distinct acquisition/price revision, not archive-only edits.
            let mut previous = std::collections::BTreeMap::new();
            for revision in self.purchase_history(purchase.id)? {
                if revision.after.status != Status::Received {
                    continue;
                }
                let costs = finance::costs(&revision.after, &State::new(purchase.id))?;
                for line in revision
                    .after
                    .draft
                    .lines
                    .iter()
                    .filter(|line| lines.contains(&line.id))
                {
                    let signature = (
                        line.paid_total_cents,
                        line.pieces,
                        line.weight_mg,
                        line.copy_count,
                        revision.after.draft.seller.clone(),
                        revision.after.draft.purchased_on.clone(),
                    );
                    if previous.get(&line.id) == Some(&signature) {
                        continue;
                    }
                    previous.insert(line.id, signature);
                    let attributable = line.kind != LineKind::MixedLot;
                    result.push(Price {
                        purchase_id: purchase.id,
                        invoice_line_id: line.id,
                        purchase_revision: revision.after.revision,
                        at: revision.committed_at.clone(),
                        purchased_on: revision.after.draft.purchased_on.clone(),
                        seller: revision.after.draft.seller.clone(),
                        kind: line.kind,
                        paid_total_cents: line.paid_total_cents,
                        copy_count: line.copy_count,
                        attributable,
                        paid_unit_costs: if attributable {
                            costs
                                .lines
                                .iter()
                                .find(|cost| cost.invoice_line_id == line.id)
                                .and_then(|cost| cost.paid_unit_costs.clone())
                        } else {
                            None
                        },
                        current: false,
                    });
                }
            }
        }
        let mut seen = BTreeSet::new();
        for record in result.iter_mut().rev() {
            record.current = seen.insert(record.invoice_line_id);
        }
        result.sort_by(|a, b| {
            b.at.cmp(&a.at)
                .then(a.invoice_line_id.cmp(&b.invoice_line_id))
        });
        Ok(result)
    }
}
