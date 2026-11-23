//! A return's logistics are separate from its once-only physical handover.
use crate::{
    ledger::Id,
    physical::{Lifecycle, Portion},
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    Holding {
        holding_id: Id,
        expected_revision: u64,
        portion: Portion,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        sources: Vec<crate::physical::AcquisitionPortion>,
    },
    Copy(Lifecycle),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Return {
    pub id: Id,
    pub label: String,
    pub requested_on: String,
    pub notes: String,
    pub warranty_id: Option<Id>,
    pub dispatched_by: Option<Id>,
    pub acknowledged_on: Option<String>,
    pub corrected_by: Option<Id>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Plan(Return),
    Dispatch {
        return_id: Id,
        effective_on: String,
        target: Target,
    },
    Acknowledge {
        return_id: Id,
        effective_on: String,
    },
    CorrectDispatch {
        return_id: Id,
        dispatch_operation_id: Id,
        effective_on: String,
    },
}
