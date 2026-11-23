//! Audited financial adjustments. Settlement never changes physical stock.
use crate::{
    finance::{self, Method},
    ledger::Id,
    purchases::Purchase,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Discount,
    Coupon,
    LoyaltyReduction,
    Refund,
    Payment,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentMethod {
    Cash,
    Card,
    Bank,
    GiftCard,
    StoredValueVoucher,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub id: Id,
    pub kind: Kind,
    pub label: String,
    pub effective_on: String,
    pub amount_cents: u64,
    pub method: Method,
    pub payment_method: Option<PaymentMethod>,
    pub warranty_id: Option<Id>,
    pub return_id: Option<Id>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recorded {
    pub entry: Entry,
    pub shares: BTreeMap<Id, u64>,
    pub source_revision: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    SetGross { lines: BTreeMap<Id, u64> },
    Record(Entry),
    CorrectLast { entry_id: Id },
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub gross: BTreeMap<Id, u64>,
    pub entries: Vec<Recorded>,
    pub corrected: BTreeSet<Id>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    pub net_lines: BTreeMap<Id, Option<u64>>,
    pub refunded_lines: BTreeMap<Id, u64>,
    pub payments_cents: u64,
    pub gross_entered: bool,
}
impl State {
    pub fn is_empty(&self) -> bool {
        self.gross.is_empty() && self.entries.is_empty()
    }
    pub fn summary(&self, p: &Purchase) -> Result<Summary, String> {
        let mut result = Summary {
            net_lines: p
                .draft
                .lines
                .iter()
                .map(|l| {
                    (
                        l.id,
                        if self.gross.is_empty() {
                            l.paid_total_cents
                        } else {
                            self.gross.get(&l.id).copied()
                        },
                    )
                })
                .collect(),
            refunded_lines: BTreeMap::new(),
            payments_cents: 0,
            gross_entered: !self.gross.is_empty(),
        };
        for r in self
            .entries
            .iter()
            .filter(|r| !self.corrected.contains(&r.entry.id))
        {
            match r.entry.kind {
                Kind::Payment => {
                    result.payments_cents = result
                        .payments_cents
                        .checked_add(r.entry.amount_cents)
                        .ok_or("payment total overflow")?
                }
                Kind::Refund => {
                    for (id, n) in &r.shares {
                        let old = result.refunded_lines.entry(*id).or_default();
                        *old = old.checked_add(*n).ok_or("refund overflow")?;
                    }
                }
                _ => {
                    for (id, n) in &r.shares {
                        let total = result
                            .net_lines
                            .get_mut(id)
                            .ok_or("adjustment line no longer exists")?;
                        *total = Some(
                            total
                                .ok_or("discount requires known gross amount")?
                                .checked_sub(*n)
                                .ok_or("discount exceeds line gross")?,
                        );
                    }
                }
            }
        }
        for (id, n) in &result.refunded_lines {
            if result
                .net_lines
                .get(id)
                .copied()
                .flatten()
                .is_none_or(|net| *n > net)
            {
                return Err("refund exceeds known net line amount".into());
            }
        }
        Ok(result)
    }
    pub fn apply(&self, action: &Action, p: &Purchase) -> Result<Self, String> {
        let mut next = self.clone();
        match action {
            Action::SetGross { lines } => {
                if lines.len() != p.draft.lines.len()
                    || p.draft.lines.iter().any(|l| !lines.contains_key(&l.id))
                {
                    return Err("enter gross amounts for every invoice line".into());
                }
                if self
                    .entries
                    .iter()
                    .any(|r| !self.corrected.contains(&r.entry.id) && r.entry.kind != Kind::Payment)
                {
                    return Err(
                        "correct active financial adjustments before changing gross basis".into(),
                    );
                }
                next.gross = lines.clone();
            }
            Action::CorrectLast { entry_id } => {
                if self
                    .entries
                    .iter()
                    .rev()
                    .find(|r| !self.corrected.contains(&r.entry.id))
                    .map(|r| r.entry.id)
                    != Some(*entry_id)
                {
                    return Err("correct the most recent active financial entry first; later entries depend on its basis".into());
                }
                next.corrected.insert(*entry_id);
            }
            Action::Record(entry) => {
                if entry.label.trim().is_empty()
                    || !crate::purchases::valid_date(&entry.effective_on)
                    || self.entries.iter().any(|r| r.entry.id == entry.id)
                {
                    return Err(
                        "financial entry needs a unique identity, label and valid date".into(),
                    );
                }
                let shares = if entry.kind == Kind::Payment {
                    if entry.payment_method.is_none() {
                        return Err("payment instrument is required".into());
                    }
                    BTreeMap::new()
                } else {
                    if entry.payment_method.is_some() {
                        return Err("payment instruments are not price adjustments".into());
                    }
                    if !matches!(
                        entry.method,
                        Method::Equal
                            | Method::PriceProportional
                            | Method::PieceCount
                            | Method::Weight
                    ) {
                        return Err("choose an automatic invoice-line allocation method".into());
                    }
                    let before = self.summary(p)?;
                    if entry.kind != Kind::Refund && self.gross.is_empty() {
                        return Err("enter separate gross amounts before discounts; original paid totals remain unchanged".into());
                    }
                    let mut basis = p.clone();
                    basis.draft.shipping_cents = Some(entry.amount_cents);
                    for line in &mut basis.draft.lines {
                        line.paid_total_cents = if entry.kind == Kind::Refund {
                            before.net_lines[&line.id]
                        } else {
                            self.gross.get(&line.id).copied()
                        };
                    }
                    let mut state = finance::State::new(p.id);
                    state.method = entry.method;
                    let allocation = finance::allocate(&basis, &state).map_err(str::to_owned)?;
                    if allocation.status != finance::Status::Complete {
                        return Err(format!(
                            "financial allocation incomplete: {:?}",
                            allocation.issues
                        ));
                    }
                    allocation
                        .shares
                        .into_iter()
                        .map(|(id, n)| {
                            n.map(|n| (id, n))
                                .ok_or("missing financial share".to_string())
                        })
                        .collect::<Result<BTreeMap<_, _>, _>>()?
                };
                next.entries.push(Recorded {
                    entry: entry.clone(),
                    shares,
                    source_revision: p.revision,
                });
            }
        }
        next.summary(p)?;
        Ok(next)
    }
}
