//! Project history references both stock and profile prefixes, not supplied balances.
use crate::sqlite::{self, Result, Store};
use brickxinator_core::{
    inventory::Measure,
    ledger::{Id, Ledger},
    profiles,
    projects::{self, Command, State, Stock},
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::collections::BTreeMap;
pub(crate) fn enabled(c: &Connection) -> Result<bool> {
    Ok(c.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migration WHERE version=13)",
        [],
        |r| r.get(0),
    )?)
}
pub(crate) fn key_exists(c: &Connection, key: &str) -> Result<bool> {
    Ok(enabled(c)?
        && c.query_row(
            "SELECT EXISTS(SELECT 1 FROM project_command WHERE request_key=?1)",
            [key],
            |r| r.get(0),
        )?)
}
fn profile_sequence(c: &Connection) -> Result<u64> {
    Ok(c.query_row(
        "SELECT coalesce(max(sequence),0) FROM profile_command",
        [],
        |r| r.get::<_, i64>(0),
    )?
    .try_into()?)
}
fn measured(m: Measure) -> Option<u64> {
    match m {
        Measure::Known(n) => Some(n),
        Measure::Unknown => None,
    }
}
fn stock(c: &Connection, ledger: &Ledger, profile_sequence: u64) -> Result<BTreeMap<Id, Stock>> {
    let settings = crate::settings_store::load_settings(c)?;
    let defaults = crate::catalog_store::default_links(&settings)?;
    let mut profiles = BTreeMap::new();
    let mut q =
        c.prepare("SELECT result_json FROM profile_command WHERE sequence<=?1 ORDER BY sequence")?;
    for row in q.query_map([i64::try_from(profile_sequence)?], |r| {
        r.get::<_, String>(0)
    })? {
        let change: profiles::Change = serde_json::from_str(&row?)?;
        profiles.insert(change.after.target, change.after.links);
    }
    Ok(ledger
        .current_holdings()
        .iter()
        .map(|(id, h)| {
            let links = profiles
                .get(&profiles::Target::Holding(*id))
                .unwrap_or(&defaults);
            (
                *id,
                Stock {
                    revision: h.revision,
                    owned: measured(h.pieces.owned()),
                    damaged: measured(h.pieces.damaged()),
                    definition_id: links.definition_id,
                    color_id: links.color_id,
                    copy_id: ledger.set_contents().get(id).copied(),
                },
            )
        })
        .collect())
}
pub(crate) fn history(c: &Connection, ledger: &Ledger) -> Result<State> {
    if !enabled(c)? {
        return Ok(State::default());
    }
    let mut state = State::default();
    let mut prefix = Ledger::default();
    let mut position = 0usize;
    let mut last_profiles = 0;
    let max_profiles = profile_sequence(c)?;
    let mut q=c.prepare("SELECT sequence,id,request_key,inventory_sequence,profile_sequence,payload_json,result_json FROM project_command ORDER BY sequence")?;
    let mut rows = q.query([])?;
    while let Some(row) = rows.next()? {
        let sequence = u64::try_from(row.get::<_, i64>(0)?)?;
        let id: String = row.get(1)?;
        let key: String = row.get(2)?;
        let inv = usize::try_from(row.get::<_, i64>(3)?)?;
        let profiles = u64::try_from(row.get::<_, i64>(4)?)?;
        if inv < position
            || inv > ledger.operations().len()
            || profiles < last_profiles
            || profiles > max_profiles
        {
            return Err("invalid project source history prefix".into());
        }
        while position < inv {
            prefix.append(ledger.operations()[position].clone())?;
            position += 1;
        }
        let command: Command = serde_json::from_str(&row.get::<_, String>(5)?)?;
        if sequence != state.revision + 1
            || id != command.id.to_string()
            || key != command.request_key
        {
            return Err("project event envelope mismatch".into());
        }
        state = state
            .apply(&command, &stock(c, &prefix, profiles)?)
            .map_err(|e| format!("project replay: {e}"))?;
        if state != serde_json::from_str::<State>(&row.get::<_, String>(6)?)? {
            return Err("project audit result does not match replay".into());
        }
        last_profiles = profiles;
    }
    Ok(state)
}
fn references(c: &Connection, state: &State, ledger: &Ledger) -> Result<()> {
    let settings = crate::settings_store::load_settings(c)?;
    for version in state.versions.values() {
        for r in &version.requirements {
            let valid: bool = c.query_row(
                "SELECT EXISTS(SELECT 1 FROM catalog_definition WHERE id=?1 AND kind='Part')",
                [r.definition_id.to_string()],
                |r| r.get(0),
            )?;
            if !valid {
                return Err("project history references a missing part definition".into());
            }
            brickxinator_core::catalog::require_vocabulary(
                &settings,
                r.color_id,
                brickxinator_core::settings::Vocabulary::Color,
                false,
            )?;
        }
    }
    for m in state.external_mappings.values() {
        let valid: bool = c.query_row(
            "SELECT EXISTS(SELECT 1 FROM catalog_definition WHERE id=?1 AND kind='Part')",
            [m.definition_id.to_string()],
            |r| r.get(0),
        )?;
        if !valid {
            return Err("external mapping references a missing part".into());
        }
        brickxinator_core::catalog::require_vocabulary(
            &settings,
            m.color_id,
            brickxinator_core::settings::Vocabulary::Color,
            false,
        )?;
    }
    for q in state.price_snapshots.values() {
        let valid: bool = c.query_row(
            "SELECT EXISTS(SELECT 1 FROM catalog_definition WHERE id=?1 AND kind='Part')",
            [q.definition_id.to_string()],
            |r| r.get(0),
        )?;
        if !valid {
            return Err("price history references a missing part definition".into());
        }
    }
    if state.projects.values().any(|p| {
        p.copy_id
            .is_some_and(|id| !ledger.set_copies().contains_key(&id))
    }) {
        return Err("project history references a missing physical copy".into());
    }
    if state.projects.values().any(|p| {
        p.copy_id.is_some_and(|id| {
            ledger.lifecycle(id).status != brickxinator_core::physical::Status::Active
        }) && state.claims.values().any(|c| c.project_id == p.id)
    }) {
        return Err("inactive copy has project stock claims; release or transfer those claims explicitly before disposition".into());
    }
    Ok(())
}
pub(crate) fn validate_current(c: &Connection, ledger: &Ledger) -> Result<()> {
    if !enabled(c)? {
        return Ok(());
    }
    let state = history(c, ledger)?;
    references(c, &state, ledger)?;
    state
        .validate(&stock(c, ledger, profile_sequence(c)?)?)
        .map_err(Into::into)
}
impl Store {
    pub fn projects(&self) -> Result<serde_json::Value> {
        let tx = self.connection.unchecked_transaction()?;
        let ledger = sqlite::load(&tx)?;
        let state = history(&tx, &ledger)?;
        let stock = stock(&tx, &ledger, profile_sequence(&tx)?)?;
        state.validate(&stock)?;
        let mut coverage = BTreeMap::new();
        let mut readiness = BTreeMap::new();
        for id in state.projects.keys() {
            coverage.insert(*id, state.coverage(*id, &stock)?);
            readiness.insert(*id, state.readiness(*id, &stock)?);
        }
        let mut available = BTreeMap::new();
        for (id, h) in &stock {
            available.insert(
                *id,
                if h.copy_id.is_some() {
                    Some(0)
                } else {
                    h.owned
                        .zip(h.damaged)
                        .and_then(|(o, d)| o.checked_sub(d))
                        .and_then(|n| n.checked_sub(state.claimed(*id)))
                },
            );
        }
        let procurement = state.procurement(&stock)?;
        let mut audit = Vec::new();
        if enabled(&tx)? {
            let mut q=tx.prepare("SELECT committed_at,payload_json FROM project_command ORDER BY sequence DESC LIMIT 100")?;
            for row in q.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
                let (date, payload) = row?;
                let c: Command = serde_json::from_str(&payload)?;
                audit.push(serde_json::json!({"id":c.id,"date":date,"actor":c.actor,"reason":c.reason,"action":c.action}));
            }
        }

