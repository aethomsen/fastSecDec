//! Actual companion forwarding into finite lower contact recursion. Global
//! maximum selection and whole-source resolution remain separate obligations.
use super::*;
mod origin;
pub use origin::CompanionDispatchOrigin;
mod driver;
pub use driver::CompanionContactDispatcher;

mod ascent;
pub use ascent::{CompanionAncestorOrder, CompanionDispatchedCenter};

mod work;
pub use work::{CompanionOuterFrontier, CompanionOuterWork};

#[cfg(test)]
mod tests;
