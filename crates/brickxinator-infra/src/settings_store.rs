//! Audited settings commands. No settings edit changes inventory ownership.
use crate::sqlite::{Result, Store, has_settings};
use brickxinator_core::{
    ledger::Id,
    settings::{self, Change, Command, Kind, Record, Vocabulary},
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsRevision {
    pub committed_at: String,
    pub actor: String,
    pub reason: String,
    pub change: Change,
}

impl Store {
    pub fn settings_history(&self, id: Id) -> Result<Vec<SettingsRevision>> {
        let tx = self.connection.unchecked_transaction()?;
        load_settings(&tx)?;
        let history = {
            let mut statement = tx.prepare("SELECT committed_at,actor,reason,result_json FROM settings_command WHERE target_id=?1 ORDER BY sequence")?;
            let rows = statement.query_map([id.to_string()], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })?;
            rows.map(|row| {
                let (committed_at, actor, reason, json) = row?;
                Ok(SettingsRevision {
                    committed_at,
                    actor,
                    reason,
                    change: serde_json::from_str(&json)?,
                })
            })
            .collect::<Result<Vec<_>>>()?
        };
        tx.commit()?;
        Ok(history)
    }

    pub fn settings(&self) -> Result<BTreeMap<Id, Record>> {
        let tx = self.connection.unchecked_transaction()?;
        let records = load_settings(&tx)?;
        tx.commit()?;
        Ok(records)
    }

    pub fn selectable_settings(&self) -> Result<Vec<Record>> {
        let records = self.settings()?;
        Ok(records
            .values()
            .filter(|r| settings::selectable(&records, r.id))
            .cloned()
            .collect())
    }

    pub fn execute_settings(&mut self, command: Command) -> Result<Change> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let records = load_settings(&tx)?;
        let prior: Option<(String, String)> = tx
            .query_row(
                "SELECT payload_json,result_json FROM settings_command WHERE request_key=?1",
                [&command.request_key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((payload, result)) = prior {
            if serde_json::from_str::<Command>(&payload)? != command {
                return Err("settings request key conflicts with an earlier payload".into());
            }
            let change = serde_json::from_str(&result)?;
            tx.commit()?;
            return Ok(change);
        }
        if crate::sqlite::catalog_key_exists(&tx, &command.request_key)? {
            return Err("request key belongs to a catalog or profile command".into());
        }
        let key_exists: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM inventory_operation WHERE request_key=?1) OR EXISTS(SELECT 1 FROM purchase_command WHERE request_key=?1)", [&command.request_key], |r| r.get(0))?;
        if key_exists {
            return Err("request key already belongs to another command".into());
        }
        let change = settings::prepare(&records, &command)?;
        write_record(&tx, &change.after)?;
        tx.execute("INSERT INTO settings_command(id,request_key,actor,reason,target_id,payload_json,result_json) VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![command.id.to_string(),command.request_key,command.actor,command.reason,change.after.id.to_string(),serde_json::to_string(&command)?,serde_json::to_string(&change)?])?;
        tx.commit()?;
        Ok(change)
    }
}

fn vocabulary_name(v: Vocabulary) -> &'static str {
    match v {
        Vocabulary::Manufacturer => "Manufacturer",
        Vocabulary::Color => "Color",
        Vocabulary::Condition => "Condition",
    }
}

fn seed_records() -> Result<BTreeMap<Id, Record>> {
    [
        (Vocabulary::Manufacturer, "Unknown", true),
        (Vocabulary::Color, "Unknown", true),
        (Vocabulary::Condition, "Unknown", true),
        (Vocabulary::Condition, "New", false),
        (Vocabulary::Condition, "Used", false),
        (Vocabulary::Condition, "Mixed", false),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (vocabulary, name, system_unknown))| {
        let id: Id = serde_json::from_value(serde_json::Value::String(format!(
            "01900000-0000-7000-8000-{:012}",
            index + 1
        )))?;
        Ok((
            id,
            Record {
                id,
                revision: 1,
                name: name.into(),
                archived: false,
                kind: Kind::Vocabulary {
                    vocabulary,
                    system_unknown,
                },
            },
        ))
    })
    .collect()
}

