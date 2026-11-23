//! Local persistence adapters.
pub mod sqlite;

pub mod purchase_store;

pub mod settings_store;

pub mod catalog_store;

pub mod verification_store;

pub mod stock_store;

pub mod search_store;

pub mod attachments;
pub mod backups;
pub mod corrections;
pub mod exchange;
pub mod finance_store;
pub mod maintenance;
pub mod prices;
pub mod purge;
pub mod recovery;
pub mod snapshots;

pub mod project_store;

pub mod physical_store;

pub mod fulfilment_store;

pub mod child_cost_store;

pub mod integration_store;

pub mod cost_rollups;

pub mod enrichment_store;
