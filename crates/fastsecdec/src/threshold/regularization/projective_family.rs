//! Complete finite triangular projective cell families.
//!
//! The sufficient class has constant algebraic first-axis bounds and rational
//! linear later-axis bounds. Native algebraic fields preserve selected constants;
//! native cube fans and replayed closed-box margins resolve every chart face.
mod algebra;
mod continuation;
mod density;
mod geometry;
#[cfg(test)]
mod tests;

use super::{ConvergenceStrip, meromorphic};
use crate::threshold::{
    gcad::VerifiedDecomposition,
    maps::{CellMap, regular},
};
pub use continuation::ContinuedProjectiveFamily;
use std::{ops::ControlFlow, sync::Arc};
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    domains::rational::Rational,
};

pub use regular::{Error, Result};
fn native(e: impl std::fmt::Display) -> Error {
    Error::Native(e.to_string())
}
fn unsupported(s: impl Into<String>) -> Error {
    Error::Unsupported(s.into())
}
fn invalid(s: impl Into<String>) -> Error {
    Error::Invalid(s.into())
}
fn prefactor_error(e: meromorphic::Error) -> Error {
    match e {
        meromorphic::Error::Invalid(s) => invalid(s),
        meromorphic::Error::Unsupported(s) => unsupported(s),
        meromorphic::Error::ResourceIncomplete(s) => Error::ResourceIncomplete(s.into()),
        meromorphic::Error::Cancelled => Error::Cancelled,
        meromorphic::Error::Native(s) => Error::Native(s),
    }
}

#[derive(Clone, Debug)]
pub struct Limits {
    pub algebra: regular::Limits,
    pub sectors: fastsecdec_sectors::DecompositionOptions,
    pub prefactors: meromorphic::Limits,
    pub max_charts: usize,
    pub max_field_degree: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            algebra: regular::Limits::default(),
            sectors: fastsecdec_sectors::DecompositionOptions {
                max_support_pairs: 100_000,
                max_rays: 4096,
                max_sectors: 1024,
            },
            prefactors: meromorphic::Limits::default(),
            max_charts: 4096,
            max_field_degree: 16,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Progress {
    Cell(usize),
    Half { cell: usize, half: usize },
    Chart(usize),
    Continue(usize),
}
struct Work<'a> {
    limits: &'a Limits,
    budget: regular::Budget,
    observer: &'a mut dyn FnMut(Progress) -> ControlFlow<()>,
}
impl Work<'_> {
    fn check(&self) -> Result<()> {
        self.budget.check()
    }
    fn emit(&mut self, p: Progress) -> Result<()> {
        self.check()?;
        if (self.observer)(p).is_break() {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }
    fn atom(&self, a: &Atom) -> Result<()> {
        self.check()?;
        let mut n = 0usize;
        a.visitor(&mut |_| {
            n = n.saturating_add(1);
            n <= self.limits.prefactors.source_nodes
        });
        if n > self.limits.prefactors.source_nodes {
            Err(Error::ResourceIncomplete(
                "projective expression node allowance".into(),
            ))
        } else {
            Ok(())
        }
    }
}

/// Exact original prepared role. Polynomial numerator bodies remain separate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FactorOrigin {
    Coordinate(usize),
    Factor { term: usize, factor: usize },
    Measure,
    BoundDenominator(usize),
}

