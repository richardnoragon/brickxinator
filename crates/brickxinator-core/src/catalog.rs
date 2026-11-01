//! Catalog definitions describe items; they never introduce ownership.
use crate::{
    ledger::Id,
    settings::{self, Kind as SettingKind, Record as Setting, Vocabulary},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Part,
    Set,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Details {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expected_contents: Vec<TemplateLine>,
    pub manufacturer_id: Id,
    pub number: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub release_year: Option<u16>,
    pub theme: Option<String>,
    pub stated_piece_count: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateLine {
    pub definition_id: Option<Id>,
    pub color_id: Id,
    pub expected: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Definition {
    pub id: Id,
    pub revision: u64,
    pub kind: Kind,
    pub archived: bool,
    pub details: Details,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub id: Id,
    pub request_key: String,
    pub actor: String,
    pub reason: String,
    pub action: Action,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Action {
    Create {
        id: Id,
        kind: Kind,
        details: Details,
    },
    Edit {
        id: Id,
        expected_revision: u64,
        details: Details,
    },
    Archive {
        id: Id,
        expected_revision: u64,
        archived: bool,
    },
}
impl Action {
    pub fn target(&self) -> Id {
        match *self {
            Self::Create { id, .. } | Self::Edit { id, .. } | Self::Archive { id, .. } => id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    pub before: Option<Definition>,
    pub after: Definition,
}

pub fn normalized_number(number: &Option<String>) -> Option<String> {
    number
        .as_ref()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
}

pub fn require_vocabulary(
    settings: &BTreeMap<Id, Setting>,
    id: Id,
    vocabulary: Vocabulary,
    require_selectable: bool,
) -> Result<(), &'static str> {
    if !matches!(settings.get(&id).map(|r|&r.kind),Some(SettingKind::Vocabulary {vocabulary:v,..}) if *v==vocabulary)
    {
        return Err("reference belongs to the wrong vocabulary or does not exist");
    }
    if require_selectable && !settings::selectable(settings, id) {
        return Err("archived settings cannot be newly selected");
    }
    Ok(())
}

/// Historical replay permits retained archived references. New commands check
/// selectability only when the selected identity changes.
pub fn prepare(
    records: &BTreeMap<Id, Definition>,
    settings: &BTreeMap<Id, Setting>,
    command: &Command,
    historical: bool,
) -> Result<Change, &'static str> {
    if command.actor.trim().is_empty() || command.request_key.trim().is_empty() {
        return Err("actor and request key are required");
    }
    let id = command.action.target();
    let before = records.get(&id).cloned();
    let after = match &command.action {
        Action::Create { kind, details, .. } => {
            if before.is_some() {
                return Err("definition identity already exists");
            }
            Definition {
                id,
                revision: 1,
                kind: *kind,
                archived: false,
                details: details.clone(),
            }
        }
        action => {
            let mut record = before.clone().ok_or("definition does not exist")?;
            let expected = match action {
                Action::Edit {
                    expected_revision, ..
                }
                | Action::Archive {
                    expected_revision, ..
                } => *expected_revision,
                _ => unreachable!(),
            };
            if record.revision != expected {
                return Err("definition changed since this command was prepared");
            }
            record.revision = record
                .revision
                .checked_add(1)
                .ok_or("definition revision overflow")?;
            match action {
                Action::Edit { details, .. } => record.details = details.clone(),
                Action::Archive { archived, .. } => record.archived = *archived,
                _ => unreachable!(),
            }
            record
        }
    };
    let changed_manufacturer = before
        .as_ref()
        .is_none_or(|old| old.details.manufacturer_id != after.details.manufacturer_id);
    require_vocabulary(
        settings,
        after.details.manufacturer_id,
        Vocabulary::Manufacturer,
        !historical && changed_manufacturer,
    )?;
    if after.kind == Kind::Part
        && (after.details.release_year.is_some()
            || after.details.theme.is_some()
            || after.details.stated_piece_count.is_some()
            || !after.details.expected_contents.is_empty())
    {
        return Err("set metadata is only valid for set definitions");
    }
    if after.details.expected_contents.len() > 10_000 {
        return Err("definition checklist exceeds 10000 lines");
    }
    for line in &after.details.expected_contents {
        let retained =
            before.as_ref().is_some_and(|record| {
                record.details.expected_contents.iter().any(|old| {
                    old.color_id == line.color_id && old.definition_id == line.definition_id
                })
            });
        require_vocabulary(
            settings,
            line.color_id,
            Vocabulary::Color,
            !historical && !retained,
        )?;
        if let Some(id) = line.definition_id {
            let part = records
                .get(&id)
                .ok_or("template part definition does not exist")?;
            if part.kind != Kind::Part || !historical && !retained && part.archived {
                return Err(
                    "template needs a part definition, with active references for new selections",
                );
            }
        }
    }
    for value in [
        &after.details.number,
        &after.details.name,
        &after.details.theme,
    ] {
        if value
            .as_ref()
            .is_some_and(|s| s.chars().count() > 512 || s.chars().any(char::is_control))
        {
            return Err("catalog text exceeds its limit or contains control characters");
        }
    }
    if after
        .details
        .description
        .as_ref()
        .is_some_and(|s| s.len() > 65536)
    {
        return Err("catalog description exceeds its limit");
    }
    let unknown = matches!(
        settings[&after.details.manufacturer_id].kind,
        SettingKind::Vocabulary {
            system_unknown: true,
            ..
        }
    );
    if !unknown
        && let Some(number) = normalized_number(&after.details.number)
        && records.values().any(|r| {
            r.id != id
                && r.kind == after.kind
                && r.details.manufacturer_id == after.details.manufacturer_id
                && normalized_number(&r.details.number).as_ref() == Some(&number)
        })
    {
        return Err(
            "known manufacturer and number already identify a definition in this namespace",
        );
    }
    Ok(Change { before, after })
}
