//! Catalog and inventory metadata share the stock transaction boundary but never
//! write balances. Archived references survive; new selections must be active.
use crate::{
    settings_store::load_settings,
    sqlite::{Result, Store, has_catalog},
};
use brickxinator_core::{
    catalog::{self, Definition},
    ledger::{Body, Id, Operation},
    profiles::{self, Links, Profile, Target},
    settings::{Kind, Record, Vocabulary},
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision<C> {
    pub committed_at: String,
    pub actor: String,
    pub reason: String,
    pub related_operation_id: Option<Id>,
    pub change: C,
}

fn require_schema(connection: &Connection) -> Result<()> {
    if !has_catalog(connection)? {
        return Err("catalog and inventory links need schema 4; use a new development database until safe upgrades exist".into());
    }
    Ok(())
}
fn decode<T: DeserializeOwned>(value: String) -> Result<T> {
    Ok(serde_json::from_str(&value)?)
}
fn id(value: String) -> Result<Id> {
    Ok(serde_json::from_value(serde_json::Value::String(value))?)
}
fn target_name(target: Target) -> &'static str {
    match target {
        Target::Holding(_) => "Holding",
        Target::SetCopy(_) => "SetCopy",
    }
}
fn target(kind: String, value: String) -> Result<Target> {
    match kind.as_str() {
        "Holding" => Ok(Target::Holding(id(value)?)),
        "SetCopy" => Ok(Target::SetCopy(id(value)?)),
        _ => Err("invalid inventory profile target kind".into()),
    }
}

/// Shared key namespace. The excluded table is always a library literal.
fn other_key(connection: &Connection, key: &str, excluded: &str) -> Result<bool> {
    if crate::sqlite::finance_key_exists(connection, key)? {
        return Ok(true);
    }
    for table in [
        "inventory_operation",
        "purchase_command",
        "settings_command",
        "catalog_command",
        "profile_command",
    ] {
        if table != excluded
            && connection.query_row(
                &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE request_key=?1)"),
                [key],
                |r| r.get::<_, bool>(0),
            )?
        {
            return Ok(true);
        }
    }
    Ok(false)
}
fn cached<C: DeserializeOwned + PartialEq, R: DeserializeOwned>(
    connection: &Connection,
    table: &str,
    key: &str,
    command: &C,
) -> Result<Option<R>> {
    let row: Option<(String, String)> = connection
        .query_row(
            &format!("SELECT payload_json,result_json FROM {table} WHERE request_key=?1"),
            [key],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if let Some((payload, result)) = row {
        if decode::<C>(payload)? != *command {
            return Err("request key conflicts with an earlier payload".into());
        }
        return Ok(Some(decode(result)?));
    }
    Ok(None)
}

impl Store {
    pub fn catalog_history(&self, id: Id) -> Result<Vec<Revision<catalog::Change>>> {
        let tx = self.connection.unchecked_transaction()?;
        let settings = load_settings(&tx)?;
        load_catalog(&tx, &settings)?;
        let history = read_history(&tx, "catalog_command", id, None)?;
        tx.commit()?;
        Ok(history)
    }

    pub fn profile_history(&self, target: Target) -> Result<Vec<Revision<profiles::Change>>> {
        let tx = self.connection.unchecked_transaction()?;
        let settings = load_settings(&tx)?;
        let definitions = load_catalog(&tx, &settings)?;
        load_profiles(&tx, &settings, &definitions)?;
        let history = read_history(
            &tx,
            "profile_command",
            target.id(),
            Some(target_name(target)),
        )?;
        tx.commit()?;
        Ok(history)
    }

    pub fn catalog(&self) -> Result<BTreeMap<Id, Definition>> {
        let tx = self.connection.unchecked_transaction()?;
        let settings = load_settings(&tx)?;
        let definitions = load_catalog(&tx, &settings)?;
        tx.commit()?;
        Ok(definitions)
    }
    pub fn inventory_profiles(&self) -> Result<BTreeMap<Target, Profile>> {
        let tx = self.connection.unchecked_transaction()?;
        let settings = load_settings(&tx)?;
        let definitions = load_catalog(&tx, &settings)?;
        let profiles = load_profiles(&tx, &settings, &definitions)?;
        tx.commit()?;
        Ok(profiles)
    }
    pub fn default_inventory_links(&self) -> Result<Links> {
        default_links(&self.settings()?)
    }

    pub fn execute_catalog(&mut self, command: catalog::Command) -> Result<catalog::Change> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let settings = load_settings(&tx)?;
        let definitions = load_catalog(&tx, &settings)?;
        if let Some(result) = cached(&tx, "catalog_command", &command.request_key, &command)? {
            tx.commit()?;
            return Ok(result);
        }
        if other_key(&tx, &command.request_key, "catalog_command")? {
            return Err("request key already belongs to another command".into());
        }
        let change = catalog::prepare(&definitions, &settings, &command, false)?;
        write_definition(&tx, &settings, &change.after)?;
        tx.execute("INSERT INTO catalog_command(id,request_key,actor,reason,target_id,payload_json,result_json) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![command.id.to_string(),command.request_key,command.actor,command.reason,change.after.id.to_string(),serde_json::to_string(&command)?,serde_json::to_string(&change)?])?;
        tx.commit()?;
        Ok(change)
    }

    pub fn execute_profile(&mut self, command: profiles::Command) -> Result<profiles::Change> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let settings = load_settings(&tx)?;
        let definitions = load_catalog(&tx, &settings)?;
        let records = load_profiles(&tx, &settings, &definitions)?;
        if let Some(result) = cached(&tx, "profile_command", &command.request_key, &command)? {
            tx.commit()?;
            return Ok(result);
        }
        if let Target::Holding(id) = command.target
            && crate::sqlite::has_verification(&tx)?
            && tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM verified_content WHERE holding_id=?1)",
                [id.to_string()],
                |r| r.get::<_, bool>(0),
            )?
        {
            return Err("verified set contents inherit their set location and cannot be independently reassigned".into());
        }
        if other_key(&tx, &command.request_key, "profile_command")? {
            return Err("request key already belongs to another command".into());
        }
        let change = profiles::prepare(
            records.get(&command.target).cloned(),
            &settings,
            &definitions,
            &command,
            false,
        )?;
        persist_profile(&tx, &command, &change, None)?;
        tx.commit()?;
        Ok(change)
    }
}

