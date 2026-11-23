//! Append-only order fulfilment, tied to purchase snapshots and stock prefixes.
use crate::sqlite::{self, Result, Store};
use brickxinator_core::{
    fulfilment::{Command, State},
    ledger::{Body, Id, Ledger},
    purchases::{Purchase, PurchaseAction},
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::collections::{BTreeMap, BTreeSet};
pub(crate) fn enabled(c: &Connection) -> Result<bool> {
    Ok(c.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migration WHERE version=15)",
        [],
        |r| r.get(0),
    )?)
}
pub(crate) fn key_exists(c: &Connection, key: &str) -> Result<bool> {
    Ok(enabled(c)?
        && c.query_row(
            "SELECT EXISTS(SELECT 1 FROM fulfilment_command WHERE request_key=?1)",
            [key],
            |r| r.get(0),
        )?)
}
fn profile_sequence(c: &Connection) -> Result<i64> {
    Ok(c.query_row(
        "SELECT coalesce(max(sequence),0) FROM profile_command",
        [],
        |r| r.get(0),
    )?)
}
pub(crate) fn canonical_source(c: &Connection, source: &Purchase) -> Result<()> {
    let mut q = c.prepare("SELECT after_json FROM purchase_command WHERE purchase_id=?1")?;
    for row in q.query_map([source.id.to_string()], |r| r.get::<_, String>(0))? {
        if serde_json::from_str::<Purchase>(&row?)? == *source {
            return Ok(());
        }
    }
    Err("fulfilment source snapshot is not in canonical purchase history".into())
}
pub(crate) fn states(c: &Connection, ledger: &Ledger) -> Result<BTreeMap<Id, State>> {
    if !enabled(c)? {
        if ledger.operations().iter().any(|o| {
            matches!(
                o.body,
                Body::SupplierReturn { .. }
                    | Body::SupplierReturnCorrection { .. }
                    | Body::Delivery { .. }
                    | Body::ReceiptCorrection { .. }
            )
        }) {
            return Err("partial receipts require schema 15".into());
        }
        return Ok(BTreeMap::new());
    }
    let mut states = BTreeMap::<Id, State>::new();
    let mut prefix = Ledger::default();
    let mut position = 0;
    let mut last_sequence = 0i64;
    let mut linked = BTreeSet::new();
    let mut last_profile = 0;
    let max_profile = profile_sequence(c)?;
    let mut q=c.prepare("SELECT sequence,id,request_key,purchase_id,inventory_sequence,profile_sequence,source_snapshot_json,payload_json,result_json FROM fulfilment_command ORDER BY sequence")?;
    let mut rows = q.query([])?;
    while let Some(row) = rows.next()? {
        let seq: i64 = row.get(0)?;
        let id: String = row.get(1)?;
        let key: String = row.get(2)?;
        let purchase: String = row.get(3)?;
        let inv = usize::try_from(row.get::<_, i64>(4)?)?;
        let profile: i64 = row.get(5)?;
        if seq <= last_sequence
            || inv < position
            || inv > ledger.operations().len()
            || profile < last_profile
            || profile > max_profile
        {
            return Err("invalid fulfilment source history prefix".into());
        }
        last_sequence = seq;
        last_profile = profile;
        while position < inv {
            prefix.append(ledger.operations()[position].clone())?;
            position += 1;
        }
        let source: Purchase = serde_json::from_str(&row.get::<_, String>(6)?)?;
        canonical_source(c, &source)?;
        let command: Command = serde_json::from_str(&row.get::<_, String>(7)?)?;
        if id != command.id.to_string()
            || key != command.request_key
            || purchase != source.id.to_string()
            || command.purchase_id != source.id
        {
            return Err("fulfilment envelope mismatch".into());
        }
        let before = states
            .get(&command.purchase_id)
            .cloned()
            .unwrap_or_default();
        let after = before.apply(&command, &source, &prefix)?;
        if let Some(operation) = State::operation(&command, &prefix) {
            if ledger.operations().get(position) != Some(&operation) {
                return Err("fulfilment effects do not match inventory journal".into());
            }
            prefix.append(operation.clone())?;
            position += 1;
            linked.insert(operation.id);
        }
        if after != serde_json::from_str::<State>(&row.get::<_, String>(8)?)? {
            return Err("fulfilment result differs from audit replay".into());
        }
        states.insert(command.purchase_id, after);
    }
    let expected: BTreeSet<Id> = ledger
        .operations()
        .iter()
        .filter(|o| {
            matches!(
                o.body,
                Body::SupplierReturn { .. }
                    | Body::SupplierReturnCorrection { .. }
                    | Body::Delivery { .. }
                    | Body::ReceiptCorrection { .. }
            )
        })
        .map(|o| o.id)
        .collect();
    if !expected.is_subset(&linked) {
        return Err("receipt/reversal lacks a canonical fulfilment command".into());
    }
    Ok(states)
}
pub(crate) fn validate(c: &Connection, ledger: &Ledger) -> Result<()> {
    states(c, ledger)?;
    Ok(())
}
pub(crate) fn guard_purchase(
    c: &Connection,
    ledger: &Ledger,
    purchase: &Purchase,
    action: &PurchaseAction,
) -> Result<()> {
    if !enabled(c)? {
        return Ok(());
    }
    let state = states(c, ledger)?.remove(&purchase.id).unwrap_or_default();
    if matches!(action, PurchaseAction::Receive { .. }) && !state.receipts.is_empty() {
        return Err("order has explicit receipt history; receive only outstanding quantities through Fulfilment".into());
    }
    state.financial.summary(purchase)?;
    for row in state.coverage(purchase, ledger)? {
        if let (
            brickxinator_core::inventory::Measure::Known(o),
            brickxinator_core::inventory::Measure::Known(r),
        ) = (row.ordered, row.received)
            && r > o
        {
            return Err("order quantity cannot be reduced below non-reversed receipts".into());
        }
    }
    Ok(())
}
impl Store {
    pub fn fulfilment(&self, purchase_id: Id) -> Result<serde_json::Value> {
        let tx = self.connection.unchecked_transaction()?;
        let ledger = sqlite::load(&tx)?;
        let state = states(&tx, &ledger)?
            .remove(&purchase_id)
            .unwrap_or_default();
        let purchase = crate::purchase_store::read_purchase(&tx, purchase_id)?
            .ok_or("purchase does not exist")?;
        let mut receipts = Vec::new();
        for op in ledger.operations() {
            if matches!(&op.body,Body::Receive{purchase_id:id,..}|Body::Delivery{purchase_id:id,..} if *id==purchase_id)
            {
                receipts.push(
                    serde_json::json!({"operation":op,"reversed":ledger.receipt_reversed(op.id)}),
                );
            }
        }
        let mut history = Vec::new();
        if enabled(&tx)? {
            let mut q=tx.prepare("SELECT committed_at,payload_json FROM fulfilment_command WHERE purchase_id=?1 ORDER BY sequence DESC LIMIT 100")?;
            for row in q.query_map([purchase_id.to_string()], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })? {
                let (at, json) = row?;
                history.push(
                    serde_json::json!({"at":at,"command":serde_json::from_str::<Command>(&json)?}),
                );
            }
        }
        Ok(
            serde_json::json!({"financial_summary":state.financial.summary(&purchase)?,"state":state,"purchase":purchase,"coverage":state.coverage(&purchase,&ledger)?,"status":state.status(&purchase,&ledger)?,"receipts":receipts,"history":history,"inventory_sequence":ledger.operations().len(),"profile_sequence":profile_sequence(&tx)?}),
        )
    }
    pub fn preview_fulfilment(&self, command: Command) -> Result<serde_json::Value> {
        let tx = self.connection.unchecked_transaction()?;
        if !enabled(&tx)? {
            return Err("fulfilment needs schema 15".into());
        }
        let mut ledger = sqlite::load(&tx)?;
        let before = states(&tx, &ledger)?
            .remove(&command.purchase_id)
            .unwrap_or_default();
        let source = crate::purchase_store::read_purchase(&tx, command.purchase_id)?
            .ok_or("purchase does not exist")?;
        let after = before.apply(&command, &source, &ledger)?;
        let inventory_sequence = ledger.operations().len();
        let old = ledger.holdings().clone();
        let effects = if let Some(op) = State::operation(&command, &ledger) {
            crate::physical_store::reversal_metadata(&tx, &ledger, &op)?;
            let result = ledger.append(op)?;
            crate::project_store::validate_current(&tx, &ledger)?;
            serde_json::json!({"holdings":result.holdings.iter().map(|(id,h)|serde_json::json!({"id":id,"before":old.get(id),"after":h})).collect::<Vec<_>>(),"copies":result.set_copies})
        } else {
            serde_json::json!({"holdings":[],"copies":{}})
        };
        Ok(
            serde_json::json!({"financial_after":after.financial.summary(&source)?,"command":command,"expected_inventory_sequence":inventory_sequence,"expected_profile_sequence":profile_sequence(&tx)?,"effects":effects,"status_after":after.status(&source,&ledger)?}),
        )
    }
    pub fn execute_fulfilment(
        &mut self,
        command: Command,
        expected_inventory_sequence: usize,
        expected_profile_sequence: i64,
    ) -> Result<State> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if !enabled(&tx)? {
            return Err("fulfilment needs schema 15".into());
        }
        let cached: Option<(String, String)> = tx
            .query_row(
                "SELECT payload_json,result_json FROM fulfilment_command WHERE request_key=?1",
                [&command.request_key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((payload, result)) = cached {
            if serde_json::from_str::<Command>(&payload)? != command {
                return Err("request key conflict".into());
            }
            return Ok(serde_json::from_str(&result)?);
        }
        if sqlite::metadata_key_exists(&tx,&command.request_key)?||tx.query_row("SELECT EXISTS(SELECT 1 FROM inventory_operation WHERE request_key=?1) OR EXISTS(SELECT 1 FROM purchase_command WHERE request_key=?1)",[&command.request_key],|r|r.get::<_,bool>(0))?{return Err("request key belongs to another command".into());}
        let mut ledger = sqlite::load(&tx)?;
        if ledger.operations().len() != expected_inventory_sequence
            || profile_sequence(&tx)? != expected_profile_sequence
        {
            return Err("stock or metadata changed since fulfilment preview".into());
        }
        let before = states(&tx, &ledger)?
            .remove(&command.purchase_id)
            .unwrap_or_default();
        let source = crate::purchase_store::read_purchase(&tx, command.purchase_id)?
            .ok_or("purchase does not exist")?;
        let after = before.apply(&command, &source, &ledger)?;
        if let Some(op) = State::operation(&command, &ledger) {
            crate::physical_store::reversal_metadata(&tx, &ledger, &op)?;
            if crate::physical_store::physical(&op.body) {
                crate::physical_store::persist_physical(
                    &tx,
                    &mut ledger,
                    &op,
                    expected_profile_sequence,
                )?;
            } else {
                let result = ledger.append(op.clone())?;
                sqlite::persist_operation(&tx, &ledger, &op, &result)?;
            }
        }
        tx.execute("INSERT INTO fulfilment_command(id,request_key,purchase_id,inventory_sequence,profile_sequence,source_snapshot_json,payload_json,result_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![command.id.to_string(),command.request_key,command.purchase_id.to_string(),i64::try_from(expected_inventory_sequence)?,expected_profile_sequence,serde_json::to_string(&source)?,serde_json::to_string(&command)?,serde_json::to_string(&after)?])?;
        tx.commit()?;
        Ok(after)
    }
}
