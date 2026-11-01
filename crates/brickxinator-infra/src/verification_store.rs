//! Set verification is journal-backed; relational rows constrain identity only.
use crate::{
    catalog_store::{load_catalog, load_profiles},
    settings_store::load_settings,
    sqlite::{Result, Store, has_verification, load, metadata_key_exists, persist_operation},
};
use brickxinator_core::{
    catalog,
    inventory::AssemblyState,
    journal::Holding,
    ledger::{Body, Id, Ledger, Operation, Outcome},
    profiles::{Links, Target},
    settings::Vocabulary,
    verification::{Action, Command, State},
};
use rusqlite::{Connection, TransactionBehavior, params};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Content {
    pub line_id: Id,
    pub holding_id: Id,
    pub holding: Holding,
    pub links: Links,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision {
    pub committed_at: String,
    pub actor: String,
    pub reason: String,
    pub before: State,
    pub after: State,
}

impl Store {
    pub fn prepare_definition_expected(
        &self,
        definition_id: Id,
    ) -> Result<Vec<brickxinator_core::verification::ExpectedLine>> {
        let definitions = self.catalog()?;
        let definition = definitions
            .get(&definition_id)
            .ok_or("set definition does not exist")?;
        if definition.kind != catalog::Kind::Set || definition.archived {
            return Err("choose an active set definition".into());
        }
        Ok(definition
            .details
            .expected_contents
            .iter()
            .map(|line| brickxinator_core::verification::ExpectedLine {
                line_id: Id::new(),
                holding_id: Id::new(),
                definition_id: line.definition_id,
                color_id: line.color_id,
                expected: line.expected,
            })
            .collect())
    }
    pub fn verification_history(&self, copy_id: Id) -> Result<Vec<Revision>> {
        let tx = self.connection.unchecked_transaction()?;
        require_schema(&tx)?;
        let current = load(&tx)?;
        if !current.set_copies().contains_key(&copy_id) {
            return Err("set copy does not exist".into());
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
                let before = if matches!(op.body,Body::Verify {copy_id:id,..} if id==copy_id) {
                    Some(
                        replay
                            .verification()
                            .get(&copy_id)
                            .cloned()
                            .unwrap_or_else(|| {
                                State::new(copy_id, replay.set_copies()[&copy_id].assembly)
                            }),
                    )
                } else {
                    None
                };
                let actor = op.actor.clone();
                let reason = op.reason.clone();
                replay.append(op)?;
                if let Some(before) = before {
                    history.push(Revision {
                        committed_at: row.get(0)?,
                        actor,
                        reason,
                        before,
                        after: replay.verification()[&copy_id].clone(),
                    });
                }
            }
        }
        tx.commit()?;
        Ok(history)
    }

    pub fn execute_verification(&mut self, command: Command) -> Result<Outcome> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        require_schema(&tx)?;
        let mut ledger = load(&tx)?;
        let reason = if command.reason.trim().is_empty()
            && !matches!(
                command.action,
                Action::Count { .. } | Action::ReconcileCount { .. }
            ) {
            "Set verification lifecycle".to_string()
        } else {
            command.reason
        };
        let operation = Operation {
            id: command.id,
            request_key: command.request_key,
            actor: command.actor,
            reason,
            body: Body::Verify {
                copy_id: command.copy_id,
                expected_revision: command.expected_revision,
                action: command.action,
            },
        };
        let retry = ledger
            .operations()
            .iter()
            .any(|op| op.request_key == operation.request_key);
        if !retry {
            if metadata_key_exists(&tx, &operation.request_key)?
                || tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM purchase_command WHERE request_key=?1)",
                    [&operation.request_key],
                    |r| r.get::<_, bool>(0),
                )?
            {
                return Err("request key already belongs to another command".into());
            }
            if let Body::Verify { action, .. } = &operation.body {
                let fields = match action {
                    Action::AddLine {
                        definition_id,
                        color_id,
                        ..
                    } => vec![(*definition_id, *color_id, true, true)],
                    Action::CopyExpected { lines, .. } | Action::AddExpected { lines } => lines
                        .iter()
                        .map(|line| (line.definition_id, line.color_id, true, true))
                        .collect(),
                    Action::Identity {
                        line_id,
                        definition_id,
                        color_id,
                    } => {
                        let supported: bool = tx.query_row(
                            "SELECT EXISTS(SELECT 1 FROM schema_migration WHERE version=12)",
                            [],
                            |r| r.get(0),
                        )?;
                        if !supported {
                            return Err("checklist identity corrections require schema 12".into());
                        }
                        let old = ledger
                            .verification()
                            .get(&command.copy_id)
                            .and_then(|state| state.lines.get(line_id))
                            .ok_or("checklist line does not exist")?;
                        vec![(
                            *definition_id,
                            *color_id,
                            *definition_id != old.definition_id,
                            *color_id != old.color_id,
                        )]
                    }
                    _ => vec![],
                };
                let settings = load_settings(&tx)?;
                let definitions = load_catalog(&tx, &settings)?;
                for (definition_id, color_id, new_definition, new_color) in fields {
                    catalog::require_vocabulary(&settings, color_id, Vocabulary::Color, new_color)?;
                    if let Some(id) = definition_id {
                        let definition = definitions
                            .get(&id)
                            .ok_or("checklist part definition does not exist")?;
                        if definition.kind != catalog::Kind::Part
                            || new_definition && definition.archived
                        {
                            return Err("checklist needs an active part definition".into());
                        }
                    }
                }
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

    /// Generates a preview whose identities must be retained for stable retries.
    /// Execute CopyExpected afterward; changed source expectations conflict.
    pub fn prepare_expected_copy(
        &self,
        source_copy_id: Id,
    ) -> Result<Vec<brickxinator_core::verification::ExpectedLine>> {
        Ok(self
            .set_verification(source_copy_id)?
            .lines
            .values()
            .filter(|line| !line.retired)
            .map(|line| brickxinator_core::verification::ExpectedLine {
                line_id: Id::new(),
                holding_id: Id::new(),
                definition_id: line.definition_id,
                color_id: line.color_id,
                expected: line.expected,
            })
            .collect())
    }

    pub fn set_verification(&self, copy_id: Id) -> Result<State> {
        let tx = self.connection.unchecked_transaction()?;
        require_schema(&tx)?;
        let ledger = load(&tx)?;
        let copy = ledger
            .set_copies()
            .get(&copy_id)
            .ok_or("set copy does not exist")?;
        let state = ledger
            .verification()
            .get(&copy_id)
            .cloned()
            .unwrap_or_else(|| State::new(copy_id, copy.assembly));
        tx.commit()?;
        Ok(state)
    }

    pub fn set_contents(&self, copy_id: Id) -> Result<Vec<Content>> {
        let tx = self.connection.unchecked_transaction()?;
        require_schema(&tx)?;
        let ledger = load(&tx)?;
        if !ledger.set_copies().contains_key(&copy_id) {
            return Err("set copy does not exist".into());
        }
        let settings = load_settings(&tx)?;
        let definitions = load_catalog(&tx, &settings)?;
        let profiles = load_profiles(&tx, &settings, &definitions)?;
        let condition = profiles
            .get(&Target::SetCopy(copy_id))
            .map(|p| p.links.condition_id)
            .unwrap_or_else(|| {
                settings
                    .values()
                    .find(|r| {
                        matches!(
                            r.kind,
                            brickxinator_core::settings::Kind::Vocabulary {
                                vocabulary: Vocabulary::Condition,
                                system_unknown: true
                            }
                        )
                    })
                    .expect("settings validation retains Unknown")
                    .id
            });
        let location = profiles
            .get(&Target::SetCopy(copy_id))
            .and_then(|p| p.links.location_id);
        let contents = ledger
            .verification()
            .get(&copy_id)
            .into_iter()
            .flat_map(|state| state.lines.values())
            .filter(|line| line.verified.is_some())
            .map(|line| Content {
                line_id: line.id,
                holding_id: line.holding_id,
                holding: ledger.holdings()[&line.holding_id],
                links: Links {
                    archived: profiles
                        .get(&Target::SetCopy(copy_id))
                        .is_some_and(|p| p.links.archived),
                    definition_id: line.definition_id,
                    color_id: line.color_id,
                    condition_id: condition,
                    location_id: location,
                },
            })
            .collect();
        tx.commit()?;
        Ok(contents)
    }
}

