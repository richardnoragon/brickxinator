//! Managed vocabularies and location trees. IDs survive edits and archiving.
use crate::ledger::Id;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vocabulary {
    Manufacturer,
    Color,
    Condition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Vocabulary {
        vocabulary: Vocabulary,
        system_unknown: bool,
    },
    Location {
        parent_id: Option<Id>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub id: Id,
    pub revision: u64,
    pub name: String,
    pub archived: bool,
    pub kind: Kind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub id: Id,
    pub request_key: String,
    pub actor: String,
    pub reason: String,
    pub accept_name_warnings: bool,
    pub action: Action,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Action {
    CreateVocabulary {
        id: Id,
        vocabulary: Vocabulary,
        name: String,
    },
    CreateLocation {
        id: Id,
        parent_id: Option<Id>,
        name: String,
    },
    Rename {
        id: Id,
        expected_revision: u64,
        name: String,
    },
    Reparent {
        id: Id,
        expected_revision: u64,
        parent_id: Option<Id>,
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
            Self::CreateVocabulary { id, .. }
            | Self::CreateLocation { id, .. }
            | Self::Rename { id, .. }
            | Self::Reparent { id, .. }
            | Self::Archive { id, .. } => id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Warning {
    Duplicate { id: Id, name: String },
    Similar { id: Id, name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    pub before: Option<Record>,
    pub after: Record,
    pub warnings: Vec<Warning>,
}

/// Normalization is for comparisons only. The entered display name is preserved.
pub fn normalized(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub fn prepare(records: &BTreeMap<Id, Record>, command: &Command) -> Result<Change, SettingsError> {
    if command.actor.trim().is_empty() || command.request_key.trim().is_empty() {
        return Err(SettingsError::MissingContext);
    }
    let id = command.action.target();
    let before = records.get(&id).cloned();
    let after = match &command.action {
        Action::CreateVocabulary {
            vocabulary, name, ..
        } => {
            if before.is_some() {
                return Err(SettingsError::Conflict);
            }
            Record {
                id,
                revision: 1,
                name: name.clone(),
                archived: false,
                kind: Kind::Vocabulary {
                    vocabulary: *vocabulary,
                    system_unknown: false,
                },
            }
        }
        Action::CreateLocation {
            parent_id, name, ..
        } => {
            if before.is_some() {
                return Err(SettingsError::Conflict);
            }
            require_parent(records, *parent_id)?;
            Record {
                id,
                revision: 1,
                name: name.clone(),
                archived: false,
                kind: Kind::Location {
                    parent_id: *parent_id,
                },
            }
        }
        action => {
            let mut record = before.clone().ok_or(SettingsError::MissingRecord)?;
            if matches!(
                record.kind,
                Kind::Vocabulary {
                    system_unknown: true,
                    ..
                }
            ) {
                return Err(SettingsError::ProtectedUnknown);
            }
            let revision = match action {
                Action::Rename {
                    expected_revision, ..
                }
                | Action::Reparent {
                    expected_revision, ..
                }
                | Action::Archive {
                    expected_revision, ..
                } => *expected_revision,
                _ => unreachable!(),
            };
            if record.revision != revision {
                return Err(SettingsError::StaleRevision);
            }
            record.revision = record
                .revision
                .checked_add(1)
                .ok_or(SettingsError::OutOfRange)?;
            match action {
                Action::Rename { name, .. } => record.name = name.clone(),
                Action::Archive { archived, .. } => record.archived = *archived,
                Action::Reparent { parent_id, .. } => {
                    if !matches!(record.kind, Kind::Location { .. }) {
                        return Err(SettingsError::NotLocation);
                    }
                    require_parent(records, *parent_id)?;
                    record.kind = Kind::Location {
                        parent_id: *parent_id,
                    };
                }
                _ => unreachable!(),
            }
            record
        }
    };
    validate_record(&after)?;
    let mut candidate = records.clone();
    candidate.insert(id, after.clone());
    validate_tree(&candidate)?;
    let check_names = !matches!(command.action, Action::Archive { .. });
    let warnings = if check_names {
        name_warnings(records, &after)
    } else {
        vec![]
    };
    if !warnings.is_empty() && !command.accept_name_warnings {
        return Err(SettingsError::NameWarnings(warnings));
    }
    Ok(Change {
        before,
        after,
        warnings,
    })
}

pub fn validate_record(record: &Record) -> Result<(), SettingsError> {
    if record.revision == 0
        || normalized(&record.name).is_empty()
        || record.name.chars().count() > 128
        || record.name.chars().any(char::is_control)
    {
        return Err(SettingsError::InvalidName);
    }
    if let Kind::Vocabulary { system_unknown, .. } = record.kind {
        if system_unknown && (record.name != "Unknown" || record.archived || record.revision != 1) {
            return Err(SettingsError::ProtectedUnknown);
        }
        if !system_unknown && normalized(&record.name) == "unknown" {
            return Err(SettingsError::ProtectedUnknown);
        }
    }
    Ok(())
}

pub fn validate_tree(records: &BTreeMap<Id, Record>) -> Result<(), SettingsError> {
    for record in records.values() {
        validate_record(record)?;
        if !matches!(record.kind, Kind::Location { .. }) {
            continue;
        }
        let mut seen = BTreeSet::new();
        let mut current = Some(record.id);
        while let Some(id) = current {
            if !seen.insert(id) {
                return Err(SettingsError::Cycle);
            }
            let parent = records.get(&id).ok_or(SettingsError::MissingRecord)?;
            let Kind::Location { parent_id } = parent.kind else {
                return Err(SettingsError::NotLocation);
            };
            current = parent_id;
        }
    }
    Ok(())
}

pub fn selectable(records: &BTreeMap<Id, Record>, id: Id) -> bool {
    let mut seen = BTreeSet::new();
    let mut current = Some(id);
    while let Some(id) = current {
        if !seen.insert(id) {
            return false;
        }
        let Some(record) = records.get(&id) else {
            return false;
        };
        if record.archived {
            return false;
        }
        current = match record.kind {
            Kind::Location { parent_id } => parent_id,
            _ => None,
        };
    }
    true
}

fn require_parent(records: &BTreeMap<Id, Record>, parent: Option<Id>) -> Result<(), SettingsError> {
    if let Some(id) = parent {
        if !matches!(
            records.get(&id).map(|r| &r.kind),
            Some(Kind::Location { .. })
        ) {
            return Err(SettingsError::NotLocation);
        }
        if !selectable(records, id) {
            return Err(SettingsError::ArchivedParent);
        }
    }
    Ok(())
}

fn name_warnings(records: &BTreeMap<Id, Record>, target: &Record) -> Vec<Warning> {
    let name = normalized(&target.name);
    records
        .values()
        .filter(|r| {
            r.id != target.id
                && match (&r.kind, &target.kind) {
                    (
                        Kind::Vocabulary { vocabulary: a, .. },
                        Kind::Vocabulary { vocabulary: b, .. },
                    ) => a == b,
                    (Kind::Location { parent_id: a }, Kind::Location { parent_id: b }) => a == b,
                    _ => false,
                }
        })
        .filter_map(|r| {
            let other = normalized(&r.name);
            if name == other {
                Some(Warning::Duplicate {
                    id: r.id,
                    name: r.name.clone(),
                })
            } else if name.chars().count() >= 3
                && other.chars().count() >= 3
                && edit_distance(&name, &other) <= 1
            {
                Some(Warning::Similar {
                    id: r.id,
                    name: r.name.clone(),
                })
            } else {
                None
            }
        })
        .collect()
}

fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<_> = b.chars().collect();
    let mut previous: Vec<_> = (0..=b.len()).collect();
    for (i, a) in a.chars().enumerate() {
        let mut row = vec![i + 1];
        for (j, b) in b.iter().enumerate() {
            row.push(
                (row[j] + 1)
                    .min(previous[j + 1] + 1)
                    .min(previous[j] + usize::from(a != *b)),
            );
        }
        previous = row;
    }
    previous[b.len()]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsError {
    MissingContext,
    Conflict,
    MissingRecord,
    ProtectedUnknown,
    StaleRevision,
    OutOfRange,
    NotLocation,
    Cycle,
    ArchivedParent,
    InvalidName,
    NameWarnings(Vec<Warning>),
}
impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::MissingContext => "actor and request key are required",
            Self::Conflict => "settings identity already exists",
            Self::MissingRecord => "settings record does not exist",
            Self::ProtectedUnknown => "system Unknown is protected and cannot be duplicated",
            Self::StaleRevision => "settings record changed since this command was prepared",
            Self::OutOfRange => "settings revision is out of range",
            Self::NotLocation => "parent or target must be a location",
            Self::Cycle => "location hierarchy cannot contain cycles",
            Self::ArchivedParent => "new parent must be selectable",
            Self::InvalidName => "name must contain 1 to 128 characters and no control characters",
            Self::NameWarnings(_) => "similar or duplicate names require explicit acknowledgement",
        })
    }
}
impl Error for SettingsError {}
