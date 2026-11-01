//! Shipping settings and exact cost views; never a stock or paid-price mutation.
use crate::{
    purchase_store::read_purchase,
    sqlite::{Result, Store, has_finance},
};
use brickxinator_core::{
    finance::{self, Change, Command, Costs, Method, State},
    ledger::Id,
    purchases::Purchase,
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllocationResult {
    pub change: Change,
    pub costs: Costs,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision {
    pub committed_at: String,
    pub actor: String,
    pub reason: String,
    pub source: Purchase,
    pub result: AllocationResult,
}

impl Store {
    pub fn purchase_costs(&self, purchase_id: Id) -> Result<Costs> {
        let tx = self.connection.unchecked_transaction()?;
        require_schema(&tx)?;
        let (state, _) = read_state(&tx, purchase_id)?;
        let purchase = read_purchase(&tx, purchase_id)?.ok_or("purchase does not exist")?;
        let costs = finance::costs(&purchase, &state.unwrap_or_else(|| State::new(purchase_id)))?;
        tx.commit()?;
        Ok(costs)
    }

    pub fn allocation_history(&self, purchase_id: Id) -> Result<Vec<Revision>> {
        let tx = self.connection.unchecked_transaction()?;
        require_schema(&tx)?;
        let (_, history) = read_state(&tx, purchase_id)?;
        tx.commit()?;
        Ok(history)
    }

    pub fn allocation_states(&self) -> Result<BTreeMap<Id, State>> {
        let tx = self.connection.unchecked_transaction()?;
        require_schema(&tx)?;
        let ids:Vec<String>=tx.prepare("SELECT purchase_id FROM shipping_allocation UNION SELECT purchase_id FROM shipping_command")?.query_map([],|r|r.get(0))?.collect::<rusqlite::Result<_>>()?;
        let mut states = BTreeMap::new();
        for value in ids {
            let id: Id = serde_json::from_value(serde_json::Value::String(value))?;
            let (state, _) = read_state(&tx, id)?;
            states.insert(id, state.ok_or("shipping state missing")?);
        }
        tx.commit()?;
        Ok(states)
    }

    pub fn execute_allocation(&mut self, command: Command) -> Result<AllocationResult> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_schema(&tx)?;
        let (state, _) = read_state(&tx, command.purchase_id)?;
        let prior: Option<(String, String)> = tx
            .query_row(
                "SELECT payload_json,result_json FROM shipping_command WHERE request_key=?1",
                [&command.request_key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((payload, result)) = prior {
            if serde_json::from_str::<Command>(&payload)? != command {
                return Err("shipping request key conflicts with an earlier payload".into());
            }
            let result = serde_json::from_str(&result)?;
            tx.commit()?;
            return Ok(result);
        }
        if crate::sqlite::attachment_key_exists(&tx, &command.request_key)? {
            return Err("request key belongs to an attachment command".into());
        }
        for table in [
            "inventory_operation",
            "purchase_command",
            "settings_command",
            "catalog_command",
            "profile_command",
        ] {
            if tx.query_row(
                &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE request_key=?1)"),
                [&command.request_key],
                |r| r.get::<_, bool>(0),
            )? {
                return Err("request key belongs to another command".into());
            }
        }
        let purchase = read_purchase(&tx, command.purchase_id)?.ok_or("purchase does not exist")?;
        let change = finance::prepare(state, &purchase, &command)?;
        let costs = finance::costs(&purchase, &change.after)?;
        let result = AllocationResult { change, costs };
        let state = &result.change.after;
        tx.execute("INSERT INTO shipping_allocation(purchase_id,revision,method,snapshot_json) VALUES(?1,?2,?3,?4) ON CONFLICT(purchase_id) DO UPDATE SET revision=excluded.revision,method=excluded.method,snapshot_json=excluded.snapshot_json",
            params![state.purchase_id.to_string(),i64::try_from(state.revision)?,method_name(state.method),serde_json::to_string(state)?])?;
        tx.execute("INSERT INTO shipping_command(id,request_key,purchase_id,actor,reason,source_snapshot_json,payload_json,result_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![command.id.to_string(),command.request_key,command.purchase_id.to_string(),command.actor,command.reason,serde_json::to_string(&purchase)?,serde_json::to_string(&command)?,serde_json::to_string(&result)?])?;
        tx.commit()?;
        Ok(result)
    }
}

fn require_schema(connection: &Connection) -> Result<()> {
    if !has_finance(connection)? {
        return Err(
            "shipping allocation/cost views need schema 8; automatic upgrades are not implemented"
                .into(),
        );
    }
    Ok(())
}
fn method_name(method: Method) -> &'static str {
    match method {
        Method::PurchaseLevel => "PurchaseLevel",
        Method::Equal => "Equal",
        Method::PriceProportional => "PriceProportional",
        Method::PieceCount => "PieceCount",
        Method::Weight => "Weight",
        Method::Manual => "Manual",
    }
}

fn read_state(connection: &Connection, purchase_id: Id) -> Result<(Option<State>, Vec<Revision>)> {
    let mut state = None;
    let mut history = vec![];
    let mut query=connection.prepare("SELECT id,request_key,actor,reason,committed_at,source_snapshot_json,payload_json,result_json FROM shipping_command WHERE purchase_id=?1 ORDER BY sequence")?;
    let mut rows = query.query([purchase_id.to_string()])?;
    while let Some(row) = rows.next()? {
        let command: Command = serde_json::from_str(&row.get::<_, String>(6)?)?;
        let source: Purchase = serde_json::from_str(&row.get::<_, String>(5)?)?;
        let result: AllocationResult = serde_json::from_str(&row.get::<_, String>(7)?)?;
        if command.id.to_string() != row.get::<_, String>(0)?
            || command.request_key != row.get::<_, String>(1)?
            || command.actor != row.get::<_, String>(2)?
            || command.reason != row.get::<_, String>(3)?
            || command.purchase_id != purchase_id
            || source.id != purchase_id
        {
            return Err("shipping audit envelope/source mismatch".into());
        }
        source.draft.validate()?;
        let audited_source:String=connection.query_row("SELECT after_json FROM purchase_command WHERE purchase_id=?1 AND json_extract(after_json,'$.revision')=?2",params![purchase_id.to_string(),i64::try_from(source.revision)?],|r|r.get(0))?;
        if serde_json::from_str::<Purchase>(&audited_source)? != source {
            return Err("shipping source differs from audited purchase revision".into());
        }
        let change = finance::prepare(state, &source, &command)?;
        let costs = finance::costs(&source, &change.after)?;
        if result
            != (AllocationResult {
                change: change.clone(),
                costs,
            })
        {
            return Err("shipping audit result does not replay".into());
        }
        state = Some(change.after);
        history.push(Revision {
            committed_at: row.get(4)?,
            actor: command.actor,
            reason: command.reason,
            source,
            result,
        });
    }
    let stored: Option<(i64, String, String)> = connection
        .query_row(
            "SELECT revision,method,snapshot_json FROM shipping_allocation WHERE purchase_id=?1",
            [purchase_id.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let actual = if let Some((revision, method, json)) = stored {
        let state: State = serde_json::from_str(&json)?;
        if state.purchase_id != purchase_id
            || i64::try_from(state.revision)? != revision
            || method_name(state.method) != method
        {
            return Err("shipping state columns differ from snapshot".into());
        }
        Some(state)
    } else {
        None
    };
    if actual != state {
        return Err("shipping allocation differs from immutable audit history".into());
    }
    Ok((state, history))
}
