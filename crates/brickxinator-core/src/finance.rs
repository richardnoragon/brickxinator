//! Exact EUR costs and invoice-line shipping allocation. No stock effects.
use crate::{inventory::Measure, ledger::Id, purchases::Purchase};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Exact(#[serde(with = "crate::views::decimal")] pub u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Method {
    PurchaseLevel,
    Equal,
    PriceProportional,
    PieceCount,
    Weight,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub purchase_id: Id,
    pub revision: u64,
    pub method: Method,
    pub manual_shares: BTreeMap<Id, Option<u64>>,
}
impl State {
    pub fn new(purchase_id: Id) -> Self {
        Self {
            purchase_id,
            revision: 0,
            method: Method::PurchaseLevel,
            manual_shares: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub id: Id,
    pub request_key: String,
    pub actor: String,
    pub reason: String,
    pub purchase_id: Id,
    pub expected_revision: u64,
    pub expected_purchase_revision: u64,
    pub action: Action,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Action {
    SetMethod(Method),
    SetManualShares(BTreeMap<Id, Option<u64>>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    pub before: Option<State>,
    pub after: State,
}

pub fn prepare(
    before: Option<State>,
    purchase: &Purchase,
    command: &Command,
) -> Result<Change, &'static str> {
    if command.actor.trim().is_empty() || command.request_key.trim().is_empty() {
        return Err("allocation actor and request key are required");
    }
    if purchase.id != command.purchase_id
        || before
            .as_ref()
            .is_some_and(|s| s.purchase_id != purchase.id)
    {
        return Err("allocation purchase identity mismatch");
    }
    if purchase.revision != command.expected_purchase_revision {
        return Err("purchase changed since allocation was prepared");
    }
    let mut next = before.clone().unwrap_or_else(|| State::new(purchase.id));
    if next.revision != command.expected_revision {
        return Err("shipping allocation changed since this command was prepared");
    }
    match &command.action {
        Action::SetMethod(method) => next.method = *method,
        Action::SetManualShares(shares) => {
            if shares
                .keys()
                .any(|id| !purchase.draft.lines.iter().any(|line| line.id == *id))
            {
                return Err(
                    "manual share targets must belong to this purchase's current invoice lines",
                );
            }
            next.manual_shares = shares.clone();
        }
    }
    next.revision = next
        .revision
        .checked_add(1)
        .ok_or("allocation revision overflow")?;
    Ok(Change {
        before,
        after: next,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    PurchaseLevel,
    Complete,
    Incomplete,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Issue {
    MissingShipping,
    NoLines,
    MissingBasis(Vec<Id>),
    ZeroBasis,
    MissingManualShares(Vec<Id>),
    ObsoleteManualTargets(Vec<Id>),
    UnbalancedManualShares,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Remainder {
    Balanced,
    Unallocated(Exact),
    Overallocated(Exact),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Allocation {
    pub method: Method,
    pub status: Status,
    pub issues: Vec<Issue>,
    pub shares: BTreeMap<Id, Option<u64>>,
    pub manual_remainder: Option<Remainder>,
}

pub fn allocate(purchase: &Purchase, state: &State) -> Result<Allocation, &'static str> {
    purchase
        .draft
        .validate()
        .map_err(|_| "invalid purchase draft")?;
    if state.purchase_id != purchase.id {
        return Err("allocation purchase identity mismatch");
    }
    let method = state.method;
    let mut result = Allocation {
        method,
        status: Status::Incomplete,
        issues: vec![],
        shares: BTreeMap::new(),
        manual_remainder: None,
    };
    if method == Method::PurchaseLevel {
        result.status = Status::PurchaseLevel;
        return Ok(result);
    }
    for line in &purchase.draft.lines {
        result.shares.insert(line.id, None);
    }
    let shipping = purchase.draft.shipping_cents;
    if shipping.is_none() {
        result.issues.push(Issue::MissingShipping);
    }
    if purchase.draft.lines.is_empty() {
        result.issues.push(Issue::NoLines);
    }
    if method == Method::Manual {
        let mut missing = vec![];
        let mut sum = 0_u128;
        for line in &purchase.draft.lines {
            let share = state.manual_shares.get(&line.id).copied().flatten();
            result.shares.insert(line.id, share);
            if let Some(share) = share {
                sum = sum
                    .checked_add(u128::from(share))
                    .ok_or("manual sum overflow")?;
            } else {
                missing.push(line.id);
            }
        }
        let obsolete: Vec<_> = state
            .manual_shares
            .keys()
            .filter(|id| !result.shares.contains_key(id))
            .copied()
            .collect();
        if !obsolete.is_empty() {
            result.issues.push(Issue::ObsoleteManualTargets(obsolete));
        }
        if !missing.is_empty() {
            result.issues.push(Issue::MissingManualShares(missing));
        }
        if let Some(shipping) = shipping {
            let shipping = u128::from(shipping);
            result.manual_remainder = Some(if sum < shipping {
                Remainder::Unallocated(Exact(shipping - sum))
            } else if sum > shipping {
                Remainder::Overallocated(Exact(sum - shipping))
            } else {
                Remainder::Balanced
            });
            if sum != shipping {
                result.issues.push(Issue::UnbalancedManualShares);
            }
        }
        if result.issues.is_empty() {
            result.status = Status::Complete;
        }
        return Ok(result);
    }
    let mut missing = vec![];
    let mut bases = vec![];
    let mut total = 0_u128;
    for line in &purchase.draft.lines {
        let basis = match method {
            Method::Equal => Some(1),
            Method::PriceProportional => line.paid_total_cents,
            Method::PieceCount => match line.pieces {
                Measure::Known(n) => Some(n),
                Measure::Unknown => None,
            },
            Method::Weight => match line.weight_mg {
                Measure::Known(n) => Some(n),
                Measure::Unknown => None,
            },
            _ => unreachable!(),
        };
        if let Some(basis) = basis {
            total = total
                .checked_add(u128::from(basis))
                .ok_or("basis sum overflow")?;
            bases.push(basis);
        } else {
            missing.push(line.id);
            bases.push(0);
        }
    }
    if !missing.is_empty() {
        result.issues.push(Issue::MissingBasis(missing));
    } else if total == 0 {
        result.issues.push(Issue::ZeroBasis);
    }
    if !result.issues.is_empty() {
        return Ok(result);
    }
    let shipping = shipping.ok_or("missing shipping")?;
    let mut remainders = vec![];
    let mut allocated = 0_u64;
    for (position, (line, basis)) in purchase.draft.lines.iter().zip(bases).enumerate() {
        let numerator = u128::from(shipping) * u128::from(basis);
        let floor = u64::try_from(numerator / total).map_err(|_| "allocation share overflow")?;
        allocated = allocated
            .checked_add(floor)
            .ok_or("allocation sum overflow")?;
        result.shares.insert(line.id, Some(floor));
        remainders.push((position, numerator % total));
    }
    remainders.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let remaining = usize::try_from(
        shipping
            .checked_sub(allocated)
            .ok_or("allocation exceeds shipping")?,
    )
    .map_err(|_| "remainder exceeds supported range")?;
    if remaining > remainders.len() {
        return Err("invalid allocation remainder");
    }
    for (position, _) in remainders.into_iter().take(remaining) {
        let share = result
            .shares
            .get_mut(&purchase.draft.lines[position].id)
            .and_then(Option::as_mut)
            .ok_or("missing allocation share")?;
        *share = share.checked_add(1).ok_or("allocation share overflow")?;
    }
    result.status = Status::Complete;
    Ok(result)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rate {
    pub numerator: Exact,
    pub denominator: Exact,
}
impl Rate {
    pub fn new(numerator: u128, denominator: u128) -> Option<Self> {
        if denominator == 0 {
            return None;
        }
        let (mut a, mut b) = (numerator, denominator);
        while b != 0 {
            let remainder = a % b;
            a = b;
            b = remainder;
        }
        Some(Self {
            numerator: Exact(numerator / a),
            denominator: Exact(denominator / a),
        })
    }
    /// Exact half-up decimal display; source ratios remain unchanged.
    pub fn display(self, places: u8) -> Result<String, &'static str> {
        if places > 12 || self.denominator.0 == 0 {
            return Err("rate display requires a valid ratio and 0..12 decimal places");
        }
        let scale = 10_u128.pow(u32::from(places));
        let scaled = self
            .numerator
            .0
            .checked_mul(scale)
            .ok_or("rate display overflow")?;
        let mut rounded = scaled / self.denominator.0;
        let remainder = scaled % self.denominator.0;
        if remainder >= self.denominator.0 / 2 + self.denominator.0 % 2 {
            rounded = rounded.checked_add(1).ok_or("rate rounding overflow")?;
        }
        if places == 0 {
            Ok(rounded.to_string())
        } else {
            Ok(format!(
                "{}.{:0width$}",
                rounded / scale,
                rounded % scale,
                width = usize::from(places)
            ))
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitCosts {
    pub eur_per_piece: Option<Rate>,
    pub eur_per_kg: Option<Rate>,
}
fn unit_costs(cents: u128, pieces: Measure, weight: Measure) -> UnitCosts {
    UnitCosts {
        eur_per_piece: match pieces {
            Measure::Known(n) if n > 0 => Rate::new(cents, u128::from(n) * 100),
            _ => None,
        },
        eur_per_kg: match weight {
            Measure::Known(n) if n > 0 => cents
                .checked_mul(10_000)
                .and_then(|numerator| Rate::new(numerator, u128::from(n))),
            _ => None,
        },
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineCosts {
    pub invoice_line_id: Id,
    pub paid_total_cents: Option<u64>,
    pub shipping_share_cents: Option<u64>,
    pub allocated_total_cents: Option<Exact>,
    pub paid_unit_costs: Option<UnitCosts>,
    pub allocated_unit_costs: Option<UnitCosts>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Costs {
    pub purchase_id: Id,
    pub purchase_revision: u64,
    pub state: State,
    pub allocation: Allocation,
    pub known_subtotal_cents: Exact,
    pub unknown_price_lines: u64,
    pub shipping_cents: Option<u64>,
    pub purchase_total_cents: Option<Exact>,
    pub lines: Vec<LineCosts>,
}

pub fn costs(purchase: &Purchase, state: &State) -> Result<Costs, &'static str> {
    let allocation = allocate(purchase, state)?;
    let mut subtotal = 0_u128;
    let mut unknown_price_lines = 0;
    let mut lines = vec![];
    for line in &purchase.draft.lines {
        if let Some(paid) = line.paid_total_cents {
            subtotal = subtotal
                .checked_add(u128::from(paid))
                .ok_or("subtotal overflow")?;
        } else {
            unknown_price_lines += 1;
        }
        let share = allocation.shares.get(&line.id).copied().flatten();
        let allocated = if allocation.status == Status::Complete {
            line.paid_total_cents
                .zip(share)
                .map(|(paid, share)| u128::from(paid) + u128::from(share))
        } else {
            None
        };
        lines.push(LineCosts {
            invoice_line_id: line.id,
            paid_total_cents: line.paid_total_cents,
            shipping_share_cents: share,
            allocated_total_cents: allocated.map(Exact),
            paid_unit_costs: line
                .paid_total_cents
                .map(|paid| unit_costs(u128::from(paid), line.pieces, line.weight_mg)),
            allocated_unit_costs: allocated
                .map(|paid| unit_costs(paid, line.pieces, line.weight_mg)),
        });
    }
    let total = if unknown_price_lines == 0 {
        purchase
            .draft
            .shipping_cents
            .map(|shipping| Exact(subtotal + u128::from(shipping)))
    } else {
        None
    };
    Ok(Costs {
        purchase_id: purchase.id,
        purchase_revision: purchase.revision,
        state: state.clone(),
        allocation,
        known_subtotal_cents: Exact(subtotal),
        unknown_price_lines,
        shipping_cents: purchase.draft.shipping_cents,
        purchase_total_cents: total,
        lines,
    })
}
