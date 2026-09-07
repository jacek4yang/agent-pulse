//! Persistence: schema-versioned JSON store.

pub mod json;

pub use json::{JsonStore, SCHEMA_VERSION, StoreDocument, bound_history};