/// A replayed strict margin on the full cube and a selected primitive-root box.
#[derive(Debug)]
pub struct ClosedUnit {
    expression: Atom,
    polynomial: symgcad::algebra::Poly,
    constraints: Vec<symgcad::algebra::Poly>,
    sign: i8,
    margin: Rational,
    certificate: symgcad::domain::DomainCertificate,
}
impl ClosedUnit {
    pub fn expression(&self) -> &Atom {
        &self.expression
    }
    pub fn sign(&self) -> i8 {
        self.sign
    }
    pub fn margin(&self) -> &Rational {
        &self.margin
    }
    pub fn polynomial(&self) -> &symgcad::algebra::Poly {
        &self.polynomial
    }
    pub fn constraints(&self) -> &[symgcad::algebra::Poly] {
        &self.constraints
    }
    pub fn certificate(&self) -> &symgcad::domain::DomainCertificate {
        &self.certificate
    }
}
#[derive(Debug)]
pub struct FactorReceipt {
    origin: FactorOrigin,
    source: Atom,
    powers: Vec<i64>,
    numerator: ClosedUnit,
    denominator: ClosedUnit,
}
impl FactorReceipt {
    pub fn origin(&self) -> FactorOrigin {
        self.origin
    }
    pub fn source(&self) -> &Atom {
        &self.source
    }
    pub fn powers(&self) -> &[i64] {
        &self.powers
    }
    pub fn numerator(&self) -> &ClosedUnit {
        &self.numerator
    }
    pub fn denominator(&self) -> &ClosedUnit {
        &self.denominator
    }
    pub fn sign(&self) -> i8 {
        self.numerator.sign * self.denominator.sign
    }
    pub fn magnitude(&self) -> Atom {
        Atom::num(i64::from(self.sign())) * &self.numerator.expression
            / &self.denominator.expression
    }
}
pub struct CellGeometry {
    source: Arc<CellMap>,
    sections: Vec<Arc<regular::RegularSection>>,
    selected_constants: Vec<Atom>,
    algebra: algebra::Field,
}
impl CellGeometry {
    pub fn source(&self) -> &Arc<CellMap> {
        &self.source
    }
    pub fn sections(&self) -> &[Arc<regular::RegularSection>] {
        &self.sections
    }
    pub fn selected_constants(&self) -> &[Atom] {
        &self.selected_constants
    }
    pub fn constant_field(&self) -> &symbolica::domains::algebraic::AlgebraicContext {
        self.algebra.context()
    }
    pub fn primitive_interval(&self) -> &[Rational; 2] {
        self.algebra.interval()
    }
}
pub struct NormalizedTerm {
    term: usize,
    prefactor: Atom,
    regular: Atom,
    powers: Vec<Atom>,
}
impl NormalizedTerm {
    pub fn source_index(&self) -> usize {
        self.term
    }
    pub fn prefactor(&self) -> &Atom {
        &self.prefactor
    }
    pub fn regular(&self) -> &Atom {
        &self.regular
    }
    pub fn powers(&self) -> &[Atom] {
        &self.powers
    }
}
pub struct ResolvedChart {
    geometry: Arc<CellGeometry>,
    half: usize,
    sector: usize,
    map: fastsecdec_sectors::SectorMap,
    orientation: i8,
    images: Vec<Atom>,
    measure: Atom,
    receipts: Vec<FactorReceipt>,
    terms: Vec<NormalizedTerm>,
}
impl ResolvedChart {
    pub fn geometry(&self) -> &Arc<CellGeometry> {
        &self.geometry
    }
    pub fn cell_index(&self) -> usize {
        self.geometry.source.cell_index()
    }
    /// Complete half-cube ordinal followed by native sector ordinal.
    pub fn endpoint_path(&self) -> [usize; 2] {
        [self.half, self.sector]
    }
    pub fn sector_map(&self) -> &fastsecdec_sectors::SectorMap {
        &self.map
    }
    /// Signed determinant in prepared source order. Integration uses the
    /// separately retained positive measure.
    pub fn orientation(&self) -> i8 {
        self.orientation
    }
    /// Exact images in the original request's prepared-coordinate order.
    pub fn images(&self) -> &[Atom] {
        &self.images
    }
    pub fn positive_measure(&self) -> &Atom {
        &self.measure
    }
    pub fn factors(&self) -> &[FactorReceipt] {
        &self.receipts
    }
    pub fn terms(&self) -> &[NormalizedTerm] {
        &self.terms
    }
}
pub struct ResolvedProjectiveFamily {
    owner: Arc<VerifiedDecomposition>,
    coordinates: Vec<Symbol>,
    regulators: Vec<Symbol>,
    charts: Vec<ResolvedChart>,
    strip: ConvergenceStrip,
    witness: meromorphic::MeromorphicWitness,
}
impl ResolvedProjectiveFamily {
    pub fn admit(
        owner: Arc<VerifiedDecomposition>,
        coordinates: Vec<Symbol>,
        limits: Limits,
        mut observer: impl FnMut(Progress) -> ControlFlow<()>,
    ) -> Result<Self> {
        density::admit(owner, coordinates, limits, &mut observer)
    }
    pub fn owner(&self) -> &Arc<VerifiedDecomposition> {
        &self.owner
    }
    pub fn coordinates(&self) -> &[Symbol] {
        &self.coordinates
    }
    pub fn regulators(&self) -> &[Symbol] {
        &self.regulators
    }
    pub fn runtime_parameters(&self) -> &[Symbol] {
        &[]
    }
    pub fn charts(&self) -> &[ResolvedChart] {
        &self.charts
    }
    pub fn convergence_strip(&self) -> &ConvergenceStrip {
        &self.strip
    }
    pub fn prefactor_witness(&self) -> &meromorphic::MeromorphicWitness {
        &self.witness
    }
    /// Constant native algebraic expressions use native transcendental support;
    /// no process-local sampled-root callback is introduced by this issuer.
    pub fn callback_scope(&self) -> crate::kernel::algebraic::Scope {
        Default::default()
    }
}
