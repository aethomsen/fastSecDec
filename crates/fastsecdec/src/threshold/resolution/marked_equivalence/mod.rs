//! Checked principal marked-power equivalence on one actual regular frame.
//!
//! This auxiliary proof never changes physical factors, C/J constructions,
//! source histories, or caller budgets. It is not a recursive-center adapter.
#[cfg(test)]
mod cusp_tests;
mod produce;
#[cfg(test)]
mod tests;
use super::{Ideal, MarkedIdeal, Poly, ResolutionHistory};
pub use produce::produce_principal_power_presentation;
use std::sync::Arc;

/// A native square-free factor whose unit classification was checked in the
/// original localization. This is not a claim of irreducible factorization.
#[derive(Clone, Debug)]
pub struct PrincipalPowerFactor {
    polynomial: Poly,
    multiplicity: usize,
    unit_ideal: Option<Ideal>,
}
impl PrincipalPowerFactor {
    pub fn polynomial(&self) -> &Poly {
        &self.polynomial
    }
    pub fn multiplicity(&self) -> usize {
        self.multiplicity
    }
    pub fn is_unit(&self) -> bool {
        self.unit_ideal.is_some()
    }
}
#[derive(Clone, Debug, Default)]
pub struct PrincipalPowerProgress {
    pub stage: &'static str,
    pub native_factor_count: usize,
    pub checked_factors: Vec<PrincipalPowerFactor>,
    pub common_power: Option<usize>,
    pub recombination_checked: bool,
    pub operations: usize,
    pub ideal_slots: usize,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrincipalPowerUnchangedReason {
    ZeroIdeal,
    MultipleGenerators,
    UnitIdeal,
    NoCommonPower,
}
#[derive(Clone, Debug)]
pub struct PrincipalPowerUnchanged {
    source: Arc<MarkedIdeal>,
    history: Arc<ResolutionHistory>,
    reason: PrincipalPowerUnchangedReason,
    progress: PrincipalPowerProgress,
}
impl PrincipalPowerUnchanged {
    pub fn source(&self) -> &Arc<MarkedIdeal> {
        &self.source
    }
    pub fn history(&self) -> &Arc<ResolutionHistory> {
        &self.history
    }
    pub fn reason(&self) -> PrincipalPowerUnchangedReason {
        self.reason
    }
    pub fn progress(&self) -> &PrincipalPowerProgress {
        &self.progress
    }
}
#[derive(Clone, Debug)]
pub enum PrincipalPowerOutcome {
    Reduced(Arc<PrincipalPowerPresentation>),
    Unchanged(PrincipalPowerUnchanged),
    /// Completed prefix evidence survives, but a retry may repeat native work.
    Incomplete {
        source: Arc<MarkedIdeal>,
        history: Arc<ResolutionHistory>,
        reason: &'static str,
        progress: PrincipalPowerProgress,
    },
}
/// Private-constructor proof of (p,d) ~ (g,d/k), from p=u*g^k with an actual
/// local unit and positive integer d divisible by k>1. This is marked
/// equivalence, not equality of the underlying unmarked ideals.
#[derive(Clone, Debug)]
pub struct PrincipalPowerPresentation {
    source: Arc<MarkedIdeal>,
    history: Arc<ResolutionHistory>,
    reduced: Arc<MarkedIdeal>,
    unit: Poly,
    base: Poly,
    power: usize,
    progress: PrincipalPowerProgress,
}
impl PrincipalPowerPresentation {
    pub fn source(&self) -> &Arc<MarkedIdeal> {
        &self.source
    }
    pub fn history(&self) -> &Arc<ResolutionHistory> {
        &self.history
    }
    pub fn reduced(&self) -> &Arc<MarkedIdeal> {
        &self.reduced
    }
    pub fn unit(&self) -> &Poly {
        &self.unit
    }
    pub fn base(&self) -> &Poly {
        &self.base
    }
    pub fn power(&self) -> usize {
        self.power
    }
    pub fn progress(&self) -> &PrincipalPowerProgress {
        &self.progress
    }
}
