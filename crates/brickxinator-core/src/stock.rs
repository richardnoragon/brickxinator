//! Explicit stock adjustments and classification lifecycle; no purchase rewrites.
use crate::{
    inventory::Measure,
    journal::{Action as MeasureAction, Holding, MeasureChanges},
    ledger::Id,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Removal {
    Lost,
    Donated,
    Sold,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Adjustment {
    /// Explicit removed totals and damaged subsets. Missing dimensions become
    /// unknown; no piece/weight conversion is performed.
    Remove {
        kind: Removal,
        pieces: Option<u64>,
        weight_mg: Option<u64>,
        #[serde(default)]
        damaged_pieces: u64,
        #[serde(default)]
        damaged_weight_mg: u64,
    },
    Broken {
        pieces: Option<u64>,
        weight_mg: Option<u64>,
    },
    Correction {
        pieces: MeasureChanges,
        weight_mg: MeasureChanges,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Action {
    Adjust(Adjustment),
    Complete { confirm_physical_empty: bool },
    Reopen,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub id: Id,
    pub request_key: String,
    pub actor: String,
    pub reason: String,
    pub holding_id: Id,
    pub expected_revision: u64,
    pub action: Action,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Completion {
    pub operation_id: Id,
    pub physical_empty_confirmed: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Classification {
    pub complete: bool,
    pub last_completion: Option<Completion>,
    pub last_reopen: Option<Id>,
}

pub fn changes(
    holding: Holding,
    adjustment: &Adjustment,
) -> Result<(MeasureChanges, MeasureChanges), &'static str> {
    match adjustment {
        Adjustment::Correction { pieces, weight_mg } => {
            if *pieces == MeasureChanges::default() && *weight_mg == MeasureChanges::default() {
                return Err("correction must specify a measure action");
            }
            Ok((*pieces, *weight_mg))
        }
        Adjustment::Remove {
            pieces,
            weight_mg,
            damaged_pieces,
            damaged_weight_mg,
            ..
        } => {
            positive(*pieces, *weight_mg)?;
            Ok((
                remove(
                    holding.pieces.owned(),
                    holding.pieces.damaged(),
                    *pieces,
                    *damaged_pieces,
                )?,
                remove(
                    holding.weight_mg.owned(),
                    holding.weight_mg.damaged(),
                    *weight_mg,
                    *damaged_weight_mg,
                )?,
            ))
        }
        Adjustment::Broken { pieces, weight_mg } => {
            positive(*pieces, *weight_mg)?;
            Ok((
                broken(holding.pieces.owned(), holding.pieces.damaged(), *pieces)?,
                broken(
                    holding.weight_mg.owned(),
                    holding.weight_mg.damaged(),
                    *weight_mg,
                )?,
            ))
        }
    }
}

fn positive(pieces: Option<u64>, weight: Option<u64>) -> Result<(), &'static str> {
    if !pieces.is_some_and(|n| n > 0) && !weight.is_some_and(|n| n > 0) {
        return Err("adjustment needs a positive count or weight");
    }
    Ok(())
}
fn remove(
    owned: Measure,
    damaged: Measure,
    amount: Option<u64>,
    removed_damage: u64,
) -> Result<MeasureChanges, &'static str> {
    match amount {
        Some(amount) => {
            let (Measure::Known(owned), Measure::Known(damaged)) = (owned, damaged) else {
                return Err("removal needs known ownership and damage in the selected unit");
            };
            if amount > owned || removed_damage > amount || removed_damage > damaged {
                return Err("removed totals or damaged subset exceed known stock");
            }
            let next_owned = owned - amount;
            let next_damage = damaged - removed_damage;
            if next_damage > next_owned {
                return Err(
                    "removal would discard damaged stock without recording its damaged subset",
                );
            }
            Ok(MeasureChanges {
                owned: MeasureAction::SetMeasured(next_owned),
                damaged: MeasureAction::SetMeasured(next_damage),
            })
        }
        None => {
            if removed_damage != 0 {
                return Err(
                    "a damaged removal subset needs a measured removal total in the same unit",
                );
            }
            Ok(MeasureChanges {
                owned: if owned == Measure::Known(0) {
                    MeasureAction::Unchanged
                } else {
                    MeasureAction::Invalidate
                },
                damaged: MeasureAction::Unchanged,
            })
        }
    }
}

fn broken(
    owned: Measure,
    damaged: Measure,
    amount: Option<u64>,
) -> Result<MeasureChanges, &'static str> {
    let action = match amount {
        Some(amount) => {
            let (Measure::Known(owned), Measure::Known(damaged)) = (owned, damaged) else {
                return Err(
                    "damage adjustment needs known ownership and prior damage in the selected unit",
                );
            };
            let next = damaged.checked_add(amount).ok_or("damage overflow")?;
            if next > owned {
                return Err("new damage exceeds ownership");
            }
            MeasureAction::SetMeasured(next)
        }
        None if owned == Measure::Known(0) => MeasureAction::Unchanged,
        None => MeasureAction::Invalidate,
    };
    Ok(MeasureChanges {
        owned: MeasureAction::Unchanged,
        damaged: action,
    })
}

pub fn can_complete(holding: Holding, confirmed: bool) -> Result<(), &'static str> {
    let measures = [
        holding.pieces.owned(),
        holding.weight_mg.owned(),
        holding.pieces.damaged(),
        holding.weight_mg.damaged(),
    ];
    if measures
        .iter()
        .any(|m| matches!(m,Measure::Known(n) if *n>0))
    {
        return Err("known nonzero residue blocks classification completion");
    }
    if measures.contains(&Measure::Unknown) && !confirmed {
        return Err("unknown residue requires explicit physical-empty confirmation");
    }
    Ok(())
}
