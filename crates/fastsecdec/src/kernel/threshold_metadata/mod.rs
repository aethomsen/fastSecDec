//! Data-only threshold provenance. Shape validation is never mathematical proof.
//!
//! These types intentionally depend on neither symGCAD nor the optional
//! generation module. A later native completion factory owns proof admission.
mod records;
mod selected_cells;
mod serde_wire;
mod types;
mod validate;
pub use types::*;
pub use validate::{Error, Limits, Result, StructureCheckedLineage};

#[cfg(test)]
mod tests;
