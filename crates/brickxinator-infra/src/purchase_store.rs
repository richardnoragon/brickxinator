//! Purchase commands commit metadata, audit, receipt and inventory in one transaction.
use crate::sqlite::{Result, Store, has_purchases, load, persist_operation};
use brickxinator_core::{
    ledger::{Body, Id, Ledger, Outcome, ReceivedStock, SetCopy},
    purchases::{InvoiceLine, Purchase, PurchaseAction, PurchaseCommand, PurchaseError, Status},
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurchaseResult {
    pub purchase: Purchase,
    pub receipt: Option<Outcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchaseRevision {
    pub committed_at: String,
    pub actor: String,
    pub reason: String,
    pub before: Option<Purchase>,
    pub after: Purchase,
}

impl Store {
    pub fn purchases(&self) -> Result<Vec<Purchase>> {
        let tx = self.connection.unchecked_transaction()?;
        let ids: Vec<String> = tx
            .prepare("SELECT id FROM purchase ORDER BY id DESC")?
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        let mut purchases = Vec::new();
        for id in ids {
            let id = serde_json::from_value(serde_json::Value::String(id))?;
            purchases.push(read_purchase(&tx, id)?.ok_or("purchase not found")?);
        }
        tx.commit()?;
        Ok(purchases)
    }
    pub fn purchase_history(&self, id: Id) -> Result<Vec<PurchaseRevision>> {
        require_schema(&self.connection)?;
        let mut query = self.connection.prepare("SELECT committed_at,actor,reason,before_json,after_json FROM purchase_command WHERE purchase_id=?1 ORDER BY sequence")?;
        let rows = query.query_map([id.to_string()], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, String>(4)?,
            ))
        })?;
        rows.map(|row| {
            let (committed_at, actor, reason, before, after) = row?;
            Ok(PurchaseRevision {
                committed_at,
                actor,
                reason,
                before: before.as_deref().map(serde_json::from_str).transpose()?,
                after: serde_json::from_str(&after)?,
            })
        })
        .collect()
    }

    pub fn purchase(&self, id: Id) -> Result<Option<Purchase>> {
        require_schema(&self.connection)?;
        let tx = self.connection.unchecked_transaction()?;
        let purchase = read_purchase(&tx, id)?;
        tx.commit()?;
        Ok(purchase)
    }

    pub fn execute_purchase(&mut self, command: PurchaseCommand) -> Result<PurchaseResult> {
        require_schema(&self.connection)?;
        if command.actor.trim().is_empty() || command.request_key.trim().is_empty() {
            return Err("purchase actor and request key are required".into());
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let prior: Option<(String, String)> = tx
            .query_row(
                "SELECT payload_json,result_json FROM purchase_command WHERE request_key=?1",
                [&command.request_key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((payload, result)) = prior {
            if serde_json::from_str::<PurchaseCommand>(&payload)? != command {
                return Err("purchase request key conflicts with an earlier payload".into());
            }
            let result = serde_json::from_str(&result)?;
            tx.commit()?;
            return Ok(result);
        }
        if crate::sqlite::metadata_key_exists(&tx, &command.request_key)? {
            return Err("request key belongs to a metadata command".into());
        }
        let inventory_key: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM inventory_operation WHERE request_key=?1)",
            [&command.request_key],
            |r| r.get(0),
        )?;
        if inventory_key {
            return Err("request key already belongs to an inventory operation".into());
        }
        let id = command.body.purchase_id();
        let before = read_purchase(&tx, id)?;
        let mut receipt = None;
        let mut reconciliation_operations = vec![];
        let next = match &command.body {
            PurchaseAction::Correct {
                expected_revision,
                draft,
                preview,
                ..
            } => {
                let current = before.as_ref().ok_or(PurchaseError::MissingPurchase)?;
                current.check_revision(*expected_revision)?;
                reconciliation_operations =
                    crate::corrections::apply(&tx, current, draft, preview, &command)?;
                Purchase {
                    id,
                    archived: current.archived,
                    revision: current
                        .revision
                        .checked_add(1)
                        .ok_or(PurchaseError::OutOfRange)?,
                    status: Status::Received,
                    draft: draft.clone(),
                }
            }
            PurchaseAction::Archive {
                expected_revision,
                archived,
                ..
            } => {
                let mut current = before.clone().ok_or(PurchaseError::MissingPurchase)?;
                current.check_revision(*expected_revision)?;
                current.revision = current
                    .revision
                    .checked_add(1)
                    .ok_or(PurchaseError::OutOfRange)?;
                current.archived = *archived;
                current
            }
            PurchaseAction::Create { draft, .. } => {
                if before.is_some() {
                    return Err("purchase identity already exists".into());
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
                .ok_or(PurchaseError::MissingPurchase)?
                .save(*expected_revision, *status, draft.clone())?,
            PurchaseAction::Receive { .. } => {
                let current = before.as_ref().ok_or(PurchaseError::MissingPurchase)?;
                let operation = current.receipt_operation(&command)?;
                let mut ledger = load(&tx)?;
                let outcome = ledger.append(operation.clone())?;
                persist_operation(&tx, &ledger, &operation, &outcome)?;
                tx.execute(
                    "INSERT INTO receipt(purchase_id,operation_id) VALUES(?1,?2)",
                    params![id.to_string(), operation.id.to_string()],
                )?;
                receipt = Some(outcome);
                Purchase {
                    id,
                    archived: current.archived,
                    revision: current
                        .revision
                        .checked_add(1)
                        .ok_or(PurchaseError::OutOfRange)?,
                    status: Status::Received,
                    draft: current.draft.clone(),
                }
            }
        };
        let snapshot = serde_json::to_string(&next)?;
        tx.execute("INSERT INTO purchase(id,revision,status,snapshot_json) VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,status=excluded.status,snapshot_json=excluded.snapshot_json",
            params![id.to_string(), i64::try_from(next.revision)?, status_name(next.status), snapshot])?;
        if !matches!(
            command.body,
            PurchaseAction::Receive { .. } | PurchaseAction::Archive { .. }
        ) {
            let existing: Vec<String> = tx
                .prepare("SELECT id FROM invoice_line WHERE purchase_id=?1")?
                .query_map([id.to_string()], |r| r.get(0))?
                .collect::<rusqlite::Result<_>>()?;
            for old_id in existing {
                if !next
                    .draft
                    .lines
                    .iter()
                    .any(|line| line.id.to_string() == old_id)
                {
                    tx.execute("DELETE FROM invoice_line WHERE id=?1", [old_id])?;
                }
            }
            let maximum: i64 = tx.query_row(
                "SELECT coalesce(max(position),0) FROM invoice_line WHERE purchase_id=?1",
                [id.to_string()],
                |r| r.get(0),
            )?;
            let offset = maximum
                .checked_add(i64::try_from(next.draft.lines.len())?)
                .and_then(|n| n.checked_add(1))
                .ok_or("invoice position overflow")?;
            tx.execute(
                "UPDATE invoice_line SET position=position+?1 WHERE purchase_id=?2",
                params![offset, id.to_string()],
            )?;
            for (position, line) in next.draft.lines.iter().enumerate() {
                let written=tx.execute("INSERT INTO invoice_line(id,purchase_id,position,snapshot_json) VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET position=excluded.position,snapshot_json=excluded.snapshot_json WHERE invoice_line.purchase_id=excluded.purchase_id",
                    params![line.id.to_string(), id.to_string(), i64::try_from(position)?, serde_json::to_string(line)?])?;
                if written != 1 {
                    return Err("invoice identity belongs to another purchase".into());
                }
            }
        }
        let result = PurchaseResult {
            purchase: next,
            receipt,
        };
        tx.execute("INSERT INTO purchase_command(id,request_key,purchase_id,actor,reason,payload_json,before_json,after_json,result_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![command.id.to_string(), command.request_key, id.to_string(), command.actor, command.reason,
                serde_json::to_string(&command)?, before.as_ref().map(serde_json::to_string).transpose()?, snapshot, serde_json::to_string(&result)?])?;
        for operation in reconciliation_operations {
            tx.execute("INSERT INTO purchase_reconciliation(purchase_command_id,operation_id) VALUES(?1,?2)",params![command.id.to_string(),operation.to_string()])?;
        }
        if matches!(command.body, PurchaseAction::Correct { .. }) {
            load(&tx)?;
        }
        tx.commit()?;
        Ok(result)
    }
}

fn status_name(status: Status) -> &'static str {
    match status {
        Status::Draft => "Draft",
        Status::Ordered => "Ordered",
        Status::Shipped => "Shipped",
        Status::Received => "Received",
        Status::Cancelled => "Cancelled",
    }
}

