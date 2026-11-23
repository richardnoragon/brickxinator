//! Typed physical movement entry point. Source profiles are resolved historically.
use crate::{
    catalog_store::{default_links, persist_profile},
    settings_store::load_settings,
    sqlite::{self, Result, Store},
};
use brickxinator_core::{
    ledger::{Body, Id, Ledger, Operation, Outcome},
    physical::{Handling, Status},
    profiles::{self, Links, Target},
};
use rusqlite::{Connection, TransactionBehavior, params};
use std::collections::{BTreeMap, BTreeSet};
fn enabled(c: &Connection) -> Result<bool> {
    Ok(c.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migration WHERE version=14)",
        [],
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
fn records(c: &Connection, seq: i64) -> Result<BTreeMap<Target, profiles::Profile>> {
    let mut records = BTreeMap::new();
    let mut q =
        c.prepare("SELECT result_json FROM profile_command WHERE sequence<=?1 ORDER BY sequence")?;
    for row in q.query_map([seq], |r| r.get::<_, String>(0))? {
        let change: profiles::Change = serde_json::from_str(&row?)?;
        records.insert(change.after.target, change.after);
    }
    Ok(records)
}
fn links(
    c: &Connection,
    ledger: &Ledger,
    records: &BTreeMap<Target, profiles::Profile>,
    id: Id,
) -> Result<Links> {
    if let Some(p) = records.get(&Target::Holding(id)) {
        return Ok(p.links.clone());
    }
    let defaults = default_links(&load_settings(c)?)?;
    if let Some(copy) = ledger.original_contents().get(&id) {
        let mut result = records
            .get(&Target::SetCopy(*copy))
            .map(|p| p.links.clone())
            .unwrap_or(defaults);
        let line = ledger.verification()[copy]
            .lines
            .values()
            .find(|l| l.holding_id == id)
            .ok_or("original set content is missing")?;
        result.definition_id = line.definition_id;
        result.color_id = line.color_id;
        result.archived = false;
        return Ok(result);
    }
    Ok(defaults)
}
pub(crate) fn physical(body: &Body) -> bool {
    matches!(
        body,
        Body::Split { .. } | Body::Merge { .. } | Body::Detach { .. } | Body::Lifecycle(_)
    )
}
fn flows(body: &Body) -> Vec<(Id, Id)> {
    match body {
        Body::Split {
            source_id,
            child_id,
            ..
        } => vec![(*source_id, *child_id)],
        Body::Merge { sources, target_id } => {
            sources.iter().map(|s| (s.holding_id, *target_id)).collect()
        }
        Body::Lifecycle(e) => e
            .parts
            .iter()
            .filter_map(|p| match p.handling {
                Handling::Custom { child_id, .. } => Some((p.holding_id, child_id)),
                _ => None,
            })
            .collect(),
        _ => vec![],
    }
}
fn compatible(c: &Connection, ledger: &Ledger, op: &Operation, seq: i64) -> Result<()> {
    if let Body::Merge { sources, .. } = &op.body {
        let records = records(c, seq)?;
        let mut expected = None;
        for source in sources {
            let mut actual = links(c, ledger, &records, source.holding_id)?;
            actual.archived = false;
            if actual.definition_id.is_none() {
                return Err("identify the part before merging compatible stock".into());
            }
            if expected.as_ref().is_some_and(|old| old != &actual) {
                return Err(
                    "merge requires matching part, color, condition and physical location".into(),
                );
            }
            expected = Some(actual);
        }
    }
    Ok(())
}
pub(crate) fn validate(c: &Connection, ledger: &Ledger) -> Result<()> {
    if !enabled(c)? {
        if ledger.operations().iter().any(|o| physical(&o.body)) {
            return Err("physical events need schema 14".into());
        }
        return Ok(());
    }
    let contexts: BTreeMap<String, i64> = c
        .prepare("SELECT operation_id,profile_sequence FROM physical_operation")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let actual: BTreeSet<(String, String, String)> = c
        .prepare("SELECT operation_id,source_id,target_id FROM holding_flow")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let mut expected = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut replay = Ledger::default();
    let max = profile_sequence(c)?;
    let mut last = 0;
    for op in ledger.operations() {
        if physical(&op.body) {
            let seq = *contexts
                .get(&op.id.to_string())
                .ok_or("physical operation has no metadata context")?;
            if seq < last || seq > max {
                return Err("invalid physical metadata prefix".into());
            }
            last = seq;
            compatible(c, &replay, op, seq)?;
            ids.insert(op.id.to_string());
            for (source, target) in flows(&op.body) {
                expected.insert((op.id.to_string(), source.to_string(), target.to_string()));
            }
        }
        replay.append(op.clone())?;
    }
    if expected != actual || ids != contexts.keys().cloned().collect() {
        return Err("physical provenance differs from immutable journal".into());
    }
    Ok(())
}
fn write_profile(
    c: &Connection,
    ledger: &Ledger,
    op: &Operation,
    id: Id,
    mut links: Links,
    archived: bool,
) -> Result<()> {
    links.archived = archived;
    let settings = load_settings(c)?;
    let defs = crate::catalog_store::load_catalog(c, &settings)?;
    let records = crate::catalog_store::load_profiles(c, &settings, &defs)?;
    let before = records.get(&Target::Holding(id)).cloned();
    let command = profiles::Command {
        id: Id::new(),
        request_key: format!("physical:{}:{}", op.id, id),
        actor: op.actor.clone(),
        reason: format!("Physical movement: {}", op.reason),
        target: Target::Holding(id),
        expected_revision: before.as_ref().map_or(0, |p| p.revision),
        links,
    };
    let change = profiles::prepare(before, &settings, &defs, &command, true)?;
    if !ledger.holdings().contains_key(&id) {
        return Err("movement profile holding is missing".into());
    }
    persist_profile(c, &command, &change, None)
}
pub(crate) fn reversal_metadata(c: &Connection, ledger: &Ledger, op: &Operation) -> Result<()> {
    if let Body::SupplierReturnCorrection {
        dispatch_operation_id,
    } = &op.body
        && let Some(Body::SupplierReturn { holding_id, .. }) = ledger
            .operations()
            .iter()
            .find(|o| o.id == *dispatch_operation_id)
            .map(|o| &o.body)
    {
        let since: i64 = c.query_row(
            "SELECT profile_sequence FROM fulfilment_command WHERE id=?1",
            [dispatch_operation_id.to_string()],
            |r| r.get(0),
        )?;
        let changed:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM profile_command WHERE sequence>?1 AND target_kind='Holding' AND target_id=?2)",params![since,holding_id.to_string()],|r|r.get(0))?;
        if changed {
            return Err("return correction blocked: stock metadata changed after dispatch".into());
        }
    }

    if let Body::Lifecycle(e) = &op.body
        && let Some(corrects) = e.corrects
    {
        let effect = ledger
            .lifecycle_effects()
            .get(&corrects)
            .ok_or("lifecycle event does not exist")?;
        let since: i64 = c.query_row(
            "SELECT profile_sequence FROM physical_operation WHERE operation_id=?1",
            [corrects.to_string()],
            |r| r.get(0),
        )?;
        let mut q = c.prepare(
            "SELECT target_kind,target_id,request_key FROM profile_command WHERE sequence>?1",
        )?;
        for row in q.query_map([since], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })? {
            let (kind, id, key) = row?;
            if !key.starts_with(&format!("physical:{corrects}:"))
                && ((kind == "Holding"
                    && effect.after_holdings.keys().any(|h| h.to_string() == id))
                    || (kind == "SetCopy" && e.copy_id.to_string() == id))
            {
                return Err(
                    "reversal blocked: affected stock metadata changed after disposition".into(),
                );
            }
        }
    }
    Ok(())
}
pub(crate) fn persist_physical(
    c: &Connection,
    ledger: &mut Ledger,
    op: &Operation,
    seq: i64,
) -> Result<Outcome> {
    compatible(c, ledger, op, seq)?;
    let before = records(c, seq)?;
    reversal_metadata(c, ledger, op)?;
    let mut inherited = BTreeMap::new();
    for (source, target) in flows(&op.body) {
        inherited
            .entry(target)
            .or_insert(links(c, ledger, &before, source)?);
    }
    match &op.body {
        Body::Detach { holding_id, .. } => {
            inherited.insert(*holding_id, links(c, ledger, &before, *holding_id)?);
        }
        Body::Lifecycle(e) => {
            for p in &e.parts {
                if matches!(p.handling, Handling::KeepAll) {
                    inherited.insert(p.holding_id, links(c, ledger, &before, p.holding_id)?);
                }
            }
        }
        _ => {}
    }
    let outcome = ledger.append(op.clone())?;
    sqlite::persist_operation(c, ledger, op, &outcome)?;
    c.execute(
        "INSERT INTO physical_operation(operation_id,profile_sequence) VALUES(?1,?2)",
        params![op.id.to_string(), seq],
    )?;
    for (source, target) in flows(&op.body) {
        c.execute(
            "INSERT INTO holding_flow(operation_id,source_id,target_id) VALUES(?1,?2,?3)",
            params![op.id.to_string(), source.to_string(), target.to_string()],
        )?;
    }
    for (id, links) in inherited {
        write_profile(c, ledger, op, id, links, false)?;
    }
    if let Body::Merge { sources, .. } = &op.body {
        for source in sources {
            write_profile(
                c,
                ledger,
                op,
                source.holding_id,
                links(c, ledger, &before, source.holding_id)?,
                true,
            )?;
        }
    }
    if let Body::Lifecycle(e) = &op.body
        && e.status != Status::Active
    {
        for p in &e.parts {
            if !matches!(p.handling, Handling::KeepAll) {
                write_profile(
                    c,
                    ledger,
                    op,
                    p.holding_id,
                    links(c, ledger, &before, p.holding_id)?,
                    true,
                )?;
            }
        }
    }
    crate::project_store::validate_current(c, ledger)?;
    Ok(outcome)
}
impl Store {
    pub fn execute_physical(
        &mut self,
        op: Operation,
        expected_profile_sequence: i64,
    ) -> Result<Outcome> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if !enabled(&tx)? || !physical(&op.body) {
            return Err("typed physical movement requires schema 14".into());
        }
        let mut ledger = sqlite::load(&tx)?;
        if ledger
            .operations()
            .iter()
            .any(|old| old.request_key == op.request_key)
        {
            return Ok(ledger.append(op)?);
        }
        if sqlite::metadata_key_exists(&tx, &op.request_key)?
            || tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM purchase_command WHERE request_key=?1)",
                [&op.request_key],
                |r| r.get::<_, bool>(0),
            )?
        {
            return Err("request key belongs to another command".into());
        }
        let seq = profile_sequence(&tx)?;
        if seq != expected_profile_sequence {
            return Err("stock metadata changed since physical preview".into());
        }
        let outcome = persist_physical(&tx, &mut ledger, &op, seq)?;
        tx.commit()?;
        Ok(outcome)
    }
    pub fn physical_history(&self) -> Result<serde_json::Value> {
        let ledger = self.rebuild()?;
        let flows: Vec<_> = ledger
            .operations()
            .iter()
            .filter(|op| physical(&op.body))
            .cloned()
            .collect();
        Ok(serde_json::to_value(flows)?)
    }
}

