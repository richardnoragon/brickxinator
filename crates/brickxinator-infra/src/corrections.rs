//! Received-purchase corrections preview changes to residual stock, not a second receipt.
use crate::sqlite::{Result, Store, load};
use brickxinator_core::{
    inventory::{Balance, Measure},
    journal::{Action, MeasureChanges},
    ledger::{Body, Id, Ledger, Operation, ReceivedCopy, ReceivedStock},
    purchases::{
        CopyAddition, CorrectionEffect, CorrectionPreview, Draft, LineKind, Purchase,
        PurchaseCommand, Status,
    },
    stock,
};

fn consumed(
    ledger: &Ledger,
    id: Id,
    weight: bool,
    reconciled: &std::collections::BTreeSet<String>,
) -> Option<u128> {
    let mut total = 0u128;
    for operation in ledger.operations() {
        if reconciled.contains(&operation.id.to_string()) {
            continue;
        }
        let amount = match &operation.body {
            Body::Identify {
                source_id,
                pieces,
                weight_mg,
                ..
            } if *source_id == id => Some(if weight { *weight_mg } else { *pieces }),
            Body::Stock {
                holding_id,
                action: stock::Action::Adjust(adjustment),
                ..
            } if *holding_id == id => match adjustment {
                stock::Adjustment::Remove {
                    pieces, weight_mg, ..
                } => Some(if weight { *weight_mg } else { *pieces }),
                stock::Adjustment::Correction { pieces, weight_mg } => {
                    if (if weight { weight_mg } else { pieces }).owned != Action::Unchanged {
                        return None;
                    }
                    None
                }
                _ => None,
            },
            _ => None,
        };
        if let Some(amount) = amount {
            total = total.checked_add(u128::from(amount?))?;
        }
    }
    Some(total)
}
fn independently_observed(
    ledger: &Ledger,
    id: Id,
    weight: bool,
    reconciled: &std::collections::BTreeSet<String>,
) -> bool {
    ledger
        .operations()
        .iter()
        .filter(|op| !reconciled.contains(&op.id.to_string()))
        .any(|op| {
            let changes = match &op.body {
                Body::Stock {
                    holding_id,
                    action:
                        stock::Action::Adjust(stock::Adjustment::Correction { pieces, weight_mg }),
                    ..
                } if *holding_id == id => Some(if weight { weight_mg } else { pieces }),
                Body::Change {
                    holding_id,
                    pieces,
                    weight_mg,
                    ..
                } if *holding_id == id => Some(if weight { weight_mg } else { pieces }),
                _ => None,
            };
            changes.is_some_and(|changes| {
                matches!(changes.owned, Action::SetMeasured(_) | Action::Invalidate)
            })
        })
}
fn corrected(
    old: Measure,
    new: Measure,
    current: Measure,
    ledger: &Ledger,
    id: Id,
    weight: bool,
    reconciled: &std::collections::BTreeSet<String>,
) -> Result<Measure> {
    if old == new {
        return Ok(current);
    }
    let Measure::Known(new) = new else {
        return Ok(Measure::Unknown);
    };
    if let (Measure::Known(old), Measure::Known(current)) = (old, current) {
        let next = (u128::from(current) + u128::from(new))
            .checked_sub(u128::from(old))
            .ok_or("corrected acquisition is below already transferred or removed stock")?;
        return Ok(Measure::Known(u64::try_from(next)?));
    }
    if let Some(consumed) = consumed(ledger, id, weight, reconciled) {
        let next = u128::from(new)
            .checked_sub(consumed)
            .ok_or("corrected acquisition is below already transferred or removed stock")?;
        return Ok(Measure::Known(u64::try_from(next)?));
    }
    // There is no justified delta from an unknown baseline after an unmeasured
    // transfer or independent absolute correction. Require explicit reconciliation.
    Err("cannot reconcile this acquisition measure after unmeasured transfers or independent stock corrections; keep the acquisition measure unknown and record measured residual stock separately".into())
}
pub(crate) fn plan(
    connection: &rusqlite::Connection,
    before: &Purchase,
    draft: &Draft,
    ledger: &Ledger,
    candidates: Option<&CorrectionPreview>,
) -> Result<CorrectionPreview> {
    if before.status != Status::Received {
        return Err("correction preview requires a received purchase".into());
    }
    draft.validate()?;
    let supported: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migration WHERE version=11)",
        [],
        |row| row.get(0),
    )?;
    if !supported {
        return Err("received correction requires schema 11".into());
    }
    let reconciled = connection
        .prepare("SELECT operation_id FROM purchase_reconciliation")?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<std::collections::BTreeSet<_>>>()?;
    if before.draft.lines.len() != draft.lines.len() {
        return Err("received corrections preserve invoice-line identities; adding/removing lines requires a separate purchase".into());
    }
    let mut preview = CorrectionPreview {
        purchase_revision: before.revision,
        inventory_sequence: u64::try_from(ledger.operations().len())?,
        effects: vec![],
        copy_additions: vec![],
        warnings: vec![],
    };
    let receipt = ledger
        .receipts()
        .get(&before.id)
        .ok_or("purchase has no receipt")?;
    let Body::Receive { lines, .. } = &ledger
        .operations()
        .iter()
        .find(|op| op.id == *receipt)
        .ok_or("receipt operation missing")?
        .body
    else {
        return Err("invalid receipt".into());
    };
    for (old, new) in before.draft.lines.iter().zip(&draft.lines) {
        if old.id != new.id || old.kind != new.kind || old.assembly != new.assembly {
            return Err("received correction preserves line identity, kind and receipt assembly; use individual copy controls for assembly changes".into());
        }
        if old.kind == LineKind::Set {
            let existing = ledger
                .set_copies()
                .values()
                .filter(|copy| copy.invoice_line_id == old.id)
                .count();
            let desired = new.copy_count.map_or(existing, |count| count as usize);
            if desired < existing {
                return Err("copy count cannot discard existing physical copy identities; copy removal and whole-set disposal are deferred. Archiving preserves ownership".into());
            }
            if desired > 10_000 {
                return Err("copy reconciliation exceeds 10000 copies".into());
            }
            let mut copies = vec![];
            for index in existing..desired {
                let id = if let Some(candidate) = candidates {
                    candidate
                        .copy_additions
                        .iter()
                        .find(|a| a.invoice_line_id == old.id)
                        .and_then(|addition| addition.copies.get(index - existing))
                        .ok_or("copy preview no longer matches requested count")?
                        .id
                } else {
                    Id::new()
                };
                copies.push(ReceivedCopy {
                    id,
                    label: format!(
                        "{}_{:02}",
                        new.set_number
                            .as_deref()
                            .filter(|s| !s.trim().is_empty())
                            .unwrap_or("Set"),
                        index + 1
                    ),
                    assembly: new.assembly,
                });
            }
            if !copies.is_empty() {
                preview.copy_additions.push(CopyAddition {
                    invoice_line_id: old.id,
                    copies,
                });
            }
            if old.pieces != new.pieces || old.weight_mg != new.weight_mg {
                preview.warnings.push("Set acquisition/box measurements remain descriptive; verified part stock is unchanged.".into());
            }
            continue;
        }
        let source = lines
            .iter()
            .find(|line| line.invoice_line_id == old.id)
            .ok_or("source receipt line missing")?;
        let ReceivedStock::Batch { holding_id, .. } = source.stock else {
            return Err("receipt source is not a batch".into());
        };
        let current = *ledger
            .holdings()
            .get(&holding_id)
            .ok_or("residual holding missing")?;
        let pieces = if independently_observed(ledger, holding_id, false, &reconciled) {
            if old.pieces != new.pieces {
                preview.warnings.push("Current residual pieces have an independent stock observation; this acquisition edit will not change that current balance.".into());
            }
            current.pieces.owned()
        } else {
            corrected(
                old.pieces,
                new.pieces,
                current.pieces.owned(),
                ledger,
                holding_id,
                false,
                &reconciled,
            )?
        };
        let weight = if independently_observed(ledger, holding_id, true, &reconciled) {
            if old.weight_mg != new.weight_mg {
                preview.warnings.push("Current residual weight has an independent stock observation; this acquisition edit will not change that current balance.".into());
            }
            current.weight_mg.owned()
        } else {
            corrected(
                old.weight_mg,
                new.weight_mg,
                current.weight_mg.owned(),
                ledger,
                holding_id,
                true,
                &reconciled,
            )?
        };
        if pieces != current.pieces.owned() || weight != current.weight_mg.owned() {
            if ledger
                .classification()
                .get(&holding_id)
                .is_some_and(|s| s.complete)
            {
                return Err(
                    "reopen the completed mixed lot before changing its acquisition measurements"
                        .into(),
                );
            }
            let after = brickxinator_core::journal::Holding {
                pieces: Balance::new(pieces, current.pieces.damaged())?,
                weight_mg: Balance::new(weight, current.weight_mg.damaged())?,
                revision: current
                    .revision
                    .checked_add(1)
                    .ok_or("holding revision overflow")?,
                ..current
            };
            preview.effects.push(CorrectionEffect {
                holding_id,
                before: current,
                after,
            });
        }
    }
    Ok(preview)
}
fn action(before: Measure, after: Measure) -> Action {
    if before == after {
        Action::Unchanged
    } else {
        match after {
            Measure::Unknown => Action::Invalidate,
            Measure::Known(value) => Action::SetMeasured(value),
        }
    }
}
pub(crate) fn bodies(
    command: &PurchaseCommand,
    preview: &CorrectionPreview,
) -> Vec<(String, Body)> {
    let mut result = preview
        .effects
        .iter()
        .map(|effect| {
            let body = Body::Stock {
                holding_id: effect.holding_id,
                expected_revision: effect.before.revision,
                action: stock::Action::Adjust(stock::Adjustment::Correction {
                    pieces: MeasureChanges {
                        owned: action(effect.before.pieces.owned(), effect.after.pieces.owned()),
                        damaged: Action::Unchanged,
                    },
                    weight_mg: MeasureChanges {
                        owned: action(
                            effect.before.weight_mg.owned(),
                            effect.after.weight_mg.owned(),
                        ),
                        damaged: Action::Unchanged,
                    },
                }),
            };
            (
                format!("purchase-correction:{}:{}", command.id, effect.holding_id),
                body,
            )
        })
        .collect::<Vec<_>>();
    for addition in &preview.copy_additions {
        result.push((
            format!(
                "purchase-correction:{}:{}",
                command.id, addition.invoice_line_id
            ),
            Body::AddCopies {
                purchase_id: command.body.purchase_id(),
                invoice_line_id: addition.invoice_line_id,
                copies: addition.copies.clone(),
            },
        ));
    }
    result
}
pub(crate) fn apply(
    connection: &rusqlite::Connection,
    before: &Purchase,
    draft: &Draft,
    preview: &CorrectionPreview,
    command: &PurchaseCommand,
) -> Result<Vec<Id>> {
    let supported: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migration WHERE version=11)",
        [],
        |r| r.get(0),
    )?;
    if !supported {
        return Err("received correction requires schema 11".into());
    }
    let mut ledger = load(connection)?;
    if u64::try_from(ledger.operations().len())? != preview.inventory_sequence {
        return Err("inventory changed since correction preview; preview again".into());
    }
    if plan(connection, before, draft, &ledger, Some(preview))? != *preview {
        return Err("correction preview changed since it was prepared; preview again".into());
    }
    if command.reason.trim().is_empty() {
        return Err("a correction reason is required".into());
    }
    let mut operations = vec![];
    for (key, body) in bodies(command, preview) {
        if crate::sqlite::metadata_key_exists(connection, &key)? {
            return Err("correction operation key conflicts with metadata".into());
        }
        let operation = Operation {
            id: Id::new(),
            request_key: key,
            actor: command.actor.clone(),
            reason: command.reason.clone(),
            body,
        };
        let outcome = ledger.append(operation.clone())?;
        crate::sqlite::persist_operation(connection, &ledger, &operation, &outcome)?;
        operations.push(operation.id);
    }
    Ok(operations)
}
impl Store {
    pub fn preview_purchase_correction(
        &self,
        id: Id,
        expected_revision: u64,
        draft: &Draft,
    ) -> Result<CorrectionPreview> {
        let tx = self.connection.unchecked_transaction()?;
        let before =
            crate::purchase_store::read_purchase(&tx, id)?.ok_or("purchase does not exist")?;
        before.check_revision(expected_revision)?;
        let preview = plan(&tx, &before, draft, &load(&tx)?, None)?;
        tx.commit()?;
        Ok(preview)
    }
}