fn require_schema(connection: &Connection) -> Result<()> {
    if !has_verification(connection)? {
        return Err(
            "set verification needs schema 5; automatic upgrades are not implemented".into(),
        );
    }
    Ok(())
}

pub(crate) fn persist(
    connection: &Connection,
    ledger: &Ledger,
    operation: &Operation,
    created: &[Id],
) -> Result<()> {
    let Body::Verify {
        copy_id, action, ..
    } = &operation.body
    else {
        return Ok(());
    };
    connection.execute(
        "INSERT INTO verification_operation(operation_id,copy_id) VALUES(?1,?2)",
        params![operation.id.to_string(), copy_id.to_string()],
    )?;
    let lines = match action {
        Action::AddLine {
            line_id,
            holding_id,
            definition_id,
            color_id,
            expected,
        } => vec![brickxinator_core::verification::ExpectedLine {
            line_id: *line_id,
            holding_id: *holding_id,
            definition_id: *definition_id,
            color_id: *color_id,
            expected: *expected,
        }],
        Action::CopyExpected { lines, .. } | Action::AddExpected { lines } => lines.clone(),
        _ => vec![],
    };
    for line in lines {
        connection.execute("INSERT INTO verification_line_identity(id,copy_id,reserved_holding_id,definition_id,color_id) VALUES(?1,?2,?3,?4,?5)",params![line.line_id.to_string(),copy_id.to_string(),line.holding_id.to_string(),line.definition_id.map(|id|id.to_string()),line.color_id.to_string()])?;
    }
    if let Action::Identity {
        line_id,
        definition_id,
        color_id,
    } = action
    {
        connection.execute("INSERT INTO verification_attribute_revision(operation_id,line_id,definition_id,color_id) VALUES(?1,?2,?3,?4)",params![operation.id.to_string(),line_id.to_string(),definition_id.map(|id|id.to_string()),color_id.to_string()])?;
    }
    for holding in created {
        if let Some(copy_id) = ledger.set_contents().get(holding) {
            let line = ledger.verification()[copy_id]
                .lines
                .values()
                .find(|line| line.holding_id == *holding)
                .ok_or("missing verification line")?;
            connection.execute(
                "INSERT INTO verified_content(holding_id,copy_id,line_id) VALUES(?1,?2,?3)",
                params![
                    holding.to_string(),
                    copy_id.to_string(),
                    line.id.to_string()
                ],
            )?;
        }
    }
    Ok(())
}