fn write_record(connection: &Connection, r: &Record) -> Result<()> {
    let (vocabulary, unknown, location, parent) = match r.kind {
        Kind::Vocabulary {
            vocabulary,
            system_unknown,
        } => (
            Some(vocabulary_name(vocabulary)),
            system_unknown,
            false,
            None,
        ),
        Kind::Location { parent_id } => (None, false, true, parent_id.map(|id| id.to_string())),
    };
    connection.execute("INSERT INTO settings_record(id,revision,display_name,normalized_name,archived,vocabulary,system_unknown,is_location,parent_id) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,display_name=excluded.display_name,normalized_name=excluded.normalized_name,archived=excluded.archived,parent_id=excluded.parent_id",
        params![r.id.to_string(),i64::try_from(r.revision)?,r.name,settings::normalized(&r.name),r.archived,vocabulary,unknown,location,parent])?;
    Ok(())
}

pub(crate) fn load_settings(connection: &Connection) -> Result<BTreeMap<Id, Record>> {
    if !has_settings(connection)? {
        return Err("settings need schema 3; use a new development database until safe upgrades are implemented".into());
    }
    let mut replay = seed_records()?;
    let mut seen_keys = BTreeSet::new();
    let mut seen_ids = BTreeSet::new();
    let mut audit = connection.prepare("SELECT id,request_key,actor,reason,target_id,payload_json,result_json FROM settings_command ORDER BY sequence")?;
    let mut rows = audit.query([])?;
    while let Some(row) = rows.next()? {
        let command: Command = serde_json::from_str(&row.get::<_, String>(5)?)?;
        let result: Change = serde_json::from_str(&row.get::<_, String>(6)?)?;
        if command.id.to_string() != row.get::<_, String>(0)?
            || command.request_key != row.get::<_, String>(1)?
            || command.actor != row.get::<_, String>(2)?
            || command.reason != row.get::<_, String>(3)?
            || command.action.target().to_string() != row.get::<_, String>(4)?
            || !seen_keys.insert(command.request_key.clone())
            || !seen_ids.insert(command.id)
        {
            return Err("settings audit envelope is inconsistent".into());
        }
        let expected = settings::prepare(&replay, &command)?;
        if result != expected {
            return Err("settings audit result differs from replay".into());
        }
        replay.insert(result.after.id, result.after);
    }
    let mut stored = BTreeMap::new();
    let mut statement = connection.prepare("SELECT id,revision,display_name,normalized_name,archived,vocabulary,system_unknown,is_location,parent_id FROM settings_record")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let id: Id = serde_json::from_value(serde_json::Value::String(row.get(0)?))?;
        let name: String = row.get(2)?;
        if settings::normalized(&name) != row.get::<_, String>(3)? {
            return Err("settings normalization differs from stored name".into());
        }
        let kind = if row.get::<_, bool>(7)? {
            let parent: Option<String> = row.get(8)?;
            Kind::Location {
                parent_id: parent
                    .map(|id| serde_json::from_value(serde_json::Value::String(id)))
                    .transpose()?,
            }
        } else {
            let vocabulary = match row.get::<_, String>(5)?.as_str() {
                "Manufacturer" => Vocabulary::Manufacturer,
                "Color" => Vocabulary::Color,
                "Condition" => Vocabulary::Condition,
                _ => return Err("invalid vocabulary kind".into()),
            };
            Kind::Vocabulary {
                vocabulary,
                system_unknown: row.get(6)?,
            }
        };
        stored.insert(
            id,
            Record {
                id,
                revision: u64::try_from(row.get::<_, i64>(1)?)?,
                name,
                archived: row.get(4)?,
                kind,
            },
        );
    }
    settings::validate_tree(&stored)?;
    if stored != replay {
        return Err("settings differ from their seeds and immutable audit history".into());
    }
    Ok(stored)
}
