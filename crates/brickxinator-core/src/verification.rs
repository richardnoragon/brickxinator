//! Expected contents are a checklist, not ownership. Counts are absolute.
use crate::{inventory::AssemblyState, ledger::Id};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    #[serde(default, skip_serializing_if = "is_false")]
    pub retired: bool,
    pub id: Id,
    pub holding_id: Id,
    pub definition_id: Option<Id>,
    pub color_id: Id,
    pub expected: Option<u64>,
    pub verified: Option<u64>,
    pub finished: bool,
}
fn is_false(value: &bool) -> bool {
    !value
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difference {
    Unfinished,
    Unknown,
    Exact,
    Short(u64),
    Excess(u64),
}
impl Line {
    pub fn difference(&self) -> Difference {
        if !self.finished {
            return Difference::Unfinished;
        }
        match (self.expected, self.verified) {
            (Some(expected), Some(actual)) if actual < expected => {
                Difference::Short(expected - actual)
            }
            (Some(expected), Some(actual)) if actual > expected => {
                Difference::Excess(actual - expected)
            }
            (Some(_), Some(_)) => Difference::Exact,
            _ => Difference::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub copy_id: Id,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub revision: u64,
    pub assembly: AssemblyState,
    pub completed: bool,
    pub lines: BTreeMap<Id, Line>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    NotStarted,
    InProgress,
    Completed,
}
impl State {
    pub fn status(&self) -> Status {
        if self.completed {
            Status::Completed
        } else if self
            .lines
            .values()
            .any(|line| !line.retired && line.verified.is_some())
        {
            Status::InProgress
        } else {
            Status::NotStarted
        }
    }
    pub fn new(copy_id: Id, assembly: AssemblyState) -> Self {
        Self {
            copy_id,
            label: None,
            revision: 0,
            assembly,
            completed: false,
            lines: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub id: Id,
    pub request_key: String,
    pub actor: String,
    pub reason: String,
    pub copy_id: Id,
    pub expected_revision: u64,
    pub action: Action,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedLine {
    pub line_id: Id,
    pub holding_id: Id,
    pub definition_id: Option<Id>,
    pub color_id: Id,
    pub expected: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Action {
    Identity {
        line_id: Id,
        definition_id: Option<Id>,
        color_id: Id,
    },
    Active {
        line_id: Id,
        active: bool,
    },
    ReconcileCount {
        line_id: Id,
        total: u64,
        expected_holding_revision: u64,
        expected_owned: crate::inventory::Measure,
    },
    AddExpected {
        lines: Vec<ExpectedLine>,
    },
    Rename {
        label: String,
    },
    CopyExpected {
        source_copy_id: Id,
        lines: Vec<ExpectedLine>,
    },
    AddLine {
        line_id: Id,
        holding_id: Id,
        definition_id: Option<Id>,
        color_id: Id,
        expected: Option<u64>,
    },
    Expected {
        line_id: Id,
        expected: Option<u64>,
    },
    Count {
        line_id: Id,
        total: u64,
    },
    Finish {
        line_id: Id,
    },
    Complete,
    Assembly {
        state: AssemblyState,
    },
}
