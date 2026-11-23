//! Globally ordered stock and receipt facts. Purchase metadata is separate from
//! replayed ownership. Identification creates a fresh child with source provenance.

use crate::stock;
use crate::verification::{self, Action as VerificationAction};
use crate::{
    inventory::{AssemblyState, Balance, Measure},
    journal::{Action, Command, Holding, JournalError, MeasureChanges},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, error::Error, fmt};
use uuid::Uuid;

/// Only RFC UUIDv7 domain identities enter the ledger, including via JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "Uuid", into = "Uuid")]
pub struct Id(Uuid);

impl Id {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}
impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}
impl TryFrom<Uuid> for Id {
    type Error = &'static str;
    fn try_from(value: Uuid) -> Result<Self, Self::Error> {
        if value.get_version_num() == 7 && value.get_variant() == uuid::Variant::RFC4122 {
            Ok(Self(value))
        } else {
            Err("domain identity must be an RFC UUIDv7")
        }
    }
}
impl From<Id> for Uuid {
    fn from(value: Id) -> Self {
        value.0
    }
}
impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub id: Id,
    pub request_key: String,
    pub actor: String,
    pub reason: String,
    pub body: Body,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceivedCopy {
    pub id: Id,
    pub label: String,
    pub assembly: AssemblyState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReceivedStock {
    Batch {
        holding_id: Id,
        pieces: Measure,
        weight_mg: Measure,
    },
    Sets {
        copies: Vec<ReceivedCopy>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceivedLine {
    pub invoice_line_id: Id,
    pub stock: ReceivedStock,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetCopy {
    pub id: Id,
    pub invoice_line_id: Id,
    pub label: String,
    pub assembly: AssemblyState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Body {
    SupplierReturn {
        holding_id: Id,
        expected_revision: u64,
        portion: crate::physical::Portion,
        effective_on: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        sources: Vec<crate::physical::AcquisitionPortion>,
    },
    SupplierReturnCorrection {
        dispatch_operation_id: Id,
    },
    Delivery {
        receipt_id: Id,
        purchase_id: Id,
        lines: Vec<ReceivedLine>,
    },
    ReceiptCorrection {
        receipt_operation_id: Id,
        expected_inventory_sequence: u64,
        reverse: bool,
    },
    Split {
        source_id: Id,
        child_id: Id,
        expected_revision: u64,
        portion: crate::physical::Portion,
    },
    Merge {
        sources: Vec<crate::physical::Source>,
        target_id: Id,
    },
    Detach {
        holding_id: Id,
        expected_revision: u64,
    },
    Lifecycle(crate::physical::Lifecycle),
    AddCopies {
        purchase_id: Id,
        invoice_line_id: Id,
        copies: Vec<ReceivedCopy>,
    },
    Stock {
        holding_id: Id,
        expected_revision: u64,
        action: stock::Action,
    },
    Verify {
        copy_id: Id,
        expected_revision: u64,
        action: VerificationAction,
    },
    Receive {
        purchase_id: Id,
        lines: Vec<ReceivedLine>,
    },
    /// Explicit observation of newly introduced, undamaged standalone stock.
    Create {
        holding_id: Id,
        pieces: Measure,
        weight_mg: Measure,
    },
    Change {
        holding_id: Id,
        expected_revision: u64,
        pieces: MeasureChanges,
        weight_mg: MeasureChanges,
        assembly: Option<AssemblyState>,
    },
    Identify {
        source_id: Id,
        child_id: Id,
        expected_revision: u64,
        pieces: Option<u64>,
        weight_mg: Option<u64>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    pub sequence: u64,
    pub holdings: BTreeMap<Id, Holding>,
    pub set_copies: BTreeMap<Id, SetCopy>,
}

#[derive(Debug, Clone)]
struct ReceiptEffect {
    reversed: bool,
    snapshots: BTreeMap<Id, Holding>,
    last_event: Id,
}
#[derive(Debug, Default)]
pub struct Ledger {
    holdings: BTreeMap<Id, Holding>,
    operations: Vec<Operation>,
    outcomes: Vec<Outcome>,
    set_copies: BTreeMap<Id, SetCopy>,
    receipts: BTreeMap<Id, Id>,
    deliveries: BTreeMap<Id, Id>,
    receipt_corrections: BTreeMap<Id, ReceiptEffect>,
    provenance: BTreeMap<Id, Id>,
    verification: BTreeMap<Id, verification::State>,
    set_contents: BTreeMap<Id, Id>,
    classification: BTreeMap<Id, stock::Classification>,
    lifecycle: BTreeMap<Id, crate::physical::CopyState>,
    lifecycle_effects: BTreeMap<Id, crate::physical::Effect>,
    return_effects: BTreeMap<Id, (Id, Holding, Holding)>,
    corrected_returns: std::collections::BTreeSet<Id>,
}

impl Ledger {
    /// Replay balances, including closed historical residue. Use current_holdings
    /// for current inventory totals so physical-empty closures are respected.
    pub fn holdings(&self) -> &BTreeMap<Id, Holding> {
        &self.holdings
    }
    pub fn current_holdings(&self) -> BTreeMap<Id, Holding> {
        self.holdings
            .iter()
            .filter(|(id, _)| {
                !self
                    .classification
                    .get(id)
                    .is_some_and(|state| state.complete)
            })
            .map(|(id, h)| (*id, *h))
            .collect()
    }
    pub fn classification(&self) -> &BTreeMap<Id, stock::Classification> {
        &self.classification
    }
    /// Immutable receipt facts; use `assembly` for the current whole-set state.
    pub fn set_copies(&self) -> &BTreeMap<Id, SetCopy> {
        &self.set_copies
    }
    pub fn assembly(&self, copy_id: Id) -> Option<AssemblyState> {
        self.verification
            .get(&copy_id)
            .map(|state| state.assembly)
            .or_else(|| self.set_copies.get(&copy_id).map(|copy| copy.assembly))
    }
    pub fn copy_label(&self, copy_id: Id) -> Option<&str> {
        let copy = self.set_copies.get(&copy_id)?;
        Some(
            self.verification
                .get(&copy_id)
                .and_then(|state| state.label.as_deref())
                .unwrap_or(&copy.label),
        )
    }
    pub fn receipts(&self) -> &BTreeMap<Id, Id> {
        &self.receipts
    }
    pub fn provenance(&self) -> &BTreeMap<Id, Id> {
        &self.provenance
    }
    pub fn verification(&self) -> &BTreeMap<Id, verification::State> {
        &self.verification
    }
    pub fn set_contents(&self) -> &BTreeMap<Id, Id> {
        &self.set_contents
    }
    pub fn deliveries(&self) -> &BTreeMap<Id, Id> {
        &self.deliveries
    }
    pub fn receipt_reversed(&self, id: Id) -> bool {
        self.receipt_corrections
            .get(&id)
            .is_some_and(|e| e.reversed)
    }
    pub fn lifecycle_effects(&self) -> &BTreeMap<Id, crate::physical::Effect> {
        &self.lifecycle_effects
    }
    pub fn lifecycle(&self, copy_id: Id) -> crate::physical::CopyState {
        self.lifecycle.get(&copy_id).cloned().unwrap_or_default()
    }
    pub fn original_contents(&self) -> BTreeMap<Id, Id> {
        self.verification
            .iter()
            .flat_map(|(copy, state)| {
                state
                    .lines
                    .values()
                    .filter(|line| line.verified.is_some())
                    .map(move |line| (line.holding_id, *copy))
            })
            .collect()
    }
    pub fn outcomes(&self) -> &[Outcome] {
        &self.outcomes
    }
    pub fn operations(&self) -> &[Operation] {
        &self.operations
    }

    pub fn append(&mut self, op: Operation) -> Result<Outcome, LedgerError> {
        if let Some(index) = self
            .operations
            .iter()
            .position(|old| old.request_key == op.request_key)
        {
            return if self.operations[index] == op {
                Ok(self.outcomes[index].clone())
            } else {
                Err(LedgerError::Conflict)
            };
        }
        if self.operations.iter().any(|old| old.id == op.id) {
            return Err(LedgerError::Conflict);
        }
        if op.actor.trim().is_empty()
            || op.reason.trim().is_empty()
            || op.request_key.trim().is_empty()
        {
            return Err(LedgerError::Journal(JournalError::MissingAuditContext));
        }
        let mut added_copies = BTreeMap::new();
        let mut added_provenance = BTreeMap::new();
        let mut receipt = None;
        let mut delivered = None;
        let mut receipt_correction = None;
        let mut copy_life_updates = Vec::new();
        let mut verification_update = None;
        let mut content_update = None;
        let mut classification_update = None;
        let mut detached = Vec::new();
        let mut attached = BTreeMap::new();
        let mut life_update = None;
        let mut life_effect = None;
        let mut return_effect = None;
        let mut corrected_return = None;
        let changes = match &op.body {
            Body::SupplierReturn {
                holding_id,
                expected_revision,
                portion,
                effective_on,
                sources,
            } => {
                if !crate::purchases::valid_date(effective_on)
                    || self.set_contents.contains_key(holding_id)
                    || self
                        .classification
                        .get(holding_id)
                        .is_some_and(|s| s.complete)
                {
                    return Err(LedgerError::InvalidStockAction(
                        "return needs loose active stock and a valid dispatch date",
                    ));
                }
                let source_layers = crate::source_layers::project(self);
                crate::source_layers::validate_return(
                    source_layers.get(holding_id).unwrap_or(&BTreeMap::new()),
                    *portion,
                    sources,
                )?;
                let before = *self
                    .holdings
                    .get(holding_id)
                    .ok_or(LedgerError::MissingHolding)?;
                let (after, _) = crate::physical::split(
                    before,
                    *expected_revision,
                    *portion,
                    &op.actor,
                    &op.reason,
                    &op.request_key,
                )?;
                return_effect = Some((op.id, (*holding_id, before, after)));
                BTreeMap::from([(*holding_id, after)])
            }
            Body::SupplierReturnCorrection {
                dispatch_operation_id,
            } => {
                let (id, before, after) = self.return_effects.get(dispatch_operation_id).ok_or(
                    LedgerError::InvalidStockAction("return dispatch does not exist"),
                )?;
                if self.corrected_returns.contains(dispatch_operation_id)
                    || self.holdings.get(id) != Some(after)
                {
                    return Err(LedgerError::InvalidStockAction(
                        "return correction blocked: stock changed after dispatch or dispatch already corrected",
                    ));
                }
                let restored = crate::physical::replace(
                    *after,
                    before.pieces,
                    before.weight_mg,
                    before.assembly,
                )?;
                corrected_return = Some(*dispatch_operation_id);
                BTreeMap::from([(*id, restored)])
            }
            Body::Split {
                source_id,
                child_id,
                expected_revision,
                portion,
            } => {
                self.require_new(*child_id)?;
                let h = *self
                    .holdings
                    .get(source_id)
                    .ok_or(LedgerError::MissingHolding)?;
                if self
                    .classification
                    .get(source_id)
                    .is_some_and(|c| c.complete)
                {
                    return Err(LedgerError::ClosedClassification);
                }
                let (remaining, child) = crate::physical::split(
                    h,
                    *expected_revision,
                    *portion,
                    &op.actor,
                    &op.reason,
                    &op.request_key,
                )?;
                if let Some(line) = self.provenance.get(source_id) {
                    added_provenance.insert(*child_id, *line);
                }
                BTreeMap::from([(*source_id, remaining), (*child_id, child)])
            }
            Body::Merge { sources, target_id } => {
                self.require_new(*target_id)?;
                if sources.len() < 2 || sources.len() > 1000 {
                    return Err(LedgerError::InvalidStockAction(
                        "merge requires 2..1000 source holdings",
                    ));
                }
                let mut ids = std::collections::BTreeSet::new();
                let mut hs = vec![];
                let mut changes = BTreeMap::new();
                for source in sources {
                    if !ids.insert(source.holding_id)
                        || self.set_contents.contains_key(&source.holding_id)
                    {
                        return Err(LedgerError::InvalidStockAction(
                            "detach set contents before merging; source identities must be unique",
                        ));
                    }
                    if self
                        .classification
                        .get(&source.holding_id)
                        .is_some_and(|c| c.complete)
                    {
                        return Err(LedgerError::ClosedClassification);
                    }
                    let h = *self
                        .holdings
                        .get(&source.holding_id)
                        .ok_or(LedgerError::MissingHolding)?;
                    if h.revision != source.expected_revision {
                        return Err(LedgerError::Journal(JournalError::StaleRevision));
                    }
                    hs.push(h);
                    changes.insert(source.holding_id, crate::physical::empty(h)?);
                }
                changes.insert(*target_id, crate::physical::merged(&hs)?);
                changes
            }
            Body::Detach {
                holding_id,
                expected_revision,
            } => {
                if !self.set_contents.contains_key(holding_id) {
                    return Err(LedgerError::InvalidStockAction(
                        "holding is not currently assigned to a copy",
                    ));
                }
                let h = *self
                    .holdings
                    .get(holding_id)
                    .ok_or(LedgerError::MissingHolding)?;
                if h.revision != *expected_revision {
                    return Err(LedgerError::Journal(JournalError::StaleRevision));
                }
                detached.push(*holding_id);
                BTreeMap::from([(
                    *holding_id,
                    crate::physical::replace(h, h.pieces, h.weight_mg, AssemblyState::Loose)?,
                )])
            }
            Body::Lifecycle(event) => {
                use crate::physical::{CopyState, Effect, Handling, Status};
                if !self.set_copies.contains_key(&event.copy_id)
                    || !crate::purchases::valid_date(&event.effective_on)
                {
                    return Err(LedgerError::InvalidStockAction(
                        "copy and valid effective date are required",
                    ));
                }
                let old = self.lifecycle(event.copy_id);
                if old.revision != event.expected_revision {
                    return Err(LedgerError::Journal(JournalError::StaleRevision));
                }
                if event.proceeds_cents.is_some() && event.status != Status::Sold {
                    return Err(LedgerError::InvalidStockAction(
                        "sale proceeds only belong to a Sold event",
                    ));
                }
                let mut changes = BTreeMap::new();
                if let Some(corrects) = event.corrects {
                    if event.status != Status::Active
                        || old.status == Status::Active
                        || old.last_event != Some(corrects)
                        || !event.parts.is_empty()
                    {
                        return Err(LedgerError::InvalidStockAction(
                            "correction must reference the last terminal event and restore Active",
                        ));
                    }
                    let effect = self
                        .lifecycle_effects
                        .get(&corrects)
                        .ok_or(LedgerError::Conflict)?;
                    if effect.copy_id != event.copy_id
                        || effect
                            .after_holdings
                            .iter()
                            .any(|(id, h)| self.holdings.get(id) != Some(h))
                    {
                        return Err(LedgerError::InvalidStockAction(
                            "reversal blocked: affected holdings changed after lifecycle event",
                        ));
                    }
                    for (id, after) in &effect.after_holdings {
                        let next = if let Some(before) = effect.before_holdings.get(id) {
                            crate::physical::replace(
                                *after,
                                before.pieces,
                                before.weight_mg,
                                before.assembly,
                            )?
                        } else {
                            crate::physical::empty(*after)?
                        };
                        changes.insert(*id, next);
                    }
                    attached = effect.parents.clone();
                } else {
                    if old.status != Status::Active || event.status == Status::Active {
                        return Err(LedgerError::InvalidStockAction(
                            "terminal transitions require Active; correct prior event before another disposition",
                        ));
                    }
                    let parents: BTreeMap<Id, Id> = self
                        .set_contents
                        .iter()
                        .filter(|(_, copy)| **copy == event.copy_id)
                        .map(|(h, c)| (*h, *c))
                        .collect();
                    let mut seen = std::collections::BTreeSet::new();
                    let mut before = BTreeMap::new();
                    for part in &event.parts {
                        if !seen.insert(part.holding_id) || !parents.contains_key(&part.holding_id)
                        {
                            return Err(LedgerError::InvalidStockAction(
                                "disposition must explicitly cover each currently linked holding once",
                            ));
                        }
                        let h = *self
                            .holdings
                            .get(&part.holding_id)
                            .ok_or(LedgerError::MissingHolding)?;
                        if h.revision != part.expected_revision {
                            return Err(LedgerError::Journal(JournalError::StaleRevision));
                        }
                        before.insert(part.holding_id, h);
                        detached.push(part.holding_id);
                        let after = match part.handling {
                            Handling::TransferAll => crate::physical::empty(h)?,
                            Handling::KeepAll => crate::physical::replace(
                                h,
                                h.pieces,
                                h.weight_mg,
                                AssemblyState::Loose,
                            )?,
                            Handling::Custom { child_id, keep } => {
                                self.require_new(child_id)?;
                                if changes.contains_key(&child_id)
                                    || parents.contains_key(&child_id)
                                {
                                    return Err(LedgerError::Conflict);
                                }
                                let (_, child) = crate::physical::split(
                                    h,
                                    part.expected_revision,
                                    keep,
                                    &op.actor,
                                    &op.reason,
                                    &op.request_key,
                                )?;
                                changes.insert(child_id, child);
                                if let Some(line) = self.provenance.get(&part.holding_id) {
                                    added_provenance.insert(child_id, *line);
                                }
                                crate::physical::empty(h)?
                            }
                        };
                        changes.insert(part.holding_id, after);
                    }
                    if seen.len() != parents.len() {
                        return Err(LedgerError::InvalidStockAction(
                            "choose part disposition for every current linked holding",
                        ));
                    }
                    life_effect = Some((
                        op.id,
                        Effect {
                            copy_id: event.copy_id,
                            before: old.clone(),
                            before_holdings: before,
                            after_holdings: changes.clone(),
                            parents,
                        },
                    ));
                }
                life_update = Some((
                    event.copy_id,
                    CopyState {
                        revision: old.revision.checked_add(1).ok_or(LedgerError::Conflict)?,
                        status: event.status,
                        last_event: Some(op.id),
                    },
                ));
                changes
            }
            Body::AddCopies {
                purchase_id,
                invoice_line_id,
                copies,
            } => {
                let receipt_id = self
                    .receipts
                    .get(purchase_id)
                    .ok_or(LedgerError::Conflict)?;
                let valid=self.operations.iter().find(|op|op.id==*receipt_id).is_some_and(|op|matches!(&op.body,Body::Receive{lines,..} if lines.iter().any(|line|line.invoice_line_id==*invoice_line_id && matches!(line.stock,ReceivedStock::Sets{..}))));
                if !valid || copies.is_empty() || copies.len() > 10_000 {
                    return Err(LedgerError::Conflict);
                }
                let mut ids = std::collections::BTreeSet::new();
                for copy in copies {
                    self.require_new(copy.id)?;
                    if !ids.insert(copy.id) || copy.label.trim().is_empty() {
                        return Err(LedgerError::Conflict);
                    }
                    added_copies.insert(
                        copy.id,
                        SetCopy {
                            id: copy.id,
                            invoice_line_id: *invoice_line_id,
                            label: copy.label.clone(),
                            assembly: copy.assembly,
                        },
                    );
                }
                BTreeMap::new()
            }
            Body::Stock {
                holding_id,
                expected_revision,
                action,
            } => {
                let current = *self
                    .holdings
                    .get(holding_id)
                    .ok_or(LedgerError::MissingHolding)?;
                let mut classification = self
                    .classification
                    .get(holding_id)
                    .cloned()
                    .unwrap_or_default();
                let (pieces, weight_mg) = match action {
                    stock::Action::Adjust(adjustment) => {
                        if classification.complete {
                            return Err(LedgerError::ClosedClassification);
                        }
                        if self.set_contents.contains_key(holding_id)
                            && matches!(adjustment,stock::Adjustment::Correction {pieces,..} if pieces.owned!=Action::Unchanged)
                        {
                            return Err(LedgerError::ReconciliationRequired);
                        }
                        stock::changes(current, adjustment)
                            .map_err(LedgerError::InvalidStockAction)?
                    }
                    stock::Action::Complete {
                        confirm_physical_empty,
                    } => {
                        if classification.complete || self.set_contents.contains_key(holding_id) {
                            return Err(LedgerError::Conflict);
                        }
                        stock::can_complete(current, *confirm_physical_empty)
                            .map_err(LedgerError::InvalidStockAction)?;
                        classification.complete = true;
                        classification.last_completion = Some(stock::Completion {
                            operation_id: op.id,
                            physical_empty_confirmed: *confirm_physical_empty,
                        });
                        classification_update = Some((*holding_id, classification));
                        (MeasureChanges::default(), MeasureChanges::default())
                    }
                    stock::Action::Reopen => {
                        if !classification.complete {
                            return Err(LedgerError::Conflict);
                        }
                        classification.complete = false;
                        classification.last_reopen = Some(op.id);
                        classification_update = Some((*holding_id, classification));
                        (MeasureChanges::default(), MeasureChanges::default())
                    }
                };
                let next = current
                    .apply(&Command {
                        request_key: op.request_key.clone(),
                        expected_revision: *expected_revision,
                        actor: op.actor.clone(),
                        reason: op.reason.clone(),
                        pieces,
                        weight_mg,
                        assembly: None,
                    })
                    .map_err(LedgerError::Journal)?;
                BTreeMap::from([(*holding_id, next)])
            }
            Body::Verify {
                copy_id,
                expected_revision,
                action,
            } => {
                if self.lifecycle(*copy_id).status != crate::physical::Status::Active
                    && !matches!(action, VerificationAction::Rename { .. })
                {
                    return Err(LedgerError::InvalidStockAction(
                        "inactive copy cannot change verification or assembly",
                    ));
                }
                let copy = self
                    .set_copies
                    .get(copy_id)
                    .ok_or(LedgerError::MissingHolding)?;
                let mut state = self
                    .verification
                    .get(copy_id)
                    .cloned()
                    .unwrap_or_else(|| verification::State::new(*copy_id, copy.assembly));
                if state.revision != *expected_revision {
                    return Err(LedgerError::Journal(JournalError::StaleRevision));
                }
                let mut changes = BTreeMap::new();
                if state.lines.values().any(|line| {
                    line.verified.is_some() && !self.set_contents.contains_key(&line.holding_id)
                }) && !matches!(action, VerificationAction::Rename { .. })
                {
                    return Err(LedgerError::InvalidStockAction(
                        "copy has detached contents; manage reassignment through stock allocations, not recounting historical lines",
                    ));
                }
                match action {
                    VerificationAction::Identity {
                        line_id,
                        definition_id,
                        color_id,
                    } => {
                        let line = state
                            .lines
                            .get_mut(line_id)
                            .ok_or(LedgerError::MissingHolding)?;
                        if line.retired {
                            return Err(LedgerError::Conflict);
                        }
                        if line.definition_id != *definition_id || line.color_id != *color_id {
                            line.definition_id = *definition_id;
                            line.color_id = *color_id;
                            line.finished = false;
                            state.completed = false;
                        }
                    }
                    VerificationAction::Active { line_id, active } => {
                        let line = state
                            .lines
                            .get_mut(line_id)
                            .ok_or(LedgerError::MissingHolding)?;
                        if line.retired == *active {
                            line.retired = !*active;
                            line.finished = false;
                            state.completed = false;
                        }
                    }
                    VerificationAction::AddExpected { lines } => {
                        if lines.is_empty() {
                            return Err(LedgerError::Conflict);
                        }
                        self.add_expected(&mut state, lines)?;
                    }
                    VerificationAction::Rename { label } => {
                        if label.trim().is_empty()
                            || label.len() > 200
                            || label.chars().any(char::is_control)
                        {
                            return Err(LedgerError::Conflict);
                        }
                        state.label = Some(label.trim().to_owned());
                    }
                    VerificationAction::CopyExpected {
                        source_copy_id,
                        lines,
                    } => {
                        if source_copy_id == copy_id {
                            return Err(LedgerError::Conflict);
                        }
                        let source = self
                            .verification
                            .get(source_copy_id)
                            .ok_or(LedgerError::MissingHolding)?;
                        if source.lines.values().all(|line| line.retired)
                            || source.lines.values().filter(|line| !line.retired).count()
                                != lines.len()
                        {
                            return Err(LedgerError::Conflict);
                        }
                        for (source, line) in source
                            .lines
                            .values()
                            .filter(|line| !line.retired)
                            .zip(lines)
                        {
                            if source.definition_id != line.definition_id
                                || source.color_id != line.color_id
                                || source.expected != line.expected
                            {
                                return Err(LedgerError::Conflict);
                            }
                        }
                        self.add_expected(&mut state, lines)?;
                    }
                    VerificationAction::AddLine {
                        line_id,
                        holding_id,
                        definition_id,
                        color_id,
                        expected,
                    } => {
                        self.add_expected(
                            &mut state,
                            &[verification::ExpectedLine {
                                line_id: *line_id,
                                holding_id: *holding_id,
                                definition_id: *definition_id,
                                color_id: *color_id,
                                expected: *expected,
                            }],
                        )?;
                    }
                    VerificationAction::Expected { line_id, expected } => {
                        let line = state
                            .lines
                            .get_mut(line_id)
                            .ok_or(LedgerError::MissingHolding)?;
                        if line.retired {
                            return Err(LedgerError::Conflict);
                        }
                        if line.expected != *expected {
                            line.expected = *expected;
                            line.finished = false;
                            state.completed = false;
                        }
                    }
                    VerificationAction::Count { line_id, total }
                    | VerificationAction::ReconcileCount { line_id, total, .. } => {
                        let line = state
                            .lines
                            .get_mut(line_id)
                            .ok_or(LedgerError::MissingHolding)?;
                        if line.retired {
                            return Err(LedgerError::Conflict);
                        }
                        let reconcile = if let VerificationAction::ReconcileCount {
                            expected_holding_revision,
                            expected_owned,
                            ..
                        } = action
                        {
                            let current = self
                                .holdings
                                .get(&line.holding_id)
                                .ok_or(LedgerError::MissingHolding)?;
                            if current.revision != *expected_holding_revision
                                || current.pieces.owned() != *expected_owned
                            {
                                return Err(LedgerError::Journal(JournalError::StaleRevision));
                            }
                            true
                        } else {
                            false
                        };
                        if !reconcile
                            && let Some(old) = line.verified
                            && self
                                .holdings
                                .get(&line.holding_id)
                                .ok_or(LedgerError::MissingHolding)?
                                .pieces
                                .owned()
                                != Measure::Known(old)
                        {
                            return Err(LedgerError::ReconciliationRequired);
                        }
                        if line.verified != Some(*total)
                            || reconcile
                                && self.holdings[&line.holding_id].pieces.owned()
                                    != Measure::Known(*total)
                        {
                            let holding = if line.verified.is_some() {
                                let current = *self
                                    .holdings
                                    .get(&line.holding_id)
                                    .ok_or(LedgerError::MissingHolding)?;
                                current
                                    .apply(&Command {
                                        request_key: op.request_key.clone(),
                                        expected_revision: current.revision,
                                        actor: op.actor.clone(),
                                        reason: op.reason.clone(),
                                        pieces: MeasureChanges {
                                            owned: Action::SetMeasured(*total),
                                            damaged: Action::Unchanged,
                                        },
                                        weight_mg: MeasureChanges::default(),
                                        assembly: None,
                                    })
                                    .map_err(LedgerError::Journal)?
                            } else {
                                if self.holdings.contains_key(&line.holding_id)
                                    || self.set_copies.contains_key(&line.holding_id)
                                {
                                    return Err(LedgerError::Conflict);
                                }
                                let mut holding =
                                    new_holding(Measure::Known(*total), Measure::Unknown);
                                holding.assembly = state.assembly;
                                added_provenance.insert(line.holding_id, copy.invoice_line_id);
                                content_update = Some((line.holding_id, *copy_id));
                                holding
                            };
                            changes.insert(line.holding_id, holding);
                            line.verified = Some(*total);
                            line.finished = false;
                            state.completed = false;
                        }
                    }
                    VerificationAction::Finish { line_id } => {
                        let line = state
                            .lines
                            .get_mut(line_id)
                            .ok_or(LedgerError::MissingHolding)?;
                        if line.retired || line.verified.is_none() {
                            return Err(LedgerError::UnfinishedVerification);
                        }
                        line.finished = true;
                    }
                    VerificationAction::Complete => {
                        if state.lines.values().all(|line| line.retired)
                            || state
                                .lines
                                .values()
                                .any(|line| !line.retired && !line.finished)
                        {
                            return Err(LedgerError::UnfinishedVerification);
                        }
                        state.completed = true;
                    }
                    VerificationAction::Assembly { state: assembly } => {
                        if state.assembly != *assembly {
                            for line in state.lines.values().filter(|l| l.verified.is_some()) {
                                let current = *self
                                    .holdings
                                    .get(&line.holding_id)
                                    .ok_or(LedgerError::MissingHolding)?;
                                let next = current
                                    .apply(&Command {
                                        request_key: op.request_key.clone(),
                                        expected_revision: current.revision,
                                        actor: op.actor.clone(),
                                        reason: op.reason.clone(),
                                        pieces: MeasureChanges::default(),
                                        weight_mg: MeasureChanges::default(),
                                        assembly: Some(*assembly),
                                    })
                                    .map_err(LedgerError::Journal)?;
                                changes.insert(line.holding_id, next);
                            }
                            state.assembly = *assembly;
                        }
                    }
                }
                state.revision = state
                    .revision
                    .checked_add(1)
                    .ok_or(LedgerError::Journal(JournalError::OutOfRange))?;
                verification_update = Some((*copy_id, state));
                changes
            }
            Body::Receive { purchase_id, lines }
            | Body::Delivery {
                purchase_id, lines, ..
            } => {
                let is_delivery = matches!(op.body, Body::Delivery { .. });
                if (!is_delivery && self.receipts.contains_key(purchase_id)) || lines.is_empty() {
                    return Err(LedgerError::Conflict);
                }
                let mut changes = BTreeMap::new();
                let mut line_ids = std::collections::BTreeSet::new();
                let mut entity_ids = std::collections::BTreeSet::new();
                for line in lines {
                    if !line_ids.insert(line.invoice_line_id) {
                        return Err(LedgerError::Conflict);
                    }
                    match &line.stock {
                        ReceivedStock::Batch {
                            holding_id,
                            pieces,
                            weight_mg,
                        } => {
                            self.require_new(*holding_id)?;
                            if !entity_ids.insert(*holding_id) {
                                return Err(LedgerError::Conflict);
                            }
                            changes.insert(*holding_id, new_holding(*pieces, *weight_mg));
                            added_provenance.insert(*holding_id, line.invoice_line_id);
                        }
                        ReceivedStock::Sets { copies } => {
                            for copy in copies {
                                self.require_new(copy.id)?;
                                if !entity_ids.insert(copy.id) {
                                    return Err(LedgerError::Conflict);
                                }
                                added_copies.insert(
                                    copy.id,
                                    SetCopy {
                                        id: copy.id,
                                        invoice_line_id: line.invoice_line_id,
                                        label: copy.label.clone(),
                                        assembly: copy.assembly,
                                    },
                                );
                            }
                        }
                    }
                }
                if let Body::Delivery { receipt_id, .. } = op.body {
                    if self.deliveries.contains_key(&receipt_id) {
                        return Err(LedgerError::Conflict);
                    }
                    delivered = Some((receipt_id, *purchase_id));
                } else {
                    receipt = Some((*purchase_id, op.id));
                }
                changes
            }
            Body::ReceiptCorrection {
                receipt_operation_id,
                expected_inventory_sequence,
                reverse,
            } => {
                if *expected_inventory_sequence != self.operations.len() as u64 {
                    return Err(LedgerError::Journal(JournalError::StaleRevision));
                }
                let original = self
                    .operations
                    .iter()
                    .find(|o| o.id == *receipt_operation_id)
                    .ok_or(LedgerError::Conflict)?;
                let mut lines = match &original.body {
                    Body::Receive { lines, .. } | Body::Delivery { lines, .. } => lines.clone(),
                    _ => {
                        return Err(LedgerError::InvalidStockAction(
                            "receipt correction must reference an actual receipt",
                        ));
                    }
                };
                if let Body::Receive { purchase_id, .. } = &original.body {
                    for extra in &self.operations {
                        if let Body::AddCopies {
                            purchase_id: p,
                            invoice_line_id,
                            copies,
                        } = &extra.body
                            && p == purchase_id
                            && let Some(line) = lines
                                .iter_mut()
                                .find(|l| l.invoice_line_id == *invoice_line_id)
                            && let ReceivedStock::Sets { copies: originals } = &mut line.stock
                        {
                            originals.extend(copies.clone());
                        }
                    }
                }
                let prior = self.receipt_corrections.get(receipt_operation_id);
                if prior.is_some_and(|p| p.reversed) == *reverse {
                    return Err(LedgerError::InvalidStockAction(
                        "receipt already has requested reversal state",
                    ));
                }
                let mut changes = BTreeMap::new();
                let mut snapshots = BTreeMap::new();
                for line in &lines {
                    match &line.stock {
                        ReceivedStock::Batch {
                            holding_id,
                            pieces,
                            weight_mg,
                        } => {
                            let current = *self
                                .holdings
                                .get(holding_id)
                                .ok_or(LedgerError::MissingHolding)?;
                            let initial = new_holding(*pieces, *weight_mg);
                            let expected = prior
                                .and_then(|p| p.snapshots.get(holding_id))
                                .copied()
                                .unwrap_or(initial);
                            if current != expected {
                                return Err(LedgerError::InvalidStockAction(
                                    "receipt reversal blocked: received stock changed or moved",
                                ));
                            }
                            let next = if *reverse {
                                crate::physical::empty(current)?
                            } else {
                                crate::physical::replace(
                                    current,
                                    initial.pieces,
                                    initial.weight_mg,
                                    initial.assembly,
                                )?
                            };
                            changes.insert(*holding_id, next);
                            snapshots.insert(*holding_id, next);
                        }
                        ReceivedStock::Sets { copies } => {
                            for copy in copies {
                                let old = self.lifecycle(copy.id);
                                if self.set_contents.values().any(|id| *id == copy.id)
                                    || (*reverse && old.status != crate::physical::Status::Active)
                                    || (!*reverse
                                        && (old.status != crate::physical::Status::Void
                                            || prior.is_none_or(|p| {
                                                old.last_event != Some(p.last_event)
                                            })))
                                {
                                    return Err(LedgerError::InvalidStockAction(
                                        "receipt reversal blocked: copy has contents or later lifecycle events",
                                    ));
                                }
                                copy_life_updates.push((
                                    copy.id,
                                    crate::physical::CopyState {
                                        revision: old
                                            .revision
                                            .checked_add(1)
                                            .ok_or(LedgerError::Conflict)?,
                                        status: if *reverse {
                                            crate::physical::Status::Void
                                        } else {
                                            crate::physical::Status::Active
                                        },
                                        last_event: Some(op.id),
                                    },
                                ));
                            }
                        }
                    }
                }
                receipt_correction = Some((
                    *receipt_operation_id,
                    ReceiptEffect {
                        reversed: *reverse,
                        snapshots,
                        last_event: op.id,
                    },
                ));
                changes
            }
            Body::Create {
                holding_id,
                pieces,
                weight_mg,
            } => {
                let (holding_id, pieces, weight_mg) = (*holding_id, *pieces, *weight_mg);
                self.require_new(holding_id)?;
                BTreeMap::from([(holding_id, new_holding(pieces, weight_mg))])
            }
            Body::Change {
                holding_id,
                expected_revision,
                pieces,
                weight_mg,
                assembly,
            } => {
                let (holding_id, expected_revision, pieces, weight_mg, assembly) = (
                    *holding_id,
                    *expected_revision,
                    *pieces,
                    *weight_mg,
                    *assembly,
                );
                if self
                    .classification
                    .get(&holding_id)
                    .is_some_and(|state| state.complete)
                {
                    return Err(LedgerError::ClosedClassification);
                }
                if self.set_contents.contains_key(&holding_id) && assembly.is_some() {
                    return Err(LedgerError::Conflict);
                }
                let holding = *self
                    .holdings
                    .get(&holding_id)
                    .ok_or(LedgerError::MissingHolding)?;
                let next = holding
                    .apply(&Command {
                        request_key: op.request_key.clone(),
                        actor: op.actor.clone(),
                        reason: op.reason.clone(),
                        expected_revision,
                        pieces,
                        weight_mg,
                        assembly,
                    })
                    .map_err(LedgerError::Journal)?;
                BTreeMap::from([(holding_id, next)])
            }
            Body::Identify {
                source_id,
                child_id,
                expected_revision,
                pieces,
                weight_mg,
            } => {
                let (source_id, child_id, expected_revision, pieces, weight_mg) = (
                    *source_id,
                    *child_id,
                    *expected_revision,
                    *pieces,
                    *weight_mg,
                );
                self.require_new(child_id)?;
                if self
                    .classification
                    .get(&source_id)
                    .is_some_and(|state| state.complete)
                {
                    return Err(LedgerError::ClosedClassification);
                }
                if self.set_contents.contains_key(&source_id) {
                    return Err(LedgerError::UnsupportedTransfer);
                }
                if let Some(line_id) = self.provenance.get(&source_id) {
                    added_provenance.insert(child_id, *line_id);
                }
                let source = *self
                    .holdings
                    .get(&source_id)
                    .ok_or(LedgerError::MissingHolding)?;
                if source.assembly != AssemblyState::Loose
                    || source.pieces.damaged() != Measure::Known(0)
                    || source.weight_mg.damaged() != Measure::Known(0)
                {
                    return Err(LedgerError::UnsupportedTransfer);
                }
                if !pieces.is_some_and(|v| v > 0) && !weight_mg.is_some_and(|v| v > 0) {
                    return Err(LedgerError::EmptyTransfer);
                }
                let next = source
                    .apply(&Command {
                        request_key: op.request_key.clone(),
                        actor: op.actor.clone(),
                        reason: op.reason.clone(),
                        expected_revision,
                        pieces: MeasureChanges {
                            owned: extraction(source.pieces.owned(), pieces)?,
                            damaged: Action::Unchanged,
                        },
                        weight_mg: MeasureChanges {
                            owned: extraction(source.weight_mg.owned(), weight_mg)?,
                            damaged: Action::Unchanged,
                        },
                        assembly: None,
                    })
                    .map_err(LedgerError::Journal)?;
                BTreeMap::from([
                    (source_id, next),
                    (
                        child_id,
                        new_holding(
                            pieces.map_or(Measure::Unknown, Measure::Known),
                            weight_mg.map_or(Measure::Unknown, Measure::Known),
                        ),
                    ),
                ])
            }
        };
        let sequence = u64::try_from(self.operations.len())
            .ok()
            .and_then(|n| n.checked_add(1))
            .ok_or(LedgerError::Journal(JournalError::OutOfRange))?;
        let result = Outcome {
            sequence,
            holdings: changes,
            set_copies: added_copies,
        };
        for id in detached {
            self.set_contents.remove(&id);
        }
        self.set_contents.extend(attached);
        if let Some((id, state)) = life_update {
            self.lifecycle.insert(id, state);
        }
        if let Some((id, effect)) = life_effect {
            self.lifecycle_effects.insert(id, effect);
        }
        self.holdings.extend(result.holdings.clone());
        self.set_copies.extend(result.set_copies.clone());
        self.provenance.extend(added_provenance);
        if let Some((id, state)) = verification_update {
            self.verification.insert(id, state);
        }
        if let Some((id, state)) = classification_update {
            self.classification.insert(id, state);
        }
        if let Some((holding, copy)) = content_update {
            self.set_contents.insert(holding, copy);
        }
        if let Some((id, purchase)) = delivered {
            self.deliveries.insert(id, purchase);
        }
        if let Some((id, effect)) = receipt_correction {
            self.receipt_corrections.insert(id, effect);
        }
        self.lifecycle.extend(copy_life_updates);
        if let Some((purchase_id, operation_id)) = receipt {
            self.receipts.insert(purchase_id, operation_id);
        }
        if let Some((id, effect)) = return_effect {
            self.return_effects.insert(id, effect);
        }
        if let Some(id) = corrected_return {
            self.corrected_returns.insert(id);
        }
        self.operations.push(op);
        self.outcomes.push(result.clone());
        Ok(result)
    }

    fn add_expected(
        &self,
        state: &mut verification::State,
        lines: &[verification::ExpectedLine],
    ) -> Result<(), LedgerError> {
        if state.lines.len().saturating_add(lines.len()) > 10_000 {
            return Err(LedgerError::Conflict);
        }
        for spec in lines {
            self.require_new(spec.holding_id)?;
            if state.lines.contains_key(&spec.line_id)
                || state
                    .lines
                    .values()
                    .any(|line| line.holding_id == spec.holding_id)
                || self
                    .verification
                    .values()
                    .any(|s| s.lines.contains_key(&spec.line_id))
            {
                return Err(LedgerError::Conflict);
            }
            state.lines.insert(
                spec.line_id,
                verification::Line {
                    retired: false,
                    id: spec.line_id,
                    holding_id: spec.holding_id,
                    definition_id: spec.definition_id,
                    color_id: spec.color_id,
                    expected: spec.expected,
                    verified: None,
                    finished: false,
                },
            );
        }
        state.completed = false;
        Ok(())
    }

    fn require_new(&self, id: Id) -> Result<(), LedgerError> {
        if self.holdings.contains_key(&id)
            || self.set_copies.contains_key(&id)
            || self
                .verification
                .values()
                .any(|state| state.lines.values().any(|line| line.holding_id == id))
        {
            Err(LedgerError::Conflict)
        } else {
            Ok(())
        }
    }

    pub fn replay(operations: impl IntoIterator<Item = Operation>) -> Result<Self, LedgerError> {
        let mut ledger = Self::default();
        for op in operations {
            let count = ledger.operations.len();
            ledger.append(op)?;
            if ledger.operations.len() != count + 1 {
                return Err(LedgerError::Conflict);
            }
        }
        Ok(ledger)
    }
}

fn new_holding(pieces: Measure, weight_mg: Measure) -> Holding {
    Holding {
        pieces: Balance::new(pieces, Measure::Known(0)).expect("zero damage is valid"),
        weight_mg: Balance::new(weight_mg, Measure::Known(0)).expect("zero damage is valid"),
        assembly: AssemblyState::Loose,
        revision: 1,
    }
}

fn extraction(previous: Measure, extracted: Option<u64>) -> Result<Action, LedgerError> {
    match (previous, extracted) {
        (Measure::Known(old), Some(value)) => old
            .checked_sub(value)
            .map(Action::SetMeasured)
            .ok_or(LedgerError::Journal(JournalError::OutOfRange)),
        (Measure::Known(0), None) => Ok(Action::Unchanged),
        (_, _) => Ok(Action::Invalidate),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerError {
    Journal(JournalError),
    Conflict,
    MissingHolding,
    UnsupportedTransfer,
    EmptyTransfer,
    ReconciliationRequired,
    UnfinishedVerification,
    ClosedClassification,
    InvalidStockAction(&'static str),
}
impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Journal(e) => e.fmt(f),
            Self::ClosedClassification => {
                f.write_str("classification is complete; explicitly reopen before changing residue")
            }
            Self::InvalidStockAction(message) => f.write_str(message),
            Self::Conflict => {
                f.write_str("operation key or identity conflicts with existing history")
            }
            Self::MissingHolding => f.write_str("holding does not exist"),
            Self::UnsupportedTransfer => {
                f.write_str("identification currently requires loose, undamaged stock")
            }
            Self::ReconciliationRequired => {
                f.write_str("stock changed after verification; count reconciliation is required")
            }
            Self::UnfinishedVerification => {
                f.write_str("explicit finished counts are required before completion")
            }
            Self::EmptyTransfer => {
                f.write_str("identification requires a positive measured count or weight")
            }
        }
    }
}
impl Error for LedgerError {}