impl Store {
    pub fn preview_physical(&self, op: Operation) -> Result<serde_json::Value> {
        let tx = self.connection.unchecked_transaction()?;
        if !enabled(&tx)? || !physical(&op.body) {
            return Err("physical preview requires schema 14 and a supported action".into());
        }
        let mut ledger = sqlite::load(&tx)?;
        let seq = profile_sequence(&tx)?;
        compatible(&tx, &ledger, &op, seq)?;
        reversal_metadata(&tx, &ledger, &op)?;
        let before: BTreeMap<Id, _> = ledger.holdings().clone();
        let result = ledger.append(op.clone())?;
        crate::project_store::validate_current(&tx, &ledger)?;
        let effects: Vec<_> = result
            .holdings
            .iter()
            .map(|(id, after)| serde_json::json!({"id":id,"before":before.get(id),"after":after}))
            .collect();
        Ok(serde_json::json!({"operation":op,"expected_profile_sequence":seq,"effects":effects}))
    }
    pub fn physical_view(&self) -> Result<serde_json::Value> {
        let tx = self.connection.unchecked_transaction()?;
        let ledger = sqlite::load(&tx)?;
        let source_layers = brickxinator_core::source_layers::project(&ledger);
        let seq = profile_sequence(&tx)?;
        let profiles = records(&tx, seq)?;
        let settings = load_settings(&tx)?;
        let defs = crate::catalog_store::load_catalog(&tx, &settings)?;
        let mut holdings = BTreeMap::new();
        for (id, h) in ledger.current_holdings() {
            let l = links(&tx, &ledger, &profiles, id)?;
            let label = l
                .definition_id
                .and_then(|id| defs.get(&id))
                .map(|d| {
                    d.details
                        .name
                        .clone()
                        .or_else(|| d.details.number.clone())
                        .unwrap_or_else(|| "Unnamed part".into())
                })
                .unwrap_or_else(|| "Unidentified stock".into());
            holdings.insert(id,serde_json::json!({"id":id,"holding":h,"acquisition_layers":source_layers.get(&id),"copy_id":ledger.set_contents().get(&id),"links":l,"label":label,"location":l.location_id.and_then(|id|settings.get(&id).map(|r|r.name.clone())),"color":settings.get(&l.color_id).map(|r|&r.name),"condition":settings.get(&l.condition_id).map(|r|&r.name)}));
        }
        let copies:Vec<_>=ledger.set_copies().values().map(|c|serde_json::json!({"id":c.id,"invoice_line_id":c.invoice_line_id,"label":ledger.copy_label(c.id),"lifecycle":ledger.lifecycle(c.id),"linked":ledger.set_contents().iter().filter(|(_,copy)|**copy==c.id).map(|(h,_)|*h).collect::<Vec<_>>()})).collect();
        let history: Vec<_> = ledger
            .operations()
            .iter()
            .filter(|o| physical(&o.body))
            .rev()
            .take(100)
            .collect();
        let flow:Vec<_>=ledger.operations().iter().flat_map(|o|flows(&o.body).into_iter().map(move|(source,target)|serde_json::json!({"operation_id":o.id,"source":source,"target":target}))).collect();
        Ok(
            serde_json::json!({"holdings":holdings,"copies":copies,"history":history,"flow":flow,"profile_sequence":seq}),
        )
    }
}
