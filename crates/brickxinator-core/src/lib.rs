//! Pure inventory rules. Persistence and transport belong outside this crate.

pub mod inventory;
pub mod journal;

pub mod catalog;
pub mod finance;
pub mod ledger;
pub mod profiles;
pub mod purchases;
pub mod settings;
pub mod stock;
pub mod verification;
pub mod views;

pub mod projects;

pub mod model_import;

pub mod physical;

pub mod fulfilment;

pub mod adjustments;

pub mod supplier_returns;

pub mod child_costs;

pub mod integrations;

pub mod source_layers;
