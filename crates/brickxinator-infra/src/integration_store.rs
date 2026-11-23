//! Imported files are cached locally; only explicit reviewed catalog fields change.
use crate::sqlite::{self, Result, Store};
use brickxinator_core::{
    catalog::{self, Definition},
    integrations::{self, Columns},
    ledger::Id,
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Format {
    LDraw,
    StudioParts,
    RebrickableInventory,
    RebrickableParts,
    RebrickableSets,
    RebrickableColors,
    RebrickableThemes,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub name: String,
    pub format: Format,
    pub text: String,
    pub delimiter: char,
    pub columns: Option<Columns>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mapping {
    pub row: usize,
    pub target_id: Id,
    pub expected_revision: u64,
    pub manufacturer_id: Id,
    pub fields: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme_id: Option<Id>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Command {
    pub id: Id,
    pub request_key: String,
    pub actor: String,
    pub reason: String,
    pub source: Source,
    pub mappings: Vec<Mapping>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cached {
    pub source_sha256: String,
    pub changes: Vec<catalog::Command>,
}
fn enabled(c: &Connection) -> Result<bool> {
    Ok(c.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migration WHERE version=17)",
        [],
        |r| r.get(0),
    )?)
}
pub(crate) fn key_exists(c: &Connection, key: &str) -> Result<bool> {
    Ok(enabled(c)?
        && c.query_row(
            "SELECT EXISTS(SELECT 1 FROM integration_import WHERE request_key=?1)",
            [key],
            |r| r.get(0),
        )?)
}
fn digest(source: &str) -> String {
    format!("{:x}", Sha256::digest(source.as_bytes()))
}
pub fn preview(source: &Source) -> Result<serde_json::Value> {
    if source.name.trim().is_empty() || source.name.len() > 255 {
        return Err("source file name is required and limited to 255 characters".into());
    }
    let hash = digest(&source.text);
    if matches!(
        source.format,
        Format::RebrickableColors | Format::RebrickableThemes
    ) {
        return Ok(
            serde_json::json!({"sha256":hash,"enrichment":crate::enrichment_store::parse(source)?}),
        );
    }
    if source.format == Format::LDraw {
        return Ok(
            serde_json::json!({"sha256":hash,"parts":brickxinator_core::model_import::ldraw(&source.text)?}),
        );
    }
    let table = integrations::delimited(&source.text, source.delimiter)?;
    match source.format {
        Format::StudioParts | Format::RebrickableInventory => {
            let parts = source
                .columns
                .as_ref()
                .map(|columns| integrations::bom(&table, columns))
                .transpose()?;
            Ok(serde_json::json!({"sha256":hash,"table":table,"parts":parts}))
        }
        Format::RebrickableParts | Format::RebrickableSets => {
            let number = if source.format == Format::RebrickableParts {
                "part_num"
            } else {
                "set_num"
            };
            let index = |name: &str| {
                table
                    .headers
                    .iter()
                    .position(|h| h == name)
                    .ok_or_else(|| format!("missing Rebrickable column {name}"))
            };
            let number = index(number)?;
            let name = index("name")?;
            let year = table.headers.iter().position(|h| h == "year");
            let count = table.headers.iter().position(|h| h == "num_parts");
            let rows=table.rows.iter().enumerate().map(|(i,r)|{if r[number].trim().is_empty()||r[name].trim().is_empty(){return Err("catalog rows require identifiers and names".to_string());}let year=year.map(|i|r[i].parse::<u16>().map_err(|_|"invalid release year")).transpose()?;let count=count.map(|i|r[i].parse::<u64>().map_err(|_|"invalid stated piece count")).transpose()?;Ok(serde_json::json!({"row":i,"number":r[number],"name":r[name],"release_year":year,"stated_piece_count":count}))}).collect::<std::result::Result<Vec<_>,String>>()?;
            Ok(serde_json::json!({"sha256":hash,"catalog":rows}))
        }
        _ => unreachable!(),
    }
}
fn prepare(command: &Command, definitions: &BTreeMap<Id, Definition>) -> Result<Cached> {
    if command.actor.trim().is_empty()
        || command.reason.trim().is_empty()
        || command.request_key.trim().is_empty()
    {
        return Err("import audit context is required".into());
    }
    let reviewed = preview(&command.source)?;
    let mut changes = vec![];
    let mut rows = BTreeSet::new();
    let mut targets = BTreeSet::new();
    if command.mappings.len() > 10000 {
        return Err("import at most 10000 catalog changes in one reviewed batch".into());
    }
    for mapping in &command.mappings {
        if !rows.insert(mapping.row) || !targets.insert(mapping.target_id) {
            return Err("duplicate catalog row or target in import".into());
        }
        let row = reviewed["catalog"]
            .as_array()
            .and_then(|a| a.get(mapping.row))
            .ok_or("catalog mapping row does not exist")?;
        let kind = if command.source.format == Format::RebrickableParts {
            catalog::Kind::Part
        } else {
            catalog::Kind::Set
        };
        let old = definitions.get(&mapping.target_id);
        if old
            .is_some_and(|d| d.kind != kind || d.details.manufacturer_id != mapping.manufacturer_id)
        {
            return Err(
                "catalog import cannot silently change kind or manufacturer identity".into(),
            );
        }
        if old.map_or(0, |d| d.revision) != mapping.expected_revision {
            return Err("local catalog changed since import review".into());
        }
        let mut details = old.map(|d| d.details.clone()).unwrap_or(catalog::Details {
            expected_contents: vec![],
            manufacturer_id: mapping.manufacturer_id,
            number: None,
            name: None,
            description: None,
            release_year: None,
            theme: None,
            theme_id: None,
            stated_piece_count: None,
        });
        for field in &mapping.fields {
            match field.as_str() {
                "number" => details.number = row["number"].as_str().map(str::to_owned),
                "name" => details.name = row["name"].as_str().map(str::to_owned),
                "theme" => details.theme_id = mapping.theme_id,
                "release_year" => {
                    details.release_year = row["release_year"].as_u64().map(|n| n as u16)
                }
                "stated_piece_count" => {
                    details.stated_piece_count = row["stated_piece_count"].as_u64()
                }
                _ => return Err("unsupported enrichment field".into()),
            }
        }
        let action = if old.is_some() {
            catalog::Action::Edit {
                id: mapping.target_id,
                expected_revision: mapping.expected_revision,
                details,
            }
        } else {
            catalog::Action::Create {
                id: mapping.target_id,
                kind,
                details,
            }
        };
        changes.push(catalog::Command {
            id: mapping.target_id, /* replaced for existing targets below */
            request_key: format!("import:{}:{}", command.id, mapping.row),
            actor: command.actor.clone(),
            reason: format!("{} · {}", command.source.name, command.reason),
            action,
        });
    }
    Ok(Cached {
        source_sha256: digest(&command.source.text),
        changes,
    })
}
pub(crate) fn validate(c: &Connection) -> Result<()> {
    if !enabled(c)? {
        return Ok(());
    }
    let mut q=c.prepare("SELECT id,request_key,source_sha256,payload_json,result_json FROM integration_import ORDER BY sequence")?;
    for row in q.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
        ))
    })? {
        let (id, key, hash, payload, result) = row?;
        let command: Command = serde_json::from_str(&payload)?;
        let result: Cached = serde_json::from_str(&result)?;
        if id != command.id.to_string()
            || key != command.request_key
            || hash != digest(&command.source.text)
            || hash != result.source_sha256
        {
            return Err("cached import envelope or source hash mismatch".into());
        }
        preview(&command.source)?;
        let mut definitions = BTreeMap::new();
        for change in &result.changes {
            let json: String = c.query_row(
                "SELECT result_json FROM catalog_command WHERE request_key=?1",
                [&change.request_key],
                |r| r.get(0),
            )?;
            let change: catalog::Change = serde_json::from_str(&json)?;
            if let Some(before) = change.before {
                definitions.insert(before.id, before);
            }
        }
        let expected = prepare(&command, &definitions)?;
        if expected.changes.len() != result.changes.len()
            || expected.changes.iter().zip(&result.changes).any(|(a, b)| {
                a.action != b.action
                    || a.request_key != b.request_key
                    || a.actor != b.actor
                    || a.reason != b.reason
            })
        {
            return Err("cached import effects differ from reviewed source mappings".into());
        }
        for change in result.changes {
            let stored: Option<String> = c
                .query_row(
                    "SELECT payload_json FROM catalog_command WHERE request_key=?1",
                    [&change.request_key],
                    |r| r.get(0),
                )
                .optional()?;
            if stored
                .map(|s| serde_json::from_str::<catalog::Command>(&s))
                .transpose()?
                .as_ref()
                != Some(&change)
            {
                return Err("cached import catalog result lacks canonical audit history".into());
            }
        }
    }
    Ok(())
}
impl Store {
    pub fn preview_integration(&self, command: Command) -> Result<Cached> {
        let definitions = self.catalog()?;
        let mut result = prepare(&command, &definitions)?;
        let settings = self.settings()?;
        for change in &mut result.changes {
            change.id = Id::new();
            let preview = catalog::prepare(&definitions, &settings, change, false)?;
            crate::catalog_store::validate_theme(&self.connection, &preview.after)?;
        }
        Ok(result)
    }
    pub fn integrations(&self) -> Result<serde_json::Value> {
        let mut result = vec![];
        let mut q=self.connection.prepare("SELECT committed_at,payload_json,result_json FROM integration_import ORDER BY sequence DESC")?;
        for row in q.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })? {
            let (at, command, cached) = row?;
            let command: Command = serde_json::from_str(&command)?;
            let cached: Cached = serde_json::from_str(&cached)?;
            result.push(serde_json::json!({"at":at,"command":command,"result":cached}));
        }
        Ok(serde_json::json!(result))
    }
    pub fn execute_integration(&mut self, command: Command) -> Result<Cached> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let cached: Option<(String, String)> = tx
            .query_row(
                "SELECT payload_json,result_json FROM integration_import WHERE request_key=?1",
                [&command.request_key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((payload, result)) = cached {
            if serde_json::from_str::<Command>(&payload)? != command {
                return Err("import request key conflict".into());
            }
            return Ok(serde_json::from_str(&result)?);
        }
        if sqlite::metadata_key_exists(&tx,&command.request_key)?||tx.query_row("SELECT EXISTS(SELECT 1 FROM inventory_operation WHERE request_key=?1) OR EXISTS(SELECT 1 FROM purchase_command WHERE request_key=?1)",[&command.request_key],|r|r.get::<_,bool>(0))?{return Err("request key belongs to another command".into());}
        let settings = crate::settings_store::load_settings(&tx)?;
        let mut definitions = crate::catalog_store::load_catalog(&tx, &settings)?;
        let mut result = prepare(&command, &definitions)?;
        for entry in &mut result.changes {
            if entry.request_key==command.request_key||sqlite::metadata_key_exists(&tx,&entry.request_key)?||tx.query_row("SELECT EXISTS(SELECT 1 FROM inventory_operation WHERE request_key=?1) OR EXISTS(SELECT 1 FROM purchase_command WHERE request_key=?1)",[&entry.request_key],|r|r.get::<_,bool>(0))?{return Err("import row request key conflicts with another command".into());}
            entry.id = Id::new();
            let change = catalog::prepare(&definitions, &settings, entry, false)?;
            crate::catalog_store::validate_theme(&tx, &change.after)?;
            crate::catalog_store::write_definition(&tx, &settings, &change.after)?;
            tx.execute("INSERT INTO catalog_command(id,request_key,actor,reason,target_id,payload_json,result_json) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![entry.id.to_string(),entry.request_key,entry.actor,entry.reason,change.after.id.to_string(),serde_json::to_string(entry)?,serde_json::to_string(&change)?])?;
            definitions.insert(change.after.id, change.after);
        }
        tx.execute("INSERT INTO integration_import(id,request_key,source_sha256,payload_json,result_json) VALUES(?1,?2,?3,?4,?5)",params![command.id.to_string(),command.request_key,result.source_sha256,serde_json::to_string(&command)?,serde_json::to_string(&result)?])?;
        tx.commit()?;
        Ok(result)
    }
}