fn read_history<C: DeserializeOwned>(
    connection: &Connection,
    table: &str,
    id: Id,
    kind: Option<&str>,
) -> Result<Vec<Revision<C>>> {
    let related = if kind.is_some() {
        "related_operation_id"
    } else {
        "NULL"
    };
    let filter = if kind.is_some() {
        "AND target_kind=?2"
    } else {
        "AND ?2 IS NULL"
    };
    let mut query = connection.prepare(&format!("SELECT committed_at,actor,reason,{related},result_json FROM {table} WHERE target_id=?1 {filter} ORDER BY sequence"))?;
    let mut rows = query.query(params![id.to_string(), kind])?;
    let mut history = Vec::new();
    while let Some(row) = rows.next()? {
        history.push(Revision {
            committed_at: row.get(0)?,
            actor: row.get(1)?,
            reason: row.get(2)?,
            related_operation_id: row
                .get::<_, Option<String>>(3)?
                .map(crate::catalog_store::id)
                .transpose()?,
            change: decode(row.get(4)?)?,
        });
    }
    Ok(history)
}

fn identity_number(settings: &BTreeMap<Id, Record>, definition: &Definition) -> Option<String> {
    if matches!(
        settings
            .get(&definition.details.manufacturer_id)
            .map(|r| &r.kind),
        Some(Kind::Vocabulary {
            system_unknown: true,
            ..
        })
    ) {
        None
    } else {
        catalog::normalized_number(&definition.details.number)
    }
}
fn write_definition(
    connection: &Connection,
    settings: &BTreeMap<Id, Record>,
    r: &Definition,
) -> Result<()> {
    let kind = match r.kind {
        catalog::Kind::Part => "Part",
        catalog::Kind::Set => "Set",
    };
    connection.execute("INSERT INTO catalog_definition(id,kind,revision,manufacturer_id,identity_number,archived,snapshot_json) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,manufacturer_id=excluded.manufacturer_id,identity_number=excluded.identity_number,archived=excluded.archived,snapshot_json=excluded.snapshot_json",
        params![r.id.to_string(),kind,i64::try_from(r.revision)?,r.details.manufacturer_id.to_string(),identity_number(settings,r),r.archived,serde_json::to_string(r)?])?;
    Ok(())
}