fn require_schema(connection: &Connection) -> Result<()> {
    if !has_purchases(connection)? {
        return Err("purchase commands need schema 2; automatic upgrades are not implemented, use a new development database".into());
    }
    Ok(())
}

pub(crate) fn read_purchase(connection: &Connection, id: Id) -> Result<Option<Purchase>> {
    let row: Option<(i64, String, String)> = connection
        .query_row(
            "SELECT revision,status,snapshot_json FROM purchase WHERE id=?1",
            [id.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let Some((revision, status, json)) = row else {
        return Ok(None);
    };
    let purchase: Purchase = serde_json::from_str(&json)?;
    purchase.draft.validate()?;
    if purchase.id != id
        || i64::try_from(purchase.revision)? != revision
        || status_name(purchase.status) != status
    {
        return Err("purchase snapshot does not match its row".into());
    }
    let lines: Vec<(String, i64, String)> = connection.prepare("SELECT id,position,snapshot_json FROM invoice_line WHERE purchase_id=?1 ORDER BY position")?
        .query_map([id.to_string()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?.collect::<rusqlite::Result<_>>()?;
    if lines.len() != purchase.draft.lines.len() {
        return Err("purchase lines do not match snapshot".into());
    }
    for (position, ((line_id, stored_position, line_json), line)) in
        lines.iter().zip(&purchase.draft.lines).enumerate()
    {
        if line_id != &line.id.to_string()
            || *stored_position != i64::try_from(position)?
            || serde_json::from_str::<InvoiceLine>(line_json)? != *line
        {
            return Err("invoice line identity/order does not match purchase".into());
        }
    }
    let after: String = connection.query_row("SELECT after_json FROM purchase_command WHERE purchase_id=?1 ORDER BY sequence DESC LIMIT 1", [id.to_string()], |r| r.get(0))?;
    if serde_json::from_str::<Purchase>(&after)? != purchase {
        return Err("purchase differs from last audited revision".into());
    }
    let received: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM receipt WHERE purchase_id=?1)",
        [id.to_string()],
        |r| r.get(0),
    )?;
    if received != (purchase.status == Status::Received) {
        return Err("purchase status does not match receipt history".into());
    }
    Ok(Some(purchase))
}

pub(crate) fn validate_ownership(connection: &Connection, ledger: &Ledger) -> Result<()> {
    let receipts: BTreeMap<String, String> = connection
        .prepare("SELECT purchase_id,operation_id FROM receipt")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let expected: BTreeMap<_, _> = ledger
        .receipts()
        .iter()
        .map(|(id, op)| (id.to_string(), op.to_string()))
        .collect();
    if receipts != expected {
        return Err("receipts do not match inventory history".into());
    }
    let copies: Vec<(String, String, String, String)> = connection
        .prepare(
            "SELECT id,invoice_line_id,receipt_operation_id,snapshot_json FROM owned_set_copy",
        )?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<rusqlite::Result<_>>()?;
    if copies.len() != ledger.set_copies().len() {
        return Err("set copy identities differ from replay".into());
    }
    for (id, line, operation_id, json) in copies {
        let copy: SetCopy = serde_json::from_str(&json)?;
        if copy.id.to_string() != id || copy.invoice_line_id.to_string() != line || ledger.set_copies().get(&copy.id) != Some(&copy)
            || !ledger.operations().iter().any(|op| op.id.to_string() == operation_id && (matches!(&op.body, Body::Receive { lines, .. } if lines.iter().any(|l| l.invoice_line_id == copy.invoice_line_id)) || matches!(&op.body,Body::AddCopies{invoice_line_id,..} if *invoice_line_id==copy.invoice_line_id))) {
            return Err("set-copy metadata differs from receipt history".into());
        }
    }
    let provenance: Vec<(String, String, Option<String>)> = connection
        .prepare("SELECT holding_id,invoice_line_id,parent_holding_id FROM stock_provenance")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    if provenance.len() != ledger.provenance().len() {
        return Err("stock provenance differs from history".into());
    }
    for (holding, line, parent) in provenance {
        if !ledger
            .provenance()
            .iter()
            .any(|(h, l)| h.to_string() == holding && l.to_string() == line)
        {
            return Err("stock source line differs from history".into());
        }
        let expected_parent = ledger.operations().iter().find_map(|op| match op.body {
            Body::Identify {
                source_id,
                child_id,
                ..
            } if child_id.to_string() == holding => Some(source_id.to_string()),
            _ => None,
        });
        if parent != expected_parent {
            return Err("classification parent differs from history".into());
        }
    }
    // Current metadata must match its audited snapshot and receipt state.
    let ids: Vec<String> = connection
        .prepare("SELECT id FROM purchase")?
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    for id in ids {
        let id: Id = serde_json::from_value(serde_json::Value::String(id))?;
        let purchase = read_purchase(connection, id)?.ok_or(PurchaseError::MissingPurchase)?;
        if purchase.status == Status::Received {
            let operation_id = ledger
                .receipts()
                .get(&id)
                .ok_or("missing receipt operation")?;
            let operation = ledger
                .operations()
                .iter()
                .find(|op| op.id == *operation_id)
                .ok_or("missing operation")?;
            let Body::Receive { lines, .. } = &operation.body else {
                return Err("invalid receipt operation kind".into());
            };
            let initial_json: String = connection.query_row(
                "SELECT after_json FROM purchase_command WHERE id=?1",
                [operation.id.to_string()],
                |r| r.get(0),
            )?;
            let initial: Purchase = serde_json::from_str(&initial_json)?;
            if lines.len() != initial.draft.lines.len()
                || initial.id != purchase.id
                || initial.status != Status::Received
            {
                return Err("receipt lines differ from invoice".into());
            }
            for (received, line) in lines.iter().zip(&initial.draft.lines) {
                if received.invoice_line_id != line.id {
                    return Err("receipt line identity differs from invoice".into());
                }
                let valid = match &received.stock {
                    ReceivedStock::Batch {
                        pieces, weight_mg, ..
                    } => {
                        line.kind != brickxinator_core::purchases::LineKind::Set
                            && *pieces == line.pieces
                            && *weight_mg == line.weight_mg
                    }
                    ReceivedStock::Sets { copies } => {
                        line.kind == brickxinator_core::purchases::LineKind::Set
                            && copies.len() == line.copy_count.unwrap_or(0) as usize
                            && copies.iter().all(|copy| copy.assembly == line.assembly)
                    }
                };
                if !valid {
                    return Err("receipt stock differs from purchase acquisition facts".into());
                }
            }
        }
    }
    crate::corrections::validate(connection, ledger)?;
    Ok(())
}