pub(crate) fn validate(connection: &rusqlite::Connection, ledger: &Ledger) -> Result<()> {
    use brickxinator_core::purchases::PurchaseAction;
    use std::collections::{BTreeMap, BTreeSet};
    let mut purchases = BTreeMap::<Id, Purchase>::new();
    let mut expected_links = BTreeSet::new();
    let mut query=connection.prepare("SELECT id,request_key,actor,reason,purchase_id,payload_json,before_json,after_json,result_json FROM purchase_command ORDER BY sequence")?;
    let mut rows = query.query([])?;
    while let Some(row) = rows.next()? {
        let command: PurchaseCommand = serde_json::from_str(&row.get::<_, String>(5)?)?;
        let before: Option<Purchase> = row
            .get::<_, Option<String>>(6)?
            .as_deref()
            .map(serde_json::from_str)
            .transpose()?;
        let after: Purchase = serde_json::from_str(&row.get::<_, String>(7)?)?;
        let result: crate::purchase_store::PurchaseResult =
            serde_json::from_str(&row.get::<_, String>(8)?)?;
        let id = command.body.purchase_id();
        if command.id.to_string() != row.get::<_, String>(0)?
            || command.request_key != row.get::<_, String>(1)?
            || command.actor != row.get::<_, String>(2)?
            || command.reason != row.get::<_, String>(3)?
            || id.to_string() != row.get::<_, String>(4)?
            || before.as_ref() != purchases.get(&id)
            || result.purchase != after
        {
            return Err("purchase command audit chain differs from its recorded state".into());
        }
        let expected = match &command.body {
            PurchaseAction::Create { draft, .. } => {
                if before.is_some() {
                    return Err("purchase was created twice".into());
                }
                draft.validate()?;
                Purchase {
                    id,
                    archived: false,
                    revision: 1,
                    status: Status::Draft,
                    draft: draft.clone(),
                }
            }
            PurchaseAction::Save {
                expected_revision,
                status,
                draft,
                ..
            } => before
                .as_ref()
                .ok_or("purchase edit has no prior state")?
                .save(*expected_revision, *status, draft.clone())?,
            action => {
                let mut previous = before.clone().ok_or("purchase action has no prior state")?;
                let revision = match action {
                    PurchaseAction::Archive {
                        expected_revision, ..
                    }
                    | PurchaseAction::Receive {
                        expected_revision, ..
                    }
                    | PurchaseAction::Correct {
                        expected_revision, ..
                    } => *expected_revision,
                    _ => unreachable!(),
                };
                previous.check_revision(revision)?;
                match action {
                    PurchaseAction::Archive { archived, .. } => previous.archived = *archived,
                    PurchaseAction::Receive { .. } => {
                        if matches!(previous.status, Status::Received | Status::Cancelled)
                            || previous.draft.lines.is_empty()
                        {
                            return Err("invalid audited receipt transition".into());
                        }
                        let position = ledger
                            .operations()
                            .iter()
                            .position(|op| op.id == command.id)
                            .ok_or("audited receipt operation missing")?;
                        let mut prefix =
                            Ledger::replay(ledger.operations()[..position].iter().cloned())?;
                        let outcome = prefix.append(ledger.operations()[position].clone())?;
                        if result.receipt.as_ref() != Some(&outcome) {
                            return Err(
                                "cached receipt result differs from immutable movement".into()
                            );
                        }
                        previous.status = Status::Received;
                    }
                    PurchaseAction::Correct { draft, preview, .. } => {
                        if command.reason.trim().is_empty() {
                            return Err("audited correction has no reason".into());
                        }
                        let start = usize::try_from(preview.inventory_sequence)?;
                        let prefix_ops = ledger
                            .operations()
                            .get(..start)
                            .ok_or("correction preview sequence is outside the journal")?;
                        let mut prefix = Ledger::replay(prefix_ops.iter().cloned())?;
                        if plan(connection, &previous, draft, &prefix, Some(preview))? != *preview {
                            return Err(
                                "audited correction preview differs from its prior stock".into()
                            );
                        }
                        let planned = bodies(&command, preview);
                        let actual = ledger
                            .operations()
                            .get(
                                start
                                    ..start
                                        .checked_add(planned.len())
                                        .ok_or("correction sequence overflow")?,
                            )
                            .ok_or("correction movements missing")?;
                        for ((key, body), operation) in planned.iter().zip(actual) {
                            if *key != operation.request_key
                                || *body != operation.body
                                || command.actor != operation.actor
                                || command.reason != operation.reason
                            {
                                return Err(
                                    "correction movement differs from accepted preview".into()
                                );
                            }
                            prefix.append(operation.clone())?;
                            expected_links
                                .insert((command.id.to_string(), operation.id.to_string()));
                        }
                        for effect in &preview.effects {
                            if prefix.holdings().get(&effect.holding_id) != Some(&effect.after) {
                                return Err(
                                    "correction result differs from previewed residual".into()
                                );
                            }
                        }
                        previous.draft = draft.clone();
                    }
                    _ => unreachable!(),
                }
                previous.revision = previous
                    .revision
                    .checked_add(1)
                    .ok_or("purchase revision overflow")?;
                previous
            }
        };
        expected.draft.validate()?;
        if expected != after
            || !matches!(command.body, PurchaseAction::Receive { .. }) && result.receipt.is_some()
        {
            return Err("audited purchase result differs from command".into());
        }
        purchases.insert(id, expected);
    }
    let supported: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migration WHERE version=11)",
        [],
        |r| r.get(0),
    )?;
    let actual_links = if supported {
        connection
            .prepare("SELECT purchase_command_id,operation_id FROM purchase_reconciliation")?
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .collect::<rusqlite::Result<BTreeSet<_>>>()?
    } else {
        BTreeSet::new()
    };
    if actual_links != expected_links {
        return Err("purchase reconciliation references differ from audited movements".into());
    }
    for operation in ledger
        .operations()
        .iter()
        .filter(|op| matches!(op.body, Body::AddCopies { .. }))
    {
        if !expected_links
            .iter()
            .any(|(_, id)| *id == operation.id.to_string())
        {
            return Err("physical copy addition has no purchase reconciliation".into());
        }
    }
    for (id, expected) in purchases {
        if crate::purchase_store::read_purchase(connection, id)?.as_ref() != Some(&expected) {
            return Err("purchase projection differs from audit replay".into());
        }
    }
    Ok(())
}
