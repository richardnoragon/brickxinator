//! Order intent, logistics and warranty are independent of physical receipt effects.
use crate::{
    inventory::Measure,
    ledger::{Body, Id, Ledger, ReceivedLine, ReceivedStock},
    purchases::{LineKind, Purchase},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShipmentStatus {
    Expected,
    Shipped,
    Received,
    Lost,
    Returned,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shipment {
    pub id: Id,
    pub label: String,
    pub tracking: String,
    pub status: ShipmentStatus,
    pub notes: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarrantyStatus {
    Open,
    Submitted,
    Approved,
    Rejected,
    ReplacementSent,
    Completed,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Warranty {
    pub id: Id,
    pub label: String,
    pub status: WarrantyStatus,
    pub notes: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Purpose {
    Purchase,
    Warranty { claim_id: Id },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub id: Id,
    pub shipment_id: Option<Id>,
    pub purpose: Purpose,
    pub effective_on: String,
    pub lines: Vec<ReceivedLine>,
    pub notes: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Backorder {
    pub line_id: Id,
    pub pieces: Option<u64>,
    pub weight_mg: Option<u64>,
    pub copies: Option<u64>,
    pub notes: String,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub revision: u64,
    pub shipments: BTreeMap<Id, Shipment>,
    pub receipts: BTreeMap<Id, Receipt>,
    pub receipt_operations: BTreeMap<Id, Id>,
    pub warranties: BTreeMap<Id, Warranty>,
    pub backorders: BTreeMap<Id, Backorder>,
    pub closed: bool,
    #[serde(default, skip_serializing_if = "crate::adjustments::State::is_empty")]
    pub financial: crate::adjustments::State,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub returns: BTreeMap<Id, crate::supplier_returns::Return>,
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
pub enum Action {
    Financial(crate::adjustments::Action),
    SupplierReturn(crate::supplier_returns::Action),
    Shipment(Shipment),
    Receipt(Receipt),
    ReceiptCorrection {
        receipt_operation_id: Id,
        reverse: bool,
    },
    Backorder(Backorder),
    Close {
        closed: bool,
    },
    Warranty(Warranty),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineCoverage {
    pub line_id: Id,
    pub ordered: Measure,
    pub received: Measure,
    pub outstanding: Measure,
    pub unit: String,
    pub backordered: bool,
}
fn amount(m: Measure) -> Option<u64> {
    match m {
        Measure::Known(n) => Some(n),
        Measure::Unknown => None,
    }
}
fn plus(a: Measure, b: Measure) -> Result<Measure, String> {
    match (a, b) {
        (Measure::Known(a), Measure::Known(b)) => Ok(Measure::Known(
            a.checked_add(b).ok_or("received quantity overflow")?,
        )),
        _ => Ok(Measure::Unknown),
    }
}
impl State {
    pub fn coverage(
        &self,
        purchase: &Purchase,
        ledger: &Ledger,
    ) -> Result<Vec<LineCoverage>, String> {
        let mut received = BTreeMap::new();
        for op in ledger.operations() {
            if let Body::AddCopies {
                purchase_id,
                invoice_line_id,
                copies,
            } = &op.body
                && *purchase_id == purchase.id
                && ledger
                    .receipts()
                    .get(&purchase.id)
                    .is_none_or(|id| !ledger.receipt_reversed(*id))
            {
                let count = copies
                    .iter()
                    .filter(|c| ledger.lifecycle(c.id).status != crate::physical::Status::Void)
                    .count() as u64;
                let old = received
                    .entry(*invoice_line_id)
                    .or_insert(Measure::Known(0));
                *old = plus(*old, Measure::Known(count))?;
            }
        }
        for op in ledger.operations() {
            if ledger.receipt_reversed(op.id) {
                continue;
            }
            let lines = match &op.body {
                Body::Receive { purchase_id, lines } if *purchase_id == purchase.id => lines,
                Body::Delivery {
                    receipt_id,
                    purchase_id,
                    lines,
                } if *purchase_id == purchase.id
                    && self
                        .receipts
                        .get(receipt_id)
                        .is_some_and(|r| r.purpose == Purpose::Purchase) =>
                {
                    lines
                }
                _ => continue,
            };
            for line in lines {
                let definition = purchase
                    .draft
                    .lines
                    .iter()
                    .find(|l| l.id == line.invoice_line_id)
                    .ok_or("receipt line no longer exists")?;
                let qty = match &line.stock {
                    ReceivedStock::Sets { copies } => {
                        if copies.is_empty() {
                            Measure::Unknown
                        } else {
                            Measure::Known(
                                copies
                                    .iter()
                                    .filter(|c| {
                                        ledger.lifecycle(c.id).status
                                            != crate::physical::Status::Void
                                    })
                                    .count() as u64,
                            )
                        }
                    }
                    ReceivedStock::Batch {
                        pieces, weight_mg, ..
                    } => {
                        let (pieces, weight_mg) = if matches!(op.body, Body::Receive { .. }) {
                            (definition.pieces, definition.weight_mg)
                        } else {
                            (*pieces, *weight_mg)
                        };
                        if definition.pieces == Measure::Unknown
                            && definition.weight_mg != Measure::Unknown
                        {
                            weight_mg
                        } else {
                            pieces
                        }
                    }
                };
                let old = received
                    .entry(line.invoice_line_id)
                    .or_insert(Measure::Known(0));
                *old = plus(*old, qty)?;
            }
        }
        purchase
            .draft
            .lines
            .iter()
            .map(|line| {
                let (ordered, unit) = if line.kind == LineKind::Set {
                    (
                        line.copy_count
                            .map_or(Measure::Unknown, |n| Measure::Known(n.into())),
                        "copies",
                    )
                } else if line.pieces == Measure::Unknown && line.weight_mg != Measure::Unknown {
                    (line.weight_mg, "mg")
                } else {
                    (line.pieces, "pieces")
                };
                let r = received.get(&line.id).copied().unwrap_or(Measure::Known(0));
                let outstanding = match (ordered, r) {
                    (Measure::Known(o), Measure::Known(r)) => Measure::Known(o.saturating_sub(r)),
                    _ => Measure::Unknown,
                };
                Ok(LineCoverage {
                    line_id: line.id,
                    ordered,
                    received: r,
                    outstanding,
                    unit: unit.into(),
                    backordered: self.backorders.get(&line.id).is_some_and(|b| {
                        b.copies.or(b.pieces).or(b.weight_mg).is_some_and(|n| n > 0)
                    }),
                })
            })
            .collect()
    }
    pub fn status(&self, purchase: &Purchase, ledger: &Ledger) -> Result<&'static str, String> {
        if self.closed {
            return Ok("Closed");
        }
        if purchase.status == crate::purchases::Status::Cancelled {
            return Ok("Cancelled");
        }
        let c = self.coverage(purchase, ledger)?;
        if ledger
            .receipts()
            .get(&purchase.id)
            .is_some_and(|id| !ledger.receipt_reversed(*id))
            && !c
                .iter()
                .any(|r| matches!(r.outstanding,Measure::Known(n) if n>0))
        {
            return Ok("Completed");
        }
        if !c.is_empty() && c.iter().all(|c| c.outstanding == Measure::Known(0)) {
            return Ok("Completed");
        }
        if c.iter()
            .any(|c| c.backordered && c.outstanding != Measure::Known(0))
        {
            return Ok("Backordered");
        }
        if c.iter().any(|c| c.received != Measure::Known(0)) {
            return Ok("PartiallyReceived");
        }
        Ok("Open")
    }
    pub fn apply(
        &self,
        command: &Command,
        purchase: &Purchase,
        ledger: &Ledger,
    ) -> Result<Self, String> {
        if command.purchase_id != purchase.id
            || command.expected_purchase_revision != purchase.revision
            || command.expected_revision != self.revision
        {
            return Err("order or fulfilment revision changed since review".into());
        }
        if command.actor.trim().is_empty()
            || command.reason.trim().is_empty()
            || command.request_key.trim().is_empty()
        {
            return Err("actor, reason and request key are required".into());
        }
        let mut next = self.clone();
        match &command.action {
            Action::SupplierReturn(action) => {
                use crate::supplier_returns::{Action as R, Target};
                match action {
                    R::Plan(r) => {
                        if r.label.trim().is_empty()
                            || !crate::purchases::valid_date(&r.requested_on)
                            || r.dispatched_by.is_some()
                            || r.acknowledged_on.is_some()
                            || r.corrected_by.is_some()
                            || self.returns.contains_key(&r.id)
                            || r.warranty_id
                                .is_some_and(|id| !self.warranties.contains_key(&id))
                        {
                            return Err("invalid return plan or warranty reference".into());
                        }
                        next.returns.insert(r.id, r.clone());
                    }
                    R::Dispatch {
                        return_id,
                        effective_on,
                        target,
                    } => {
                        let r = next
                            .returns
                            .get_mut(return_id)
                            .ok_or("supplier return does not exist")?;
                        if r.dispatched_by.is_some() || !crate::purchases::valid_date(effective_on)
                        {
                            return Err("return already dispatched or date invalid".into());
                        }
                        let invoice_line = match target {
                            Target::Holding {
                                holding_id,
                                sources,
                                ..
                            } => {
                                if !sources.is_empty()
                                    && sources.iter().any(|s| {
                                        !purchase
                                            .draft
                                            .lines
                                            .iter()
                                            .any(|l| l.id == s.invoice_line_id)
                                    })
                                {
                                    return Err("return acquisition sources must belong to the selected supplier order".into());
                                }
                                ledger
                                    .provenance()
                                    .get(holding_id)
                                    .copied()
                                    .or_else(|| sources.first().map(|s| s.invoice_line_id))
                            }
                            Target::Copy(e) => {
                                if e.status != crate::physical::Status::Returned
                                    || e.corrects.is_some()
                                    || e.effective_on != *effective_on
                                {
                                    return Err(
                                        "supplier copy dispatch must be a Returned lifecycle event"
                                            .into(),
                                    );
                                }
                                ledger
                                    .set_copies()
                                    .get(&e.copy_id)
                                    .map(|c| c.invoice_line_id)
                            }
                        };
                        if invoice_line
                            .is_none_or(|id| !purchase.draft.lines.iter().any(|l| l.id == id))
                        {
                            return Err("return stock must have this purchase's source provenance; split or reconcile mixed sources first".into());
                        }
                        r.dispatched_by = Some(command.id);
                    }
                    R::Acknowledge {
                        return_id,
                        effective_on,
                    } => {
                        let r = next
                            .returns
                            .get_mut(return_id)
                            .ok_or("supplier return does not exist")?;
                        if r.dispatched_by.is_none()
                            || r.corrected_by.is_some()
                            || r.acknowledged_on.is_some()
                            || !crate::purchases::valid_date(effective_on)
                        {
                            return Err("acknowledgment requires an active dispatched return and valid date".into());
                        }
                        r.acknowledged_on = Some(effective_on.clone());
                    }
                    R::CorrectDispatch {
                        return_id,
                        dispatch_operation_id,
                        effective_on,
                    } => {
                        let r = next
                            .returns
                            .get_mut(return_id)
                            .ok_or("supplier return does not exist")?;
                        if r.dispatched_by != Some(*dispatch_operation_id)
                            || r.corrected_by.is_some()
                            || !crate::purchases::valid_date(effective_on)
                        {
                            return Err("return correction reference or date is invalid".into());
                        }
                        r.corrected_by = Some(command.id);
                    }
                }
            }
            Action::Financial(action) => {
                if let crate::adjustments::Action::Record(entry) = action {
                    if entry
                        .warranty_id
                        .is_some_and(|id| !self.warranties.contains_key(&id))
                    {
                        return Err("financial warranty reference does not exist".into());
                    }
                    if entry
                        .return_id
                        .is_some_and(|id| !self.returns.contains_key(&id))
                    {
                        return Err("return reference requires an existing supplier return".into());
                    }
                }
                next.financial = self.financial.apply(action, purchase)?;
            }
            Action::Shipment(shipment) => {
                if shipment.label.trim().is_empty() {
                    return Err("shipment label is required".into());
                }
                let has_receipt = self.receipts.iter().any(|(id, receipt)| {
                    receipt.shipment_id == Some(shipment.id)
                        && self
                            .receipt_operations
                            .get(id)
                            .is_some_and(|op| !ledger.receipt_reversed(*op))
                });
                if shipment.status == ShipmentStatus::Received && !has_receipt {
                    return Err("Received shipment needs an actual non-reversed receipt".into());
                }
                next.shipments.insert(shipment.id, shipment.clone());
            }
            Action::Warranty(warranty) => {
                if warranty.label.trim().is_empty() {
                    return Err("claim label is required".into());
                }
                next.warranties.insert(warranty.id, warranty.clone());
            }
            Action::Close { closed } => next.closed = *closed,
            Action::Backorder(backorder) => {
                let line = purchase
                    .draft
                    .lines
                    .iter()
                    .find(|l| l.id == backorder.line_id)
                    .ok_or("backorder invoice line does not exist")?;
                let coverage = self
                    .coverage(purchase, ledger)?
                    .into_iter()
                    .find(|r| r.line_id == line.id)
                    .ok_or("line coverage missing")?;
                let requested = if line.kind == LineKind::Set {
                    backorder.copies
                } else if coverage.unit == "mg" {
                    backorder.weight_mg
                } else {
                    backorder.pieces
                };
                if requested
                    .zip(amount(coverage.outstanding))
                    .is_some_and(|(b, o)| b > o)
                {
                    return Err("backorder exceeds outstanding acquisition quantity".into());
                }
                next.backorders.insert(backorder.line_id, backorder.clone());
            }
            Action::Receipt(receipt) => {
                if (self.closed && receipt.purpose == Purpose::Purchase)
                    || receipt.lines.is_empty()
                    || receipt.lines.len() > 10_000
                    || !crate::purchases::valid_date(&receipt.effective_on)
                    || self.receipts.contains_key(&receipt.id)
                {
                    return Err("invalid receipt, duplicate identity or closed order".into());
                }
                if receipt
                    .shipment_id
                    .is_some_and(|id| !self.shipments.contains_key(&id))
                {
                    return Err("shipment does not exist".into());
                }
                if let Purpose::Warranty { claim_id } = receipt.purpose
                    && !self.warranties.contains_key(&claim_id)
                {
                    return Err("warranty claim does not exist".into());
                }
                let mut seen = std::collections::BTreeSet::new();
                let coverage = self.coverage(purchase, ledger)?;
                for item in &receipt.lines {
                    if !seen.insert(item.invoice_line_id) {
                        return Err("receipt invoice lines must be unique".into());
                    }
                    let line = purchase
                        .draft
                        .lines
                        .iter()
                        .find(|l| l.id == item.invoice_line_id)
                        .ok_or("receipt invoice line does not exist")?;
                    if receipt.purpose == Purpose::Purchase {
                        if purchase.status == crate::purchases::Status::Cancelled {
                            return Err("cancelled orders cannot receive purchases".into());
                        }
                        if matches!(item.stock, ReceivedStock::Sets { .. })
                            != (line.kind == LineKind::Set)
                        {
                            return Err("received stock kind does not match order line".into());
                        }
                        let outstanding = coverage
                            .iter()
                            .find(|r| r.line_id == line.id)
                            .ok_or("line coverage missing")?
                            .outstanding;
                        let quantity = match &item.stock {
                            ReceivedStock::Sets { copies } => {
                                if copies.is_empty() {
                                    Measure::Unknown
                                } else {
                                    Measure::Known(copies.len() as u64)
                                }
                            }
                            ReceivedStock::Batch {
                                pieces, weight_mg, ..
                            } => {
                                if line.pieces == Measure::Unknown
                                    && line.weight_mg != Measure::Unknown
                                {
                                    *weight_mg
                                } else {
                                    *pieces
                                }
                            }
                        };
                        if amount(quantity)
                            .zip(amount(outstanding))
                            .is_some_and(|(q, o)| q > o)
                        {
                            return Err("receipt exceeds outstanding ordered quantity; correct order or use a linked warranty receipt".into());
                        }
                        if outstanding == Measure::Known(0) || quantity == Measure::Known(0) {
                            return Err(
                                "receipt must add actual quantity or explicit unknown stock".into(),
                            );
                        }
                    }
                    if let ReceivedStock::Sets { copies } = &item.stock
                        && (copies.len() > 10_000
                            || copies.iter().any(|c| c.label.trim().is_empty()))
                    {
                        return Err("invalid receipt copy labels/count".into());
                    }
                }
                next.receipts.insert(receipt.id, receipt.clone());
                next.receipt_operations.insert(receipt.id, command.id);
            }
            Action::ReceiptCorrection {
                receipt_operation_id,
                ..
            } => {
                let owns_receipt = ledger.operations().iter().any(|operation| {
                    operation.id == *receipt_operation_id
                        && matches!(&operation.body,
                            Body::Receive { purchase_id, .. } | Body::Delivery { purchase_id, .. }
                            if *purchase_id == purchase.id)
                });
                if !owns_receipt {
                    return Err("receipt does not belong to this order".into());
                }
            }
        }
        next.revision = next
            .revision
            .checked_add(1)
            .ok_or("fulfilment revision overflow")?;
        Ok(next)
    }
    pub fn operation(command: &Command, ledger: &Ledger) -> Option<crate::ledger::Operation> {
        let body = match &command.action {
            Action::SupplierReturn(crate::supplier_returns::Action::Dispatch {
                target,
                effective_on,
                ..
            }) => match target {
                crate::supplier_returns::Target::Holding {
                    holding_id,
                    expected_revision,
                    portion,
                    sources,
                } => Body::SupplierReturn {
                    holding_id: *holding_id,
                    expected_revision: *expected_revision,
                    portion: *portion,
                    effective_on: effective_on.clone(),
                    sources: sources.clone(),
                },
                crate::supplier_returns::Target::Copy(e) => Body::Lifecycle(e.clone()),
            },
            Action::SupplierReturn(crate::supplier_returns::Action::CorrectDispatch {
                dispatch_operation_id,
                effective_on,
                ..
            }) => {
                match ledger
                    .operations()
                    .iter()
                    .find(|op| op.id == *dispatch_operation_id)
                    .map(|op| &op.body)
                {
                    Some(Body::Lifecycle(e)) => Body::Lifecycle(crate::physical::Lifecycle {
                        copy_id: e.copy_id,
                        expected_revision: ledger.lifecycle(e.copy_id).revision,
                        status: crate::physical::Status::Active,
                        effective_on: effective_on.clone(),
                        proceeds_cents: None,
                        recipient: None,
                        notes: command.reason.clone(),
                        parts: vec![],
                        corrects: Some(*dispatch_operation_id),
                    }),
                    _ => Body::SupplierReturnCorrection {
                        dispatch_operation_id: *dispatch_operation_id,
                    },
                }
            }
            Action::Receipt(r) => Body::Delivery {
                receipt_id: r.id,
                purchase_id: command.purchase_id,
                lines: r.lines.clone(),
            },
            Action::ReceiptCorrection {
                receipt_operation_id,
                reverse,
            } => Body::ReceiptCorrection {
                receipt_operation_id: *receipt_operation_id,
                expected_inventory_sequence: ledger.operations().len() as u64,
                reverse: *reverse,
            },
            _ => return None,
        };
        Some(crate::ledger::Operation {
            id: command.id,
            request_key: command.request_key.clone(),
            actor: command.actor.clone(),
            reason: command.reason.clone(),
            body,
        })
    }
}
