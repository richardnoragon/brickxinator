//! Reviewed color/theme enrichment with local UUIDs, source caches and local overrides.
use crate::{
    integration_store::{Format, Source},
    sqlite::{self, Result, Store},
};
use brickxinator_core::{
    ledger::Id,
    settings::{self, Record as Setting},
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Color,
    Theme,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Row {
    pub row: usize,
    pub external_id: String,
    pub name: String,
    pub rgb: Option<String>,
    pub is_trans: Option<bool>,
    pub parent_external_id: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub id: Id,
    pub revision: u64,
    pub kind: Kind,
    pub external_id: String,
    pub name: String,
    pub rgb: Option<String>,
    pub is_trans: Option<bool>,
    pub parent_id: Option<Id>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mapping {
    pub row: usize,
    pub target_id: Id,
    pub expected_revision: u64,
    pub expected_settings_revision: u64,
    pub fields: Vec<String>,
    pub local_name: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Command {
    pub id: Id,
    pub request_key: String,
    pub actor: String,
    pub reason: String,
    pub source: Source,
    pub expected_settings_sequence: i64,
    pub accept_name_warnings: bool,
    pub mappings: Vec<Mapping>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    pub records: Vec<Record>,
    pub settings_changes: Vec<settings::Command>,
}
fn hash(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}
fn enabled(c: &Connection) -> Result<bool> {
    Ok(c.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migration WHERE version=18)",
        [],
        |r| r.get(0),
    )?)
}
pub(crate) fn key_exists(c: &Connection, key: &str) -> Result<bool> {
    Ok(enabled(c)?
        && c.query_row(
            "SELECT EXISTS(SELECT 1 FROM enrichment_command WHERE request_key=?1)",
            [key],
            |r| r.get(0),
        )?)
}
fn sequence(c: &Connection) -> Result<i64> {
    Ok(c.query_row(
        "SELECT coalesce(max(sequence),0) FROM settings_command",
        [],
        |r| r.get(0),
    )?)
}
pub fn parse(source: &Source) -> Result<Vec<Row>> {
    if source.name.trim().is_empty() || source.name.len() > 255 {
        return Err("source file name is required and limited to 255 characters".into());
    }
    if !matches!(
        source.format,
        Format::RebrickableColors | Format::RebrickableThemes
    ) {
        return Err("choose a supported color/theme CSV format".into());
    }
    let table = brickxinator_core::integrations::delimited(&source.text, source.delimiter)?;
    let index = |name: &str| {
        table
            .headers
            .iter()
            .position(|h| h == name)
            .ok_or_else(|| format!("missing enrichment column {name}"))
    };
    let id = index("id")?;
    let name = index("name")?;
    let color = source.format == Format::RebrickableColors;
    let rgb = if color { Some(index("rgb")?) } else { None };
    let trans = if color {
        Some(index("is_trans")?)
    } else {
        None
    };
    let parent = if color {
        None
    } else {
        Some(index("parent_id")?)
    };
    let mut rows = vec![];
    let mut seen = BTreeSet::new();
    for (i, r) in table.rows.iter().enumerate() {
        let external_id = r[id]
            .parse::<i64>()
            .map_err(|_| "provider ID must be an integer")?
            .to_string();
        if !seen.insert(external_id.clone()) || r[name].trim().is_empty() {
            return Err("duplicate provider ID or empty name in enrichment file".into());
        }
        let rgb = rgb.and_then(|i| (!r[i].trim().is_empty()).then(|| r[i].trim().to_uppercase()));
        if rgb
            .as_ref()
            .is_some_and(|s| s.len() != 6 || !s.bytes().all(|c| c.is_ascii_hexdigit()))
        {
            return Err("RGB must be six hexadecimal digits, or blank for unknown".into());
        }
        let is_trans = trans
            .map(|i| match r[i].trim().to_lowercase().as_str() {
                "t" | "true" | "1" => Ok(Some(true)),
                "f" | "false" | "0" => Ok(Some(false)),
                "" => Ok(None),
                _ => Err("invalid transparency flag"),
            })
            .transpose()?
            .flatten();
        let parent_external_id = parent
            .map(|i| -> std::result::Result<Option<String>, &str> {
                if r[i].trim().is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(
                        r[i].parse::<i64>()
                            .map_err(|_| "invalid parent theme ID")?
                            .to_string(),
                    ))
                }
            })
            .transpose()?
            .flatten();
        rows.push(Row {
            row: i,
            external_id,
            name: r[name].clone(),
            rgb,
            is_trans,
            parent_external_id,
        });
    }
    Ok(rows)
}
fn validate_graph(records: &BTreeMap<Id, Record>) -> Result<()> {
    let mut identities = BTreeSet::new();
    for record in records.values() {
        if !identities.insert((format!("{:?}", record.kind), record.external_id.clone())) {
            return Err("provider identifier is already mapped to another local identity".into());
        }
        let mut seen = BTreeSet::from([record.id]);
        let mut parent = record.parent_id;
        while let Some(id) = parent {
            if !seen.insert(id) {
                return Err("theme hierarchy contains a cycle".into());
            }
            let p = records
                .get(&id)
                .filter(|p| p.kind == Kind::Theme)
                .ok_or("parent theme has not been imported/mapped")?;
            parent = p.parent_id;
        }
    }
    Ok(())
}
fn prepare(
    command: &Command,
    before: &BTreeMap<Id, Record>,
    settings: &BTreeMap<Id, Setting>,
) -> Result<Outcome> {
    if command.actor.trim().is_empty()
        || command.reason.trim().is_empty()
        || command.request_key.trim().is_empty()
        || command.mappings.is_empty()
        || command.mappings.len() > 10000
    {
        return Err("choose a bounded set of enrichment rows and supply audit context".into());
    }
    let source = parse(&command.source)?;
    let kind = if command.source.format == Format::RebrickableColors {
        Kind::Color
    } else {
        Kind::Theme
    };
    let mut next = before.clone();
    let mut current_settings = settings.clone();
    let mut rows = BTreeSet::new();
    let mut targets = BTreeSet::new();
    let mut result = Outcome {
        records: vec![],
        settings_changes: vec![],
    };
    let mut external: BTreeMap<String, Id> = before
        .values()
        .filter(|r| r.kind == Kind::Theme)
        .map(|r| (r.external_id.clone(), r.id))
        .collect();
    if kind == Kind::Theme {
        for m in &command.mappings {
            let row = source.get(m.row).ok_or("source row does not exist")?;
            external.insert(row.external_id.clone(), m.target_id);
        }
    }
    for mapping in &command.mappings {
        if !rows.insert(mapping.row) || !targets.insert(mapping.target_id) {
            return Err("duplicate source row or enrichment target".into());
        }
        let row = source.get(mapping.row).ok_or("source row does not exist")?;
        let old = before.get(&mapping.target_id);
        if old.is_some_and(|r| r.kind != kind)
            || old.map_or(0, |r| r.revision) != mapping.expected_revision
        {
            return Err("enrichment identity or revision changed since review".into());
        }
        let mut record = old.cloned().unwrap_or(Record {
            id: mapping.target_id,
            revision: 0,
            kind,
            external_id: row.external_id.clone(),
            name: row.name.clone(),
            rgb: None,
            is_trans: None,
            parent_id: None,
        });
        if kind == Kind::Color {
            let existing = current_settings.get(&mapping.target_id);
            if existing.is_some_and(|r| {
                !matches!(
                    r.kind,
                    settings::Kind::Vocabulary {
                        vocabulary: settings::Vocabulary::Color,
                        system_unknown: false
                    }
                ) || r.archived
            }) || existing.map_or(0, |r| r.revision) != mapping.expected_settings_revision
            {
                return Err("color setting is protected, archived or changed since review".into());
            }
            if let Some(existing) = existing {
                record.name = existing.name.clone();
            }
        }
        for field in &mapping.fields {
            match field.as_str() {
                "name" => {
                    record.name = mapping
                        .local_name
                        .as_ref()
                        .filter(|s| !s.trim().is_empty())
                        .unwrap_or(&row.name)
                        .clone()
                }
                "external_id" => record.external_id = row.external_id.clone(),
                "rgb" if kind == Kind::Color => record.rgb = row.rgb.clone(),
                "is_trans" if kind == Kind::Color => record.is_trans = row.is_trans,
                "parent" if kind == Kind::Theme => {
                    record.parent_id = row
                        .parent_external_id
                        .as_ref()
                        .map(|id| {
                            external
                                .get(id)
                                .copied()
                                .ok_or("import or explicitly map the parent theme first")
                        })
                        .transpose()?
                }
                _ => return Err("unsupported field for this enrichment kind".into()),
            }
        }
        if record.name.trim().is_empty() {
            return Err("local display name is required".into());
        }
        record.revision = record
            .revision
            .checked_add(1)
            .ok_or("enrichment revision overflow")?;
        if kind == Kind::Color {
            let existing = current_settings.get(&record.id);
            let action = if let Some(existing) = existing {
                (existing.name != record.name).then(|| settings::Action::Rename {
                    id: record.id,
                    expected_revision: existing.revision,
                    name: record.name.clone(),
                })
            } else {
                Some(settings::Action::CreateVocabulary {
                    id: record.id,
                    vocabulary: settings::Vocabulary::Color,
                    name: record.name.clone(),
                })
            };
            if let Some(action) = action {
                let cmd = settings::Command {
                    id: Id::new(),
                    request_key: format!("enrichment:{}:{}", command.id, mapping.row),
                    actor: command.actor.clone(),
                    reason: command.reason.clone(),
                    accept_name_warnings: command.accept_name_warnings,
                    action,
                };
                let change = settings::prepare(&current_settings, &cmd)?;
                current_settings.insert(change.after.id, change.after);
                result.settings_changes.push(cmd);
            }
        }
        next.insert(record.id, record.clone());
        result.records.push(record);
    }
    validate_graph(&next)?;
    Ok(result)
}
fn settings_at(c: &Connection, seq: i64) -> Result<BTreeMap<Id, Setting>> {
    let mut records = crate::settings_store::seed_records()?;
    let mut q =
        c.prepare("SELECT result_json FROM settings_command WHERE sequence<=?1 ORDER BY sequence")?;
    for row in q.query_map([seq], |r| r.get::<_, String>(0))? {
        let change: settings::Change = serde_json::from_str(&row?)?;
        records.insert(change.after.id, change.after);
    }
    Ok(records)
}
pub(crate) fn load(c: &Connection) -> Result<BTreeMap<Id, Record>> {
    let mut records = BTreeMap::new();
    if !enabled(c)? {
        return Ok(records);
    }
    crate::settings_store::load_settings(c)?;
    let max = sequence(c)?;
    let mut last = 0;
    let mut q=c.prepare("SELECT id,request_key,settings_sequence,source_sha256,payload_json,result_json FROM enrichment_command ORDER BY sequence")?;
    for row in q.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, String>(5)?,
        ))
    })? {
        let (id, key, seq, sha, payload, result) = row?;
        let cmd: Command = serde_json::from_str(&payload)?;
        let result: Outcome = serde_json::from_str(&result)?;
        if id != cmd.id.to_string()
            || key != cmd.request_key
            || seq != cmd.expected_settings_sequence
            || seq < last
            || seq > max
            || sha != hash(&cmd.source.text)
        {
            return Err("enrichment envelope, source hash or settings prefix is invalid".into());
        }
        let expected = prepare(&cmd, &records, &settings_at(c, seq)?)?;
        if expected.records != result.records
            || expected.settings_changes.len() != result.settings_changes.len()
        {
            return Err("enrichment result differs from reviewed source replay".into());
        }
        for (i, (expected, actual)) in expected
            .settings_changes
            .iter()
            .zip(&result.settings_changes)
            .enumerate()
        {
            if expected.action != actual.action
                || expected.request_key != actual.request_key
                || expected.actor != actual.actor
                || expected.reason != actual.reason
                || expected.accept_name_warnings != actual.accept_name_warnings
            {
                return Err("enrichment settings effects differ from review".into());
            }
            let (sequence, payload): (i64, String) = c.query_row(
                "SELECT sequence,payload_json FROM settings_command WHERE request_key=?1",
                [&actual.request_key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            if sequence != seq + i as i64 + 1
                || serde_json::from_str::<settings::Command>(&payload)? != *actual
            {
                return Err("enrichment settings change is not canonical history".into());
            }
        }
        last = seq + result.settings_changes.len() as i64;
        for record in result.records {
            records.insert(record.id, record);
        }
    }
    validate_graph(&records)?;
    Ok(records)
}
impl Store {
    pub fn enrichments(&self) -> Result<serde_json::Value> {
        let tx = self.connection.unchecked_transaction()?;
        let records = load(&tx)?;
        let mut history = vec![];
        if enabled(&tx)? {
            let mut q=tx.prepare("SELECT committed_at,payload_json,result_json FROM enrichment_command ORDER BY sequence DESC LIMIT 100")?;
            for row in q.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })? {
                let (at, c, r) = row?;
                history.push(serde_json::json!({"at":at,"command":serde_json::from_str::<Command>(&c)?,"result":serde_json::from_str::<Outcome>(&r)?}));
            }
        }
        Ok(
            serde_json::json!({"records":records,"settings_sequence":sequence(&tx)?,"history":history}),
        )
    }
    pub fn preview_enrichment(&self, command: Command) -> Result<Outcome> {
        let tx = self.connection.unchecked_transaction()?;
        if !enabled(&tx)? || sequence(&tx)? != command.expected_settings_sequence {
            return Err("settings changed since enrichment review; refresh first".into());
        }
        prepare(
            &command,
            &load(&tx)?,
            &crate::settings_store::load_settings(&tx)?,
        )
    }
    pub fn execute_enrichment(&mut self, command: Command) -> Result<Outcome> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if !enabled(&tx)? {
            return Err("enrichment requires schema 18".into());
        }
        let cached: Option<(String, String)> = tx
            .query_row(
                "SELECT payload_json,result_json FROM enrichment_command WHERE request_key=?1",
                [&command.request_key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((payload, result)) = cached {
            if serde_json::from_str::<Command>(&payload)? != command {
                return Err("enrichment request key conflict".into());
            }
            return Ok(serde_json::from_str(&result)?);
        }
        if sqlite::metadata_key_exists(&tx,&command.request_key)?||tx.query_row("SELECT EXISTS(SELECT 1 FROM inventory_operation WHERE request_key=?1) OR EXISTS(SELECT 1 FROM purchase_command WHERE request_key=?1)",[&command.request_key],|r|r.get::<_,bool>(0))?{return Err("enrichment request key belongs to another command".into());}
        if sequence(&tx)? != command.expected_settings_sequence {
            return Err("settings changed since enrichment review".into());
        }
        let mut settings = crate::settings_store::load_settings(&tx)?;
        let result = prepare(&command, &load(&tx)?, &settings)?;
        for entry in &result.settings_changes {
            if entry.request_key==command.request_key||sqlite::metadata_key_exists(&tx,&entry.request_key)?||tx.query_row("SELECT EXISTS(SELECT 1 FROM inventory_operation WHERE request_key=?1) OR EXISTS(SELECT 1 FROM purchase_command WHERE request_key=?1)",[&entry.request_key],|r|r.get::<_,bool>(0))?{return Err("enrichment row request key conflict".into());}
            let change = settings::prepare(&settings, entry)?;
            crate::settings_store::write_record(&tx, &change.after)?;
            tx.execute("INSERT INTO settings_command(id,request_key,actor,reason,target_id,payload_json,result_json) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![entry.id.to_string(),entry.request_key,entry.actor,entry.reason,change.after.id.to_string(),serde_json::to_string(entry)?,serde_json::to_string(&change)?])?;
            settings.insert(change.after.id, change.after);
        }
        tx.execute("INSERT INTO enrichment_command(id,request_key,settings_sequence,source_sha256,payload_json,result_json) VALUES(?1,?2,?3,?4,?5,?6)",params![command.id.to_string(),command.request_key,command.expected_settings_sequence,hash(&command.source.text),serde_json::to_string(&command)?,serde_json::to_string(&result)?])?;
        tx.commit()?;
        Ok(result)
    }
}
