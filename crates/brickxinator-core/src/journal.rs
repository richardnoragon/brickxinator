//! Append-only, per-holding movement kernel. The ledger module composes these
//! rules across holdings; persistence lives in brickxinator-infra.
//! This module does not accept arbitrary purchase records.

use crate::inventory::{AssemblyState, Balance, Measure};
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Unchanged,
    Delta(i64),
    SetMeasured(u64),
    Invalidate,
}

impl Action {
    fn apply(self, previous: Measure) -> Result<Measure, JournalError> {
        match self {
            Self::Unchanged => Ok(previous),
            Self::Invalidate => Ok(Measure::Unknown),
            Self::SetMeasured(value) => Ok(Measure::Known(value)),
            Self::Delta(delta) => match previous {
                Measure::Unknown => Err(JournalError::UnknownDelta),
                Measure::Known(value) => value
                    .checked_add_signed(delta)
                    .map(Measure::Known)
                    .ok_or(JournalError::OutOfRange),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeasureChanges {
    pub owned: Action,
    pub damaged: Action,
}

impl Default for MeasureChanges {
    fn default() -> Self {
        Self {
            owned: Action::Unchanged,
            damaged: Action::Unchanged,
        }
    }
}

impl MeasureChanges {
    fn apply(self, balance: Balance) -> Result<Balance, JournalError> {
        Balance::new(
            self.owned.apply(balance.owned())?,
            self.damaged.apply(balance.damaged())?,
        )
        .map_err(|_| JournalError::DamageExceedsOwnership)
    }
}

/// Weight values are integer milligrams, as proposed in the schema draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub request_key: String,
    pub expected_revision: u64,
    pub actor: String,
    pub reason: String,
    pub pieces: MeasureChanges,
    pub weight_mg: MeasureChanges,
    pub assembly: Option<AssemblyState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Holding {
    pub pieces: Balance,
    pub weight_mg: Balance,
    pub assembly: AssemblyState,
    pub revision: u64,
}

impl Default for Holding {
    fn default() -> Self {
        let unknown = Balance::new(Measure::Unknown, Measure::Unknown)
            .expect("unknown measurements are valid");
        Self {
            pieces: unknown,
            weight_mg: unknown,
            assembly: AssemblyState::Loose,
            revision: 0,
        }
    }
}

impl Holding {
    pub(crate) fn apply(self, command: &Command) -> Result<Self, JournalError> {
        if command.request_key.trim().is_empty()
            || command.actor.trim().is_empty()
            || command.reason.trim().is_empty()
        {
            return Err(JournalError::MissingAuditContext);
        }
        if command.expected_revision != self.revision {
            return Err(JournalError::StaleRevision);
        }
        Ok(Self {
            pieces: command.pieces.apply(self.pieces)?,
            weight_mg: command.weight_mg.apply(self.weight_mg)?,
            assembly: command.assembly.unwrap_or(self.assembly),
            revision: self
                .revision
                .checked_add(1)
                .ok_or(JournalError::OutOfRange)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    sequence: u64,
    command: Command,
}

impl Entry {
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
    pub fn command(&self) -> &Command {
        &self.command
    }
}

#[derive(Debug, Default)]
pub struct Journal {
    entries: Vec<Entry>,
    current: Holding,
}

impl Journal {
    pub fn current(&self) -> Holding {
        self.current
    }
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Retry checks precede revision checks, so an identical retry returns its
    /// original result even after later successful commands.
    pub fn append(&mut self, command: Command) -> Result<Holding, JournalError> {
        if let Some(index) = self
            .entries
            .iter()
            .position(|e| e.command.request_key == command.request_key)
        {
            if self.entries[index].command != command {
                return Err(JournalError::RequestKeyConflict);
            }
            return Self::replay(&self.entries[..=index]);
        }
        let next = self.current.apply(&command)?;
        self.entries.push(Entry {
            sequence: next.revision,
            command,
        });
        self.current = next;
        Ok(next)
    }

    pub fn replay(entries: &[Entry]) -> Result<Holding, JournalError> {
        let mut holding = Holding::default();
        let mut keys = std::collections::HashSet::new();
        for entry in entries {
            if entry.sequence
                != holding
                    .revision
                    .checked_add(1)
                    .ok_or(JournalError::OutOfRange)?
            {
                return Err(JournalError::InvalidSequence);
            }
            if !keys.insert(&entry.command.request_key) {
                return Err(JournalError::RequestKeyConflict);
            }
            holding = holding.apply(&entry.command)?;
        }
        Ok(holding)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JournalError {
    UnknownDelta,
    OutOfRange,
    DamageExceedsOwnership,
    MissingAuditContext,
    StaleRevision,
    RequestKeyConflict,
    InvalidSequence,
}

impl fmt::Display for JournalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UnknownDelta => "a delta requires a known previous measurement",
            Self::OutOfRange => "measurement or revision is outside its supported range",
            Self::DamageExceedsOwnership => "damaged measurement exceeds ownership",
            Self::MissingAuditContext => "request key, actor, and reason are required",
            Self::StaleRevision => "holding changed since the command was prepared",
            Self::RequestKeyConflict => "request key already used with a different payload",
            Self::InvalidSequence => "journal sequence is not contiguous",
        })
    }
}

impl Error for JournalError {}