pub(crate) fn load_catalog(
    connection: &Connection,
    settings: &BTreeMap<Id, Record>,
) -> Result<BTreeMap<Id, Definition>> {
    require_schema(connection)?;
    let mut records = BTreeMap::new();
    let mut query=connection.prepare("SELECT id,request_key,actor,reason,target_id,payload_json,result_json FROM catalog_command ORDER BY sequence")?;
    let mut rows = query.query([])?;
    while let Some(row) = rows.next()? {
        let command: catalog::Command = decode(row.get(5)?)?;
        let result: catalog::Change = decode(row.get(6)?)?;
        if command.id.to_string() != row.get::<_, String>(0)?
            || command.request_key != row.get::<_, String>(1)?
            || command.actor != row.get::<_, String>(2)?
            || command.reason != row.get::<_, String>(3)?
            || command.action.target().to_string() != row.get::<_, String>(4)?
        {
            return Err("catalog audit envelope mismatch".into());
        }
        if catalog::prepare(&records, settings, &command, true)? != result {
            return Err("catalog audit does not replay".into());
        }
        records.insert(result.after.id, result.after);
    }
    let mut current = BTreeMap::new();
    let mut query=connection.prepare("SELECT id,kind,revision,manufacturer_id,identity_number,archived,snapshot_json FROM catalog_definition")?;
    let mut rows = query.query([])?;
    while let Some(row) = rows.next()? {
        let record: Definition = decode(row.get(6)?)?;
        let kind = match record.kind {
            catalog::Kind::Part => "Part",
            catalog::Kind::Set => "Set",
        };
        if record.id.to_string() != row.get::<_, String>(0)?
            || kind != row.get::<_, String>(1)?
            || i64::try_from(record.revision)? != row.get::<_, i64>(2)?
            || record.details.manufacturer_id.to_string() != row.get::<_, String>(3)?
            || identity_number(settings, &record) != row.get::<_, Option<String>>(4)?
            || record.archived != row.get::<_, bool>(5)?
        {
            return Err("catalog columns differ from snapshot".into());
        }
        current.insert(record.id, record);
    }
    if records != current {
        return Err("catalog differs from its immutable audit".into());
    }
    Ok(current)
}