        Ok(
            serde_json::json!({"state":state,"stock":stock,"available":available,"coverage":coverage,"readiness":readiness,"procurement":procurement,"recent_history":audit}),
        )
    }
    pub fn execute_project(&mut self, command: Command) -> Result<State> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if !enabled(&tx)? {
            return Err("project commands require schema 13; upgrade first".into());
        }
        let cached: Option<(String, String)> = tx
            .query_row(
                "SELECT payload_json,result_json FROM project_command WHERE request_key=?1",
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
        if sqlite::metadata_key_exists(&tx,&command.request_key)?||tx.query_row("SELECT EXISTS(SELECT 1 FROM inventory_operation WHERE request_key=?1) OR EXISTS(SELECT 1 FROM purchase_command WHERE request_key=?1)",[&command.request_key],|r|r.get::<_,bool>(0))? {return Err("request key already belongs to another command".into());}
        let ledger = sqlite::load(&tx)?;
        let before = history(&tx, &ledger)?;
        let profiles = profile_sequence(&tx)?;
        if let projects::Action::Create(p) = &command.action
            && p.copy_id
                .is_some_and(|id| !ledger.set_copies().contains_key(&id))
        {
            return Err("set copy does not exist".into());
        }
        if let projects::Action::ConfirmExternalMapping(m) = &command.action {
            let valid:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM catalog_definition WHERE id=?1 AND kind='Part' AND archived=0)",[m.definition_id.to_string()],|r|r.get(0))?;
            if !valid {
                return Err("mapped part definition does not exist or is archived".into());
            }
            brickxinator_core::catalog::require_vocabulary(
                &crate::settings_store::load_settings(&tx)?,
                m.color_id,
                brickxinator_core::settings::Vocabulary::Color,
                true,
            )?;
        }
        if let projects::Action::AddPriceSnapshot(q) = &command.action {
            let valid: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM catalog_definition WHERE id=?1 AND kind='Part' AND archived=0)",
                [q.definition_id.to_string()],
                |r| r.get(0),
            )?;
            if !valid {
                return Err("price snapshot part definition does not exist".into());
            }
        }
        if let projects::Action::AddVersion(v) = &command.action {
            for r in &v.requirements {
                let valid: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM catalog_definition WHERE id=?1 AND kind='Part' AND archived=0)",
                    [r.definition_id.to_string()],
                    |r| r.get(0),
                )?;
                if !valid {
                    return Err("requirement part definition does not exist".into());
                }
                brickxinator_core::catalog::require_vocabulary(
                    &crate::settings_store::load_settings(&tx)?,
                    r.color_id,
                    brickxinator_core::settings::Vocabulary::Color,
                    true,
                )?;
            }
        }
        let after = before.apply(&command, &stock(&tx, &ledger, profiles)?)?;
        references(&tx, &after, &ledger)?;
        tx.execute("INSERT INTO project_command(sequence,id,request_key,inventory_sequence,profile_sequence,payload_json,result_json) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![i64::try_from(after.revision)?,command.id.to_string(),command.request_key,i64::try_from(ledger.operations().len())?,i64::try_from(profiles)?,serde_json::to_string(&command)?,serde_json::to_string(&after)?])?;
        tx.commit()?;
        Ok(after)
    }
}
