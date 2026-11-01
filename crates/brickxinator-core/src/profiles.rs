//! Descriptive inventory links. Quantities remain exclusively in the ledger.
use crate::{
    catalog::{self, Definition},
    ledger::Id,
    settings::{self, Kind, Record, Vocabulary},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Target {
    Holding(Id),
    SetCopy(Id),
}
impl Target {
    pub fn id(self) -> Id {
        match self {
            Self::Holding(id) | Self::SetCopy(id) => id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Links {
    #[serde(default, skip_serializing_if = "is_false")]
    pub archived: bool,
    pub definition_id: Option<Id>,
    pub color_id: Id,
    pub condition_id: Id,
    pub location_id: Option<Id>,
}
fn is_false(value: &bool) -> bool {
    !value
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub target: Target,
    pub revision: u64,
    pub links: Links,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub id: Id,
    pub request_key: String,
    pub actor: String,
    pub reason: String,
    pub target: Target,
    pub expected_revision: u64,
    pub links: Links,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    pub before: Option<Profile>,
    pub after: Profile,
}

pub fn prepare(
    before: Option<Profile>,
    settings: &BTreeMap<Id, Record>,
    definitions: &BTreeMap<Id, Definition>,
    command: &Command,
    historical: bool,
) -> Result<Change, &'static str> {
    if command.actor.trim().is_empty() || command.request_key.trim().is_empty() {
        return Err("actor and request key are required");
    }
    if before.as_ref().map_or(0, |r| r.revision) != command.expected_revision {
        return Err("inventory profile changed since this command was prepared");
    }
    let previous = before.as_ref().map(|r| &r.links);
    catalog::require_vocabulary(
        settings,
        command.links.color_id,
        Vocabulary::Color,
        !historical && previous.is_none_or(|r| r.color_id != command.links.color_id),
    )?;
    catalog::require_vocabulary(
        settings,
        command.links.condition_id,
        Vocabulary::Condition,
        !historical && previous.is_none_or(|r| r.condition_id != command.links.condition_id),
    )?;
    if let Some(id) = command.links.definition_id {
        let definition = definitions
            .get(&id)
            .ok_or("catalog definition does not exist")?;
        let expected = match command.target {
            Target::Holding(_) => catalog::Kind::Part,
            Target::SetCopy(_) => catalog::Kind::Set,
        };
        if definition.kind != expected {
            return Err("definition namespace does not match the inventory target");
        }
        if !historical
            && previous.is_none_or(|r| r.definition_id != Some(id))
            && definition.archived
        {
            return Err("archived definitions cannot be newly selected");
        }
    }
    if let Some(id) = command.links.location_id {
        if !matches!(
            settings.get(&id).map(|r| &r.kind),
            Some(Kind::Location { .. })
        ) {
            return Err("storage location does not exist or is not a location");
        }
        if !historical
            && previous.is_none_or(|r| r.location_id != Some(id))
            && !settings::selectable(settings, id)
        {
            return Err("archived locations or descendants cannot be newly selected");
        }
    }
    Ok(Change {
        before,
        after: Profile {
            target: command.target,
            revision: command
                .expected_revision
                .checked_add(1)
                .ok_or("profile revision overflow")?,
            links: command.links.clone(),
        },
    })
}
