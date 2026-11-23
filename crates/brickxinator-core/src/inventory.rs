//! Independent piece/weight balances, without estimates or unit conversion.

use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

/// A missing measurement is distinct from a measured zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Measure {
    Unknown,
    Known(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssemblyState {
    Loose,
    Assembled,
    PartiallyAssembled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageExceedsOwnership;

impl fmt::Display for DamageExceedsOwnership {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("damaged measurement exceeds owned measurement")
    }
}

impl Error for DamageExceedsOwnership {}

/// One unit's balance. Construct separately for pieces and weight.
/// Unknown ownership cannot establish that a known damaged measure is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "BalanceInput")]
pub struct Balance {
    owned: Measure,
    damaged: Measure,
}

impl Balance {
    pub fn new(owned: Measure, damaged: Measure) -> Result<Self, DamageExceedsOwnership> {
        if let (Measure::Known(owned), Measure::Known(damaged)) = (owned, damaged)
            && damaged > owned
        {
            return Err(DamageExceedsOwnership);
        }
        Ok(Self { owned, damaged })
    }

    pub fn owned(self) -> Measure {
        self.owned
    }

    pub fn damaged(self) -> Measure {
        self.damaged
    }

    /// Both assembled states exclude the entire holding; damage is not
    /// subtracted a second time. Loose stock requires both measurements.
    pub fn available(self, assembly: AssemblyState) -> Measure {
        if assembly != AssemblyState::Loose {
            return Measure::Known(0);
        }
        match (self.owned, self.damaged) {
            (Measure::Known(0), _) => Measure::Known(0),
            (Measure::Known(owned), Measure::Known(damaged)) => Measure::Known(owned - damaged),
            _ => Measure::Unknown,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BalanceInput {
    owned: Measure,
    damaged: Measure,
}

impl TryFrom<BalanceInput> for Balance {
    type Error = DamageExceedsOwnership;
    fn try_from(input: BalanceInput) -> Result<Self, Self::Error> {
        Self::new(input.owned, input.damaged)
    }
}