pub(crate) fn validate(connection: &Connection, ledger: &Ledger) -> Result<()> {
    let mut initial_attributes = BTreeMap::new();
    let mut attribute_revisions = BTreeMap::new();
    for operation in ledger.operations() {
        if let Body::Verify { action, .. } = &operation.body {
            match action {
                Action::AddLine {
                    line_id,
                    definition_id,
                    color_id,
                    ..
                } => {
                    initial_attributes.insert(*line_id, (*definition_id, *color_id));
                }
                Action::CopyExpected { lines, .. } | Action::AddExpected { lines } => {
                    for line in lines {
                        initial_attributes
                            .insert(line.line_id, (line.definition_id, line.color_id));
                    }
                }
                Action::Identity {
                    line_id,
                    definition_id,
                    color_id,
                } => {
                    attribute_revisions.insert(
                        operation.id.to_string(),
                        (
                            line_id.to_string(),
                            definition_id.map(|id| id.to_string()),
                            color_id.to_string(),
                        ),
                    );
                }
                _ => {}
            }
        }
    }
    let mut expected_lines = BTreeMap::new();
    let mut expected_contents = BTreeMap::new();
    let mut expected_operations = BTreeMap::new();
    for (copy_id, state) in ledger.verification() {
        for line in state.lines.values() {
            let (definition, color) = initial_attributes
                .get(&line.id)
                .ok_or("checklist line has no initial identity")?;
            expected_lines.insert(
                line.id.to_string(),
                (
                    copy_id.to_string(),
                    line.holding_id.to_string(),
                    definition.map(|id| id.to_string()),
                    color.to_string(),
                ),
            );
            if line.verified.is_some() {
                expected_contents.insert(
                    line.holding_id.to_string(),
                    (copy_id.to_string(), line.id.to_string()),
                );
            }
        }
    }
    for op in ledger.operations() {
        if let Body::Verify { copy_id, .. } = op.body {
            expected_operations.insert(op.id.to_string(), copy_id.to_string());
        }
    }
    let lines:BTreeMap<String,(String,String,Option<String>,String)>=connection.prepare("SELECT id,copy_id,reserved_holding_id,definition_id,color_id FROM verification_line_identity")?.query_map([],|r|Ok((r.get(0)?,(r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))))?.collect::<rusqlite::Result<_>>()?;
    let contents: BTreeMap<String, (String, String)> = connection
        .prepare("SELECT holding_id,copy_id,line_id FROM verified_content")?
        .query_map([], |r| Ok((r.get(0)?, (r.get(1)?, r.get(2)?))))?
        .collect::<rusqlite::Result<_>>()?;
    let operations: BTreeMap<String, String> = connection
        .prepare("SELECT operation_id,copy_id FROM verification_operation")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    if lines != expected_lines || contents != expected_contents || operations != expected_operations
    {
        return Err("verification references differ from journal replay".into());
    }
    let supported: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migration WHERE version=12)",
        [],
        |r| r.get(0),
    )?;
    let actual_attributes = if supported {
        connection.prepare("SELECT operation_id,line_id,definition_id,color_id FROM verification_attribute_revision")?.query_map([],|r|Ok((r.get::<_,String>(0)?,(r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?,r.get::<_,String>(3)?))))?.collect::<rusqlite::Result<BTreeMap<_,_>>>()?
    } else {
        BTreeMap::new()
    };
    if actual_attributes != attribute_revisions {
        return Err("checklist identity corrections differ from immutable audit".into());
    }
    for table in std::iter::once("verification_line_identity")
        .chain(supported.then_some("verification_attribute_revision"))
    {
        let invalid:bool=connection.query_row(&format!("SELECT EXISTS(SELECT 1 FROM {table} v JOIN settings_record c ON c.id=v.color_id LEFT JOIN catalog_definition d ON d.id=v.definition_id WHERE c.vocabulary IS NOT 'Color' OR v.definition_id IS NOT NULL AND d.kind!='Part')"),[],|r|r.get(0))?;
        if invalid {
            return Err("checklist references require a part definition and managed color".into());
        }
    }
    // Assembly is recorded once in the set state and materialized on holdings.
    for (holding, copy) in ledger.set_contents() {
        let expected = ledger
            .verification()
            .get(copy)
            .map_or(AssemblyState::Loose, |s| s.assembly);
        if ledger.holdings()[holding].assembly != expected {
            return Err("set-content assembly differs from its set".into());
        }
    }
    Ok(())
}
