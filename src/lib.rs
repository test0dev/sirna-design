//! siRNA design engine — Rust port of the local verification prototype.
//!
//! Modules land incrementally: `rules` → `tm` → `design` → `ensembl` → `offtarget` → `api`.

pub mod error;

pub use error::Error;
