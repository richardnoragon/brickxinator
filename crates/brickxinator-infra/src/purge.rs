//! Explicit draft purge retains only deletion/request identities, never entered metadata.
use crate::{
    purchase_store::read_purchase,
    sqlite::{Result, Store},
};
use brickxinator_core::{ledger::Id, purchases::Status};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub id: Id,
    pub purchase_id: Id,
    pub expected_revision: u64,
    pub request_key: String,
    pub actor: String,
}
pub(crate) fn available(connection: &rusqlite::Connection) -> Result<bool> {
    Ok(connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migration WHERE version=10)",
        [],
        |r| r.get(0),
    )?)
}
pub(crate) fn key_exists(connection: &rusqlite::Connection, key: &str) -> Result<bool> {
    Ok(available(connection)? && connection.query_row("SELECT EXISTS(SELECT 1 FROM purchase_deletion WHERE request_key=?1) OR EXISTS(SELECT 1 FROM deleted_request_key WHERE request_key=?1)",[key],|r|r.get(0))?)
}
pub(crate) fn validate(connection: &rusqlite::Connection) -> Result<()> {
    if !available(connection)? {
        return Ok(());
    }
    let mut query = connection
        .prepare("SELECT purchase_id,request_key,actor,payload_json FROM purchase_deletion")?;
    let rows = query.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
        ))
    })?;
    for row in rows {
        let (purchase, key, actor, payload) = row?;
        let command: Command = serde_json::from_str(&payload)?;
        if command.purchase_id.to_string() != purchase
            || command.request_key != key
            || command.actor != actor
        {
            return Err("deletion audit differs from command".into());
        }
        let exists:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM purchase WHERE id=?1) OR EXISTS(SELECT 1 FROM purchase_command WHERE purchase_id=?1) OR EXISTS(SELECT 1 FROM shipping_command WHERE purchase_id=?1)",[purchase],|r|r.get(0))?;
        if exists {
            return Err("deleted draft still has source metadata".into());
        }
    }
    Ok(())
}
impl Store {
    pub fn purge_draft(&mut self, command: Command) -> Result<()> {
        if !available(&self.connection)? {
            return Err("draft purge requires schema 10".into());
        }
        if command.actor.trim().is_empty() || command.request_key.trim().is_empty() {
            return Err("purge actor and request key are required".into());
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let prior: Option<String> = tx
            .query_row(
                "SELECT payload_json FROM purchase_deletion WHERE request_key=?1",
                [&command.request_key],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(prior) = prior {
            if serde_json::from_str::<Command>(&prior)? != command {
                return Err("purge request conflicts with previous command".into());
            }
            return Ok(());
        }
        if crate::sqlite::metadata_key_exists(&tx,&command.request_key)? || tx.query_row("SELECT EXISTS(SELECT 1 FROM purchase_command WHERE request_key=?1) OR EXISTS(SELECT 1 FROM inventory_operation WHERE request_key=?1)",[&command.request_key],|r|r.get::<_,bool>(0))? {return Err("purge request key already belongs to another command".into());}
        let purchase = read_purchase(&tx, command.purchase_id)?.ok_or("purchase does not exist")?;
        purchase.check_revision(command.expected_revision)?;
        if purchase.status == Status::Received || !purchase.draft.lines.is_empty() {
            return Err("only never-received drafts without lines may be purged".into());
        }
        // The insertion trigger also rejects attachments and receipt references.
        tx.execute("INSERT INTO purchase_deletion(purchase_id,request_key,actor,payload_json) VALUES(?1,?2,?3,?4)",params![command.purchase_id.to_string(),command.request_key,command.actor,serde_json::to_string(&command)?])?;
        let id = command.purchase_id.to_string();
        tx.execute("INSERT INTO deleted_request_key(request_key,purchase_id) SELECT request_key,purchase_id FROM purchase_command WHERE purchase_id=?1 UNION SELECT request_key,purchase_id FROM shipping_command WHERE purchase_id=?1",[&id])?;
        tx.execute("DELETE FROM shipping_command WHERE purchase_id=?1", [&id])?;
        tx.execute(
            "DELETE FROM shipping_allocation WHERE purchase_id=?1",
            [&id],
        )?;
        tx.execute("DELETE FROM purchase_command WHERE purchase_id=?1", [&id])?;
        tx.execute("DELETE FROM purchase WHERE id=?1", [&id])?;
        tx.commit()?;
        Ok(())
    }
}