fn persist_profile(
    connection: &Connection,
    command: &profiles::Command,
    change: &profiles::Change,
    related_operation: Option<Id>,
) -> Result<()> {
    let r = &change.after;
    let (holding, copy) = match r.target {
        Target::Holding(id) => (Some(id.to_string()), None),
        Target::SetCopy(id) => (None, Some(id.to_string())),
    };
    connection.execute("INSERT INTO inventory_profile(target_kind,target_id,holding_id,set_copy_id,revision,definition_id,color_id,condition_id,location_id,snapshot_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10) ON CONFLICT(target_kind,target_id) DO UPDATE SET revision=excluded.revision,definition_id=excluded.definition_id,color_id=excluded.color_id,condition_id=excluded.condition_id,location_id=excluded.location_id,snapshot_json=excluded.snapshot_json",
        params![target_name(r.target),r.target.id().to_string(),holding,copy,i64::try_from(r.revision)?,r.links.definition_id.map(|id|id.to_string()),r.links.color_id.to_string(),r.links.condition_id.to_string(),r.links.location_id.map(|id|id.to_string()),serde_json::to_string(r)?])?;
    connection.execute("INSERT INTO profile_command(id,request_key,actor,reason,target_kind,target_id,related_operation_id,payload_json,result_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![command.id.to_string(),command.request_key,command.actor,command.reason,target_name(r.target),r.target.id().to_string(),related_operation.map(|id|id.to_string()),serde_json::to_string(command)?,serde_json::to_string(change)?])?;
    Ok(())
}

pub(crate) fn load_profiles(
    connection: &Connection,
    settings: &BTreeMap<Id, Record>,
    definitions: &BTreeMap<Id, Definition>,
) -> Result<BTreeMap<Target, Profile>> {
    let mut records = BTreeMap::new();
    let mut query=connection.prepare("SELECT id,request_key,actor,reason,target_kind,target_id,related_operation_id,payload_json,result_json FROM profile_command ORDER BY sequence")?;
    let mut rows = query.query([])?;
    while let Some(row) = rows.next()? {
        let command: profiles::Command = decode(row.get(7)?)?;
        let result: profiles::Change = decode(row.get(8)?)?;
        if command.id.to_string() != row.get::<_, String>(0)?
            || command.request_key != row.get::<_, String>(1)?
            || command.actor != row.get::<_, String>(2)?
            || command.reason != row.get::<_, String>(3)?
            || command.target != target(row.get(4)?, row.get(5)?)?
        {
            return Err("profile audit envelope mismatch".into());
        }
        if let Some(operation_id) = row.get::<_, Option<String>>(6)? {
            let operation: Operation = decode(connection.query_row(
                "SELECT payload_json FROM inventory_operation WHERE id=?1",
                [operation_id],
                |r| r.get(0),
            )?)?;
            let Body::Identify {
                source_id,
                child_id,
                ..
            } = operation.body
            else {
                return Err("profile inheritance needs an identification operation".into());
            };
            let mut links = default_links(settings)?;
            links.location_id = records
                .get(&Target::Holding(source_id))
                .map(|r: &Profile| r.links.location_id)
                .ok_or("inherited profile lacks source profile")?;
            if command.target != Target::Holding(child_id)
                || command.links != links
                || command.actor != operation.actor
            {
                return Err("inherited profile differs from its source facts".into());
            }
        }
        let expected = profiles::prepare(
            records.get(&command.target).cloned(),
            settings,
            definitions,
            &command,
            true,
        )?;
        if expected != result {
            return Err("profile audit does not replay".into());
        }
        records.insert(result.after.target, result.after);
    }
    let mut current = BTreeMap::new();
    let mut query=connection.prepare("SELECT target_kind,target_id,revision,definition_id,color_id,condition_id,location_id,snapshot_json FROM inventory_profile")?;
    let mut rows = query.query([])?;
    while let Some(row) = rows.next()? {
        let record: Profile = decode(row.get(7)?)?;
        if record.target != target(row.get(0)?, row.get(1)?)?
            || i64::try_from(record.revision)? != row.get::<_, i64>(2)?
            || record.links.definition_id.map(|id| id.to_string())
                != row.get::<_, Option<String>>(3)?
            || record.links.color_id.to_string() != row.get::<_, String>(4)?
            || record.links.condition_id.to_string() != row.get::<_, String>(5)?
            || record.links.location_id.map(|id| id.to_string())
                != row.get::<_, Option<String>>(6)?
        {
            return Err("profile columns differ from snapshot".into());
        }
        current.insert(record.target, record);
    }
    if records != current {
        return Err("inventory profiles differ from immutable audit".into());
    }
    Ok(current)
}

pub(crate) fn default_links(settings: &BTreeMap<Id, Record>) -> Result<Links> {
    let unknown = |vocabulary| {
        settings.values().find(|r|matches!(r.kind,Kind::Vocabulary {vocabulary:v,system_unknown:true} if v==vocabulary)).map(|r|r.id).ok_or("missing system Unknown")
    };
    Ok(Links {
        archived: false,
        definition_id: None,
        color_id: unknown(Vocabulary::Color)?,
        condition_id: unknown(Vocabulary::Condition)?,
        location_id: None,
    })
}

pub(crate) fn inherit_location(connection: &Connection, operation: &Operation) -> Result<()> {
    let Body::Identify {
        source_id,
        child_id,
        ..
    } = operation.body
    else {
        return Ok(());
    };
    let settings = load_settings(connection)?;
    let definitions = load_catalog(connection, &settings)?;
    let records = load_profiles(connection, &settings, &definitions)?;
    let Some(source) = records.get(&Target::Holding(source_id)) else {
        return Ok(());
    };
    let mut links = default_links(&settings)?;
    links.location_id = source.links.location_id;
    let command = profiles::Command {
        id: Id::new(),
        request_key: format!("inventory:{}:inherited-profile", operation.id),
        actor: operation.actor.clone(),
        reason: "Inherit source lot location during identification".into(),
        target: Target::Holding(child_id),
        expected_revision: 0,
        links,
    };
    if other_key(connection, &command.request_key, "profile_command")? {
        return Err("inherited profile request key conflicts".into());
    }
    // Inherited archived references remain valid, like the source's retained link.
    let change = profiles::prepare(None, &settings, &definitions, &command, true)?;
    persist_profile(connection, &command, &change, Some(operation.id))
}
