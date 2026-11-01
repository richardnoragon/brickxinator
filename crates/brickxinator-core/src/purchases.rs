//! Purchase metadata is provenance. Only an explicit receipt introduces stock.
use crate::{
    inventory::{AssemblyState, Measure},
    ledger::{Body, Id, Operation, ReceivedCopy, ReceivedLine, ReceivedStock},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, error::Error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Draft,
    Ordered,
    Shipped,
    Received,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineKind {
    LooseBatch,
    MixedLot,
    Set,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SellerType {
    PrivateSeller,
    BusinessSeller,
    EbayPrivateSeller,
    EbayBusinessSeller,
    Marketplace,
    RetailStore,
    BrickDealer,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaxPresentation {
    Included,
    Unknown,
    NotApplicable,
    AddedSeparately,
}

/// Calendar date in YYYY-MM-DD form, without inventing a time or timezone.
pub fn valid_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes
            .iter()
            .enumerate()
            .any(|(i, b)| i != 4 && i != 7 && !b.is_ascii_digit())
    {
        return false;
    }
    let Ok(year) = value[..4].parse::<u32>() else {
        return false;
    };
    let Ok(month) = value[5..7].parse::<u32>() else {
        return false;
    };
    let Ok(day) = value[8..].parse::<u32>() else {
        return false;
    };
    let max = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 0,
    };
    year > 0 && day > 0 && day <= max
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvoiceLine {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
    pub id: Id,
    pub kind: LineKind,
    pub description: Option<String>,
    pub set_number: Option<String>,
    pub copy_count: Option<u32>,
    pub pieces: Measure,
    pub weight_mg: Measure,
    /// The entire invoice-line price, never an inferred per-copy price.
    pub paid_total_cents: Option<u64>,
    pub assembly: AssemblyState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    pub seller: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seller_type: Option<SellerType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seller_country: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purchased_on: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_presentation: Option<TaxPresentation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_cents: Option<u64>,
    pub order_number: Option<String>,
    pub source_url: Option<String>,
    pub notes: String,
    pub shipping_cents: Option<u64>,
    pub lines: Vec<InvoiceLine>,
}

impl Draft {
    pub fn validate(&self) -> Result<(), PurchaseError> {
        if self
            .purchased_on
            .as_deref()
            .is_some_and(|date| !valid_date(date))
            || self
                .seller_country
                .as_ref()
                .is_some_and(|country| country.len() > 200 || country.chars().any(char::is_control))
        {
            return Err(PurchaseError::InvalidMetadata);
        }
        let mut ids = BTreeSet::new();
        for line in &self.lines {
            if !ids.insert(line.id) {
                return Err(PurchaseError::DuplicateLine);
            }
            if line.kind != LineKind::Set
                && (line.copy_count.is_some()
                    || line.set_number.is_some()
                    || line.assembly != AssemblyState::Loose)
            {
                return Err(PurchaseError::InvalidLineFields);
            }
        }
        Ok(())
    }

    /// EUR only for alpha. Incomplete amounts stay unknown, including shipping.
    pub fn total_cents(&self) -> Option<u64> {
        self.lines
            .iter()
            .try_fold(self.shipping_cents?, |total, line| {
                total.checked_add(line.paid_total_cents?)
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Purchase {
    pub id: Id,
    #[serde(default, skip_serializing_if = "is_false")]
    pub archived: bool,
    pub revision: u64,
    pub status: Status,
    pub draft: Draft,
}
fn is_false(value: &bool) -> bool {
    !value
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PurchaseCommand {
    pub id: Id,
    pub request_key: String,
    pub actor: String,
    pub reason: String,
    pub body: PurchaseAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum PurchaseAction {
    Correct {
        purchase_id: Id,
        expected_revision: u64,
        draft: Draft,
        preview: CorrectionPreview,
    },
    Archive {
        purchase_id: Id,
        expected_revision: u64,
        archived: bool,
    },
    Create {
        purchase_id: Id,
        draft: Draft,
    },
    Save {
        purchase_id: Id,
        expected_revision: u64,
        status: Status,
        draft: Draft,
    },
    Receive {
        purchase_id: Id,
        expected_revision: u64,
    },
}

impl PurchaseAction {
    pub fn purchase_id(&self) -> Id {
        match *self {
            Self::Correct { purchase_id, .. } => purchase_id,
            Self::Archive { purchase_id, .. } => purchase_id,
            Self::Create { purchase_id, .. }
            | Self::Save { purchase_id, .. }
            | Self::Receive { purchase_id, .. } => purchase_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrectionEffect {
    pub holding_id: Id,
    pub before: crate::journal::Holding,
    pub after: crate::journal::Holding,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopyAddition {
    pub invoice_line_id: Id,
    pub copies: Vec<ReceivedCopy>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrectionPreview {
    pub purchase_revision: u64,
    pub inventory_sequence: u64,
    pub effects: Vec<CorrectionEffect>,
    pub copy_additions: Vec<CopyAddition>,
    pub warnings: Vec<String>,
}

impl Purchase {
    pub fn save(
        &self,
        expected_revision: u64,
        status: Status,
        draft: Draft,
    ) -> Result<Self, PurchaseError> {
        self.check_revision(expected_revision)?;
        if self.status == Status::Received {
            return Err(PurchaseError::AlreadyReceived);
        }
        if status == Status::Received {
            return Err(PurchaseError::ExplicitReceiptRequired);
        }
        draft.validate()?;
        Ok(Self {
            id: self.id,
            archived: self.archived,
            revision: self
                .revision
                .checked_add(1)
                .ok_or(PurchaseError::OutOfRange)?,
            status,
            draft,
        })
    }

    pub fn check_revision(&self, expected: u64) -> Result<(), PurchaseError> {
        if self.revision == expected {
            Ok(())
        } else {
            Err(PurchaseError::StaleRevision)
        }
    }

    pub fn receipt_operation(&self, command: &PurchaseCommand) -> Result<Operation, PurchaseError> {
        let PurchaseAction::Receive {
            purchase_id,
            expected_revision,
        } = command.body
        else {
            return Err(PurchaseError::ExplicitReceiptRequired);
        };
        if purchase_id != self.id {
            return Err(PurchaseError::MissingPurchase);
        }
        if self.status == Status::Received {
            return Err(PurchaseError::AlreadyReceived);
        }
        if self.status == Status::Cancelled {
            return Err(PurchaseError::Cancelled);
        }
        self.check_revision(expected_revision)?;
        self.draft.validate()?;
        if self.draft.lines.is_empty() {
            return Err(PurchaseError::EmptyReceipt);
        }
        // Bound materialization, not metadata entry. Large drafts may still save.
        let count: u64 = self.draft.lines.iter().try_fold(0_u64, |count, line| {
            let added = if line.kind == LineKind::Set {
                u64::from(line.copy_count.unwrap_or(0))
            } else {
                1
            };
            count
                .checked_add(added)
                .ok_or(PurchaseError::ReceiptTooLarge)
        })?;
        if count > 10_000 {
            return Err(PurchaseError::ReceiptTooLarge);
        }
        let lines = self
            .draft
            .lines
            .iter()
            .map(|line| {
                let stock = if line.kind == LineKind::Set {
                    let label = line
                        .set_number
                        .as_deref()
                        .filter(|s| !s.trim().is_empty())
                        .unwrap_or("Set");
                    ReceivedStock::Sets {
                        copies: (0..line.copy_count.unwrap_or(0))
                            .map(|i| ReceivedCopy {
                                id: Id::new(),
                                label: format!("{label}_{:02}", i + 1),
                                assembly: line.assembly,
                            })
                            .collect(),
                    }
                } else {
                    ReceivedStock::Batch {
                        holding_id: Id::new(),
                        pieces: line.pieces,
                        weight_mg: line.weight_mg,
                    }
                };
                ReceivedLine {
                    invoice_line_id: line.id,
                    stock,
                }
            })
            .collect();
        Ok(Operation {
            id: command.id,
            request_key: command.request_key.clone(),
            actor: command.actor.clone(),
            reason: if command.reason.trim().is_empty() {
                "Whole-purchase receipt".into()
            } else {
                command.reason.clone()
            },
            body: Body::Receive {
                purchase_id: self.id,
                lines,
            },
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurchaseError {
    InvalidMetadata,
    DuplicateLine,
    InvalidLineFields,
    StaleRevision,
    AlreadyReceived,
    ExplicitReceiptRequired,
    MissingPurchase,
    Cancelled,
    EmptyReceipt,
    ReceiptTooLarge,
    OutOfRange,
}
impl fmt::Display for PurchaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidMetadata => {
                "purchase date must be a valid YYYY-MM-DD calendar date; country must be plain text"
            }
            Self::DuplicateLine => "invoice line IDs must be distinct",
            Self::InvalidLineFields => "set-copy fields are only valid on set lines",
            Self::StaleRevision => "purchase changed since this command was prepared",
            Self::AlreadyReceived => {
                "purchase was already received; reconciliation is required for later edits"
            }
            Self::ExplicitReceiptRequired => {
                "received status requires the explicit receipt command"
            }
            Self::MissingPurchase => "purchase does not exist or does not match the command",
            Self::Cancelled => "cancelled purchases cannot be received",
            Self::EmptyReceipt => "an empty purchase can be saved but cannot be received",
            Self::ReceiptTooLarge => {
                "receipt exceeds the current 10000-entity materialization limit"
            }
            Self::OutOfRange => "purchase revision is out of range",
        })
    }
}
impl Error for PurchaseError {}
