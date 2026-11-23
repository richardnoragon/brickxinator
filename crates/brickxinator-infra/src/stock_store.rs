//! Explicit stock and lot commands share the immutable inventory journal.
use crate::sqlite::{
    Result, Store, has_stock_actions, load, metadata_key_exists, persist_operation,
};
use brickxinator_core::{
    journal::Holding,
    ledger::{Body, Id, Ledger, Operation, Outcome},
    purchases::{InvoiceLine, LineKind},
    stock::{Action, Adjustment, Classification, Command, Removal},
};
use rusqlite::{Connection, TransactionBehavior, params};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision {
    pub operation_id: Id,
    pub committed_at: String,
    pub actor: String,
    pub reason: String,
    pub before: Option<Holding>,
    pub after: Holding,
    pub classification_before: Classification,
    pub classification_after: Classification,
}

impl Store {
    pub fn execute_stock(&mut self, command: Command) -> Result<Outcome> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_schema(&tx)?;
        let mut ledger = load(&tx)?;
        let reason = if matches!(command.action, Action::Complete { .. })
            && command.reason.trim().is_empty()
        {
            "Complete mixed-lot classification".into()
        } else {
            command.reason
        };
        let operation = Operation {
            id: command.id,
            request_key: command.request_key,
            actor: command.actor,
            reason,
            body: Body::Stock {
                holding_id: command.holding_id,
                expected_revision: command.expected_revision,
                action: command.action,
            },
        };
        if !ledger
            .operations()
            .iter()
            .any(|op| op.request_key == operation.request_key)
        {
            if metadata_key_exists(&tx, &operation.request_key)?
                || tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM purchase_command WHERE request_key=?1)",
                    [&operation.request_key],
                    |r| r.get::<_, bool>(0),
                )?
            {
                return Err("request key already belongs to another command".into());
            }
            if matches!(
                operation.body,
                Body::Stock {
                    action: Action::Complete { .. } | Action::Reopen,
                    ..
                }
            ) {
                require_original_lot(&tx, command.holding_id)?;
            }
        }
        let count = ledger.operations().len();
        let outcome = ledger.append(operation.clone())?;
        if ledger.operations().len() != count {
            persist_operation(&tx, &ledger, &operation, &outcome)?;
        }
        tx.commit()?;
        Ok(outcome)
    }

    /// Current holdings exclude explicitly completed residual lots. Historical
    /// measurements stay accessible through rebuild()/holding_history().
    pub fn current_inventory(&self) -> Result<BTreeMap<Id, Holding>> {
        Ok(self.rebuild()?.current_holdings())
    }

    pub fn holding_history(&self, holding_id: Id) -> Result<Vec<Revision>> {
        let tx = self.connection.unchecked_transaction()?;
        require_schema(&tx)?;
        let current = load(&tx)?;
        if !current.holdings().contains_key(&holding_id) {
            return Err("holding does not exist".into());
        }
        let mut replay = Ledger::default();
        let mut history = Vec::new();
        {
            let mut query = tx.prepare(
                "SELECT committed_at,payload_json FROM inventory_operation ORDER BY sequence",
            )?;
            let mut rows = query.query([])?;
            while let Some(row) = rows.next()? {
                let op: Operation = serde_json::from_str(&row.get::<_, String>(1)?)?;
                let before = replay.holdings().get(&holding_id).copied();
                let classification_before = replay
                    .classification()
                    .get(&holding_id)
                    .cloned()
                    .unwrap_or_default();
                let result = replay.append(op.clone())?;
                if let Some(after) = result.holdings.get(&holding_id) {
                    history.push(Revision {
                        operation_id: op.id,
                        committed_at: row.get(0)?,
                        actor: op.actor,
                        reason: op.reason,
                        before,
                        after: *after,
                        classification_before,
                        classification_after: replay
                            .classification()
                            .get(&holding_id)
                            .cloned()
                            .unwrap_or_default(),
                    });
                }
            }
        }
        tx.commit()?;
        Ok(history)
    }
}

fn require_schema(connection: &Connection) -> Result<()> {
    if !has_stock_actions(connection)? {
        return Err(
            "typed stock actions need schema 6; automatic upgrades are not implemented".into(),
        );
    }
    Ok(())
}

fn require_original_lot(connection: &Connection, holding_id: Id) -> Result<()> {
    let (json,parent):(String,Option<String>)=connection.query_row("SELECT invoice_line.snapshot_json,stock_provenance.parent_holding_id FROM stock_provenance JOIN invoice_line ON invoice_line.id=stock_provenance.invoice_line_id WHERE holding_id=?1",[holding_id.to_string()],|r|Ok((r.get(0)?,r.get(1)?)))?;
    let line: InvoiceLine = serde_json::from_str(&json)?;
    if line.kind != LineKind::MixedLot || parent.is_some() {
        return Err("classification lifecycle is only for an original received mixed lot".into());
    }
    Ok(())
}

fn kind(action: &Action) -> &'static str {
    match action {
        Action::Complete { .. } => "Complete",
        Action::Reopen => "Reopen",
        Action::Adjust(Adjustment::Broken { .. }) => "Broken",
        Action::Adjust(Adjustment::Correction { .. }) => "Correction",
        Action::Adjust(Adjustment::Remove { kind, .. }) => match kind {
            Removal::Lost => "Lost",
            Removal::Donated => "Donated",
            Removal::Sold => "Sold",
        },
    }
}
pub(crate) fn persist(connection: &Connection, operation: &Operation) -> Result<()> {
    if let Body::Stock {
        holding_id, action, ..
    } = &operation.body
    {
        connection.execute(
            "INSERT INTO stock_operation(operation_id,holding_id,action_kind) VALUES(?1,?2,?3)",
            params![
                operation.id.to_string(),
                holding_id.to_string(),
                kind(action)
            ],
        )?;
    }
    Ok(())
}
pub(crate) fn validate(connection: &Connection, ledger: &Ledger) -> Result<()> {
    let mut expected = BTreeMap::new();
    for op in ledger.operations() {
        if let Body::Stock {
            holding_id, action, ..
        } = &op.body
        {
            if matches!(action, Action::Complete { .. } | Action::Reopen) {
                require_original_lot(connection, *holding_id)?;
            }
            expected.insert(
                op.id.to_string(),
                (holding_id.to_string(), kind(action).to_string()),
            );
        }
    }
    let stored: BTreeMap<String, (String, String)> = connection
        .prepare("SELECT operation_id,holding_id,action_kind FROM stock_operation")?
        .query_map([], |r| Ok((r.get(0)?, (r.get(1)?, r.get(2)?))))?
        .collect::<rusqlite::Result<_>>()?;
    if stored != expected {
        return Err("stock action references differ from journal replay".into());
    }
    Ok(())
}
