//! Physical transfers and permanent copy lifecycle facts. No prices are inferred.
use crate::{
    inventory::{AssemblyState, Balance, Measure},
    journal::{Action, Command, Holding, MeasureChanges},
    ledger::{Id, LedgerError},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Portion {
    pub pieces: Option<u64>,
    pub weight_mg: Option<u64>,
    pub damaged_pieces: Option<u64>,
    pub damaged_weight_mg: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcquisitionPortion {
    pub invoice_line_id: Id,
    pub pieces: Option<u64>,
    pub weight_mg: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub holding_id: Id,
    pub expected_revision: u64,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    #[default]
    Active,
    Sold,
    Donated,
    Lost,
    Stolen,
    Destroyed,
    Void,
    Returned,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Handling {
    TransferAll,
    KeepAll,
    Custom { child_id: Id, keep: Portion },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Disposition {
    pub holding_id: Id,
    pub expected_revision: u64,
    pub handling: Handling,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lifecycle {
    pub copy_id: Id,
    pub expected_revision: u64,
    pub status: Status,
    pub effective_on: String,
    pub proceeds_cents: Option<u64>,
    pub recipient: Option<String>,
    pub notes: String,
    pub parts: Vec<Disposition>,
    pub corrects: Option<Id>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopyState {
    pub revision: u64,
    pub status: Status,
    pub last_event: Option<Id>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effect {
    pub copy_id: Id,
    pub before: CopyState,
    pub before_holdings: BTreeMap<Id, Holding>,
    pub after_holdings: BTreeMap<Id, Holding>,
    pub parents: BTreeMap<Id, Id>,
}
fn invalid(message: &'static str) -> LedgerError {
    LedgerError::InvalidStockAction(message)
}
fn unit(
    balance: Balance,
    amount: Option<u64>,
    damage: Option<u64>,
) -> Result<(Balance, MeasureChanges), LedgerError> {
    match amount {
        Some(n) => {
            let Measure::Known(owned) = balance.owned() else {
                return Err(invalid("partial transfer needs a known source measure"));
            };
            let Measure::Known(damaged) = balance.damaged() else {
                return Err(invalid("partial transfer needs known damaged measure"));
            };
            let d = damage
                .or((damaged == 0).then_some(0))
                .ok_or(invalid("specify the transferred damaged subset"))?;
            if n > owned || d > damaged || d > n || damaged - d > owned - n {
                return Err(invalid("transfer or damaged subset exceeds source stock"));
            }
            Ok((
                Balance::new(Measure::Known(n), Measure::Known(d))
                    .map_err(|_| invalid("invalid transferred damage"))?,
                MeasureChanges {
                    owned: Action::SetMeasured(owned - n),
                    damaged: Action::SetMeasured(damaged - d),
                },
            ))
        }
        None => {
            if damage.is_some_and(|d| d != 0) {
                return Err(invalid(
                    "damage transfer needs the measured total in that unit",
                ));
            }
            let zero = balance.owned() == Measure::Known(0);
            let owned = if zero {
                Measure::Known(0)
            } else {
                Measure::Unknown
            };
            let damaged = if balance.damaged() == Measure::Known(0) {
                Measure::Known(0)
            } else {
                Measure::Unknown
            };
            Ok((
                Balance::new(owned, damaged).map_err(|_| invalid("invalid unknown transfer"))?,
                MeasureChanges {
                    owned: if zero {
                        Action::Unchanged
                    } else {
                        Action::Invalidate
                    },
                    damaged: if damaged == Measure::Known(0) {
                        Action::Unchanged
                    } else {
                        Action::Invalidate
                    },
                },
            ))
        }
    }
}
pub fn split(
    source: Holding,
    expected_revision: u64,
    portion: Portion,
    actor: &str,
    reason: &str,
    key: &str,
) -> Result<(Holding, Holding), LedgerError> {
    if !portion.pieces.is_some_and(|n| n > 0) && !portion.weight_mg.is_some_and(|n| n > 0) {
        return Err(invalid("split needs a positive measured count or weight"));
    }
    let (pieces, p) = unit(source.pieces, portion.pieces, portion.damaged_pieces)?;
    let (weight_mg, w) = unit(
        source.weight_mg,
        portion.weight_mg,
        portion.damaged_weight_mg,
    )?;
    let remaining = source
        .apply(&Command {
            request_key: key.into(),
            expected_revision,
            actor: actor.into(),
            reason: reason.into(),
            pieces: p,
            weight_mg: w,
            assembly: None,
        })
        .map_err(LedgerError::Journal)?;
    Ok((
        remaining,
        Holding {
            pieces,
            weight_mg,
            assembly: AssemblyState::Loose,
            revision: 1,
        },
    ))
}
pub fn replace(
    source: Holding,
    pieces: Balance,
    weight_mg: Balance,
    assembly: AssemblyState,
) -> Result<Holding, LedgerError> {
    Ok(Holding {
        pieces,
        weight_mg,
        assembly,
        revision: source
            .revision
            .checked_add(1)
            .ok_or(invalid("holding revision overflow"))?,
    })
}
pub fn empty(source: Holding) -> Result<Holding, LedgerError> {
    let zero =
        Balance::new(Measure::Known(0), Measure::Known(0)).map_err(|_| invalid("invalid zero"))?;
    replace(source, zero, zero, AssemblyState::Loose)
}
pub fn merged(sources: &[Holding]) -> Result<Holding, LedgerError> {
    fn sum(values: impl Iterator<Item = Measure>) -> Result<Measure, LedgerError> {
        let mut total = 0u64;
        let mut unknown = false;
        for m in values {
            match m {
                Measure::Known(n) => {
                    total = total
                        .checked_add(n)
                        .ok_or(invalid("merge quantity overflow"))?
                }
                Measure::Unknown => unknown = true,
            }
        }
        Ok(if unknown {
            Measure::Unknown
        } else {
            Measure::Known(total)
        })
    }
    let pieces = Balance::new(
        sum(sources.iter().map(|h| h.pieces.owned()))?,
        sum(sources.iter().map(|h| h.pieces.damaged()))?,
    )
    .map_err(|_| invalid("merged damage exceeds ownership"))?;
    let weight_mg = Balance::new(
        sum(sources.iter().map(|h| h.weight_mg.owned()))?,
        sum(sources.iter().map(|h| h.weight_mg.damaged()))?,
    )
    .map_err(|_| invalid("merged weight damage exceeds ownership"))?;
    Ok(Holding {
        pieces,
        weight_mg,
        assembly: AssemblyState::Loose,
        revision: 1,
    })
}
