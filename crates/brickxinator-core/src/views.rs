//! Read models expose current stock with explicit missing-measure coverage.
use crate::{
    inventory::{AssemblyState, Measure},
    journal::Holding,
    ledger::Id,
    purchases::Status,
    verification,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    Holding(Id),
    SetCopy(Id),
    UnquantifiedSetLine(Id),
}
impl Target {
    pub fn id(self) -> Id {
        match self {
            Self::Holding(id) | Self::SetCopy(id) | Self::UnquantifiedSetLine(id) => id,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    LooseBatch,
    MixedLot,
    IdentifiedBatch,
    SetContent,
    SetCopy,
    UnquantifiedSetLine,
    OpeningHolding,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Named {
    pub id: Id,
    pub name: String,
    pub archived: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryRow {
    #[serde(default)]
    pub archived: bool,
    pub target: Target,
    pub kind: Kind,
    pub label: String,
    pub number: Option<String>,
    pub definition_id: Option<Id>,
    pub manufacturer: Named,
    pub color: Named,
    pub condition: Named,
    pub location: Vec<Named>,
    pub assembly: AssemblyState,
    pub holding: Option<Holding>,
    pub set_copy_id: Option<Id>,
    pub purchase_id: Option<Id>,
    pub invoice_line_id: Option<Id>,
    pub seller: Option<String>,
    pub stated_piece_count: Option<u64>,
    pub verification: Option<verification::Status>,
    pub archived_reference: bool,
    pub incomplete: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Availability {
    Available,
    InAssembledSet,
    BrokenDamaged,
    Unknown,
}
impl InventoryRow {
    pub fn matches_availability(&self, filter: Availability) -> bool {
        let positive = |measure| matches!(measure,Measure::Known(n) if n>0);
        match filter {
            Availability::InAssembledSet => self.assembly != AssemblyState::Loose,
            Availability::Available => self.holding.is_some_and(|h| {
                positive(h.pieces.available(h.assembly))
                    || positive(h.weight_mg.available(h.assembly))
            }),
            Availability::BrokenDamaged => self
                .holding
                .is_some_and(|h| positive(h.pieces.damaged()) || positive(h.weight_mg.damaged())),
            Availability::Unknown => {
                self.holding.is_some_and(|h| {
                    [
                        h.pieces.owned(),
                        h.weight_mg.owned(),
                        h.pieces.damaged(),
                        h.weight_mg.damaged(),
                    ]
                    .contains(&Measure::Unknown)
                }) || matches!(self.target, Target::UnquantifiedSetLine(_))
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coverage {
    /// Serialized as an exact decimal string to avoid integer precision loss.
    #[serde(with = "decimal")]
    pub known_sum: u128,
    pub known_holdings: u64,
    pub unknown_holdings: u64,
}
impl Coverage {
    fn add(&mut self, value: Measure) {
        match value {
            Measure::Known(n) => {
                self.known_sum += u128::from(n);
                self.known_holdings += 1;
            }
            Measure::Unknown => self.unknown_holdings += 1,
        }
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockTotals {
    pub holdings: u64,
    pub owned_pieces: Coverage,
    pub owned_weight_mg: Coverage,
    pub available_pieces: Coverage,
    pub available_weight_mg: Coverage,
    pub damaged_pieces: Coverage,
    pub damaged_weight_mg: Coverage,
}
impl StockTotals {
    fn add(&mut self, h: Holding) {
        self.holdings += 1;
        self.owned_pieces.add(h.pieces.owned());
        self.owned_weight_mg.add(h.weight_mg.owned());
        self.available_pieces.add(h.pieces.available(h.assembly));
        self.available_weight_mg
            .add(h.weight_mg.available(h.assembly));
        self.damaged_pieces.add(h.pieces.damaged());
        self.damaged_weight_mg.add(h.weight_mg.damaged());
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryTotals {
    pub all: StockTotals,
    pub catalog_linked: StockTotals,
    pub unidentified_lots: StockTotals,
    pub other_holdings: StockTotals,
    pub known_set_copies: u64,
    pub unquantified_set_lines: u64,
    pub incomplete_records: u64,
}
impl InventoryTotals {
    pub fn add(&mut self, row: &InventoryRow) {
        if row.incomplete {
            self.incomplete_records += 1;
        }
        match row.target {
            Target::SetCopy(_) => self.known_set_copies += 1,
            Target::UnquantifiedSetLine(_) => self.unquantified_set_lines += 1,
            _ => {}
        }
        if let Some(h) = row.holding {
            self.all.add(h);
            if row.kind == Kind::MixedLot {
                self.unidentified_lots.add(h);
            } else if row.definition_id.is_some() {
                self.catalog_linked.add(h);
            } else {
                self.other_holdings.add(h);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Search {
    pub assembly: Option<AssemblyState>,
    pub archived: Option<bool>,
    pub text: String,
    pub seller: Option<String>,
    pub purchased_from: Option<String>,
    pub purchased_through: Option<String>,
    pub number: Option<String>,
    pub manufacturer_id: Option<Id>,
    pub color_id: Option<Id>,
    pub condition_id: Option<Id>,
    pub location_id: Option<Id>,
    pub include_descendants: bool,
    pub availability: Option<Availability>,
    pub kind: Option<Kind>,
    pub incomplete: Option<bool>,
    pub archived_references: Option<bool>,
    pub offset: u32,
    pub limit: u32,
}
impl Default for Search {
    fn default() -> Self {
        Self {
            assembly: None,
            archived: None,
            text: String::new(),
            seller: None,
            purchased_from: None,
            purchased_through: None,
            number: None,
            manufacturer_id: None,
            color_id: None,
            condition_id: None,
            location_id: None,
            include_descendants: true,
            availability: None,
            kind: None,
            incomplete: None,
            archived_references: None,
            offset: 0,
            limit: 50,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page {
    pub rows: Vec<InventoryRow>,
    pub total_matches: u64,
    pub totals: InventoryTotals,
    pub offset: u32,
    pub limit: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurchaseCard {
    pub id: Id,
    pub status: Status,
    pub seller: Option<String>,
    pub total_cents: Option<u64>,
    pub recorded_at: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dashboard {
    pub inventory: InventoryTotals,
    pub pending_deliveries: u64,
    pub incomplete_purchases: u64,
    pub recent_purchases: Vec<PurchaseCard>,
}

pub(crate) mod decimal {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(value: &u128, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&value.to_string())
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u128, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}
