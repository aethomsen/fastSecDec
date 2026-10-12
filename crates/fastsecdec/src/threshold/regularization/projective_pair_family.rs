//! Complete source-bound two-dimensional positive-projective families.
//!
//! Proposals carry no authority. The issuer checks original selected bounds,
//! complete interval/fibre replacements, normalized root branches and every
//! closed factor before exposing symbolic continuation or publication.
mod admission;
mod continuation;
mod geometry;
mod leaves;
mod numerator;
mod pair;
#[cfg(test)]
pub(crate) mod tests;

use super::{ConvergenceStrip, meromorphic, projective_family};
use crate::{
    kernel::algebraic::{RootProgram, Scope},
    threshold::gcad::PositiveProjectiveQuotient,
};
use std::{ops::ControlFlow, sync::Arc};
use symbolica::{
    atom::{Atom, Symbol},
    domains::rational::Rational,
    evaluate::OptimizationSettings,
};
use symgcad::output::RootBound;

pub use crate::threshold::maps::regular::{BracketProposal, Error, Result, SectionSide};
pub use continuation::ContinuedProjectivePairFamily;
pub use projective_family::{
    ClosedUnit, FactorOrigin, FactorReceipt, Limits, NormalizedTerm, Progress,
};
use projective_family::{Work, invalid, native, prefactor_error, unsupported};

/// A selected constant from an actual first-axis boundary. No caller Atom is
/// substituted for this record and no root becomes a free numerical parameter.
#[derive(Clone, Copy, Debug)]
pub struct ConstantReference {
    pub cell: usize,
    pub side: SectionSide,
}

/// Complete one-to-one maps of a retained finite interval. Powers are checked
/// positive integers; reflections retain their signed orientation.
#[derive(Clone, Copy, Debug)]
pub enum PrefixShape {
    Power { exponent: u32, reversed: bool },
    TwoEnded { exponent: u32 },
    Ray,
}

/// One normalized, independently regular root. Coefficients may depend on the
/// first chart coordinate and retained selected constants, never later roots.
/// The witness is an interior rational chart point whose original fibre is QQ.
#[derive(Clone, Debug)]
pub struct RootProposal {
    pub role: Symbol,
    pub equation: Atom,
    pub bracket: BracketProposal,
    pub witness: Rational,
}

#[derive(Clone, Debug)]
pub enum FibreProposal {
    /// Actual lower and optional upper selected-bound images. Admission checks
    /// polynomial identities, original ordinals and normalized-root continuity.
    Native {
        lower: Atom,
        upper: Option<Atom>,
        reversed: bool,
    },
    /// Both resulting fibres are always retained. Only an original (0,infinity)
    /// fibre and a finite positive closed centre can enter this constructor.
    PositiveCentre { centre: Atom },
}

/// A compact proposed spelling of one actual singular factor pullback.
/// Native cleared crossproduct reduction against the admitted root equations
/// must prove equality; the original density and denominator proofs are kept.
#[derive(Clone, Debug)]
pub struct FactorRewrite {
    pub fibre: usize,
    pub term: usize,
    pub factor: usize,
    pub expression: Atom,
}

#[derive(Clone, Debug)]
pub struct PrefixPiece {
    /// Contiguous rational fractions of the actual finite source interval.
    /// A ray requires the single inventory entry [0,1].
    pub fractions: [Rational; 2],
    pub shape: PrefixShape,
    pub roots: Vec<RootProposal>,
    pub fibre: FibreProposal,
    pub rewrites: Vec<FactorRewrite>,
}

#[derive(Clone, Debug)]
pub struct CellProposal {
    pub cell: usize,
    pub constants: Vec<ConstantReference>,
    pub pieces: Vec<PrefixPiece>,
}

/// Retained original selected branch and exact source-pullback association.
#[derive(Clone)]
pub struct BranchReceipt {
    side: SectionSide,
    bound: RootBound,
    original: Atom,
    image: Atom,
    source_fibre: Option<Rational>,
    selected_anchor: Atom,
}
impl BranchReceipt {
    pub fn side(&self) -> SectionSide {
        self.side
    }
    pub fn bound(&self) -> &RootBound {
        &self.bound
    }
    pub fn original_polynomial(&self) -> &Atom {
        &self.original
    }
    pub fn image(&self) -> &Atom {
        &self.image
    }
    pub fn source_fibre(&self) -> Option<&Rational> {
        self.source_fibre.as_ref()
    }
    pub fn selected_anchor(&self) -> &Atom {
        &self.selected_anchor
    }
}

/// A checked normalized equation with its source-cell/prefix contract. The
/// detached RootProgram is arithmetic transport, not a reconstructed owner.
pub struct NormalizedRootSection {
    owner: Arc<PositiveProjectiveQuotient>,
    cell: usize,
    prefix: Atom,
    parameter: Symbol,
    role: Symbol,
    equation: Atom,
    equation_denominator: ClosedUnit,
    coefficients: Vec<Atom>,
    bracket: BracketProposal,
    orientation: i8,
    margin: ClosedUnit,
    endpoints: [ClosedUnit; 2],
    branches: Vec<BranchReceipt>,
    program: Arc<RootProgram>,
    tag: Symbol,
}
impl NormalizedRootSection {
    pub fn owner(&self) -> &Arc<PositiveProjectiveQuotient> {
        &self.owner
    }
    pub fn cell_index(&self) -> usize {
        self.cell
    }
    pub fn axis(&self) -> usize {
        1
    }
    pub fn prefix_image(&self) -> &Atom {
        &self.prefix
    }
    pub fn parameter(&self) -> Symbol {
        self.parameter
    }
    pub fn root_symbol(&self) -> Symbol {
        self.role
    }
    pub fn equation(&self) -> &Atom {
        &self.equation
    }
    pub fn equation_denominator(&self) -> &ClosedUnit {
        &self.equation_denominator
    }
    pub fn coefficients(&self) -> &[Atom] {
        &self.coefficients
    }
    pub fn bracket(&self) -> &BracketProposal {
        &self.bracket
    }
    pub fn orientation(&self) -> i8 {
        self.orientation
    }
    pub fn derivative_margin(&self) -> &ClosedUnit {
        &self.margin
    }
    pub fn endpoints(&self) -> &[ClosedUnit; 2] {
        &self.endpoints
    }
    pub fn branches(&self) -> &[BranchReceipt] {
        &self.branches
    }
    pub fn root_program(&self) -> &Arc<RootProgram> {
        &self.program
    }
    pub fn tag(&self) -> Symbol {
        self.tag
    }
}

/// Polynomial zeros are permitted. Only the transported denominator has a
/// closed-unit obligation; this is deliberately separate from FactorReceipt.
pub struct NumeratorTransport {
    term: usize,
    source: Atom,
    factor_indices: Vec<usize>,
    template_roles: Vec<Symbol>,
    finite_images: Vec<Atom>,
    degrees: Vec<u32>,
    template: Atom,
    regular: Atom,
    powers: Vec<i64>,
    denominators: Vec<FactorReceipt>,
}
impl NumeratorTransport {
    pub fn source_index(&self) -> usize {
        self.term
    }
    pub fn source(&self) -> &Atom {
        &self.source
    }
    pub fn factor_indices(&self) -> &[usize] {
        &self.factor_indices
    }
    pub fn template_roles(&self) -> &[Symbol] {
        &self.template_roles
    }
    pub fn finite_images(&self) -> &[Atom] {
        &self.finite_images
    }
    pub fn degrees(&self) -> &[u32] {
        &self.degrees
    }
    pub fn homogeneous_template(&self) -> &Atom {
        &self.template
    }
    pub fn regular(&self) -> &Atom {
        &self.regular
    }
    pub fn powers(&self) -> &[i64] {
        &self.powers
    }
    pub fn denominators(&self) -> &[FactorReceipt] {
        &self.denominators
    }
}

pub struct FactorRewriteReceipt {
    origin: FactorOrigin,
    original: Atom,
    proposed: Atom,
    crossproduct: Atom,
}
impl FactorRewriteReceipt {
    pub fn origin(&self) -> FactorOrigin {
        self.origin
    }
    pub fn original_pullback(&self) -> &Atom {
        &self.original
    }
    pub fn proposed(&self) -> &Atom {
        &self.proposed
    }
    pub fn cleared_crossproduct(&self) -> &Atom {
        &self.crossproduct
    }
}

pub struct ResolvedPairChart {
    cell: usize,
    path: [usize; 3],
    coordinate_order: Vec<Symbol>,
    source_axes: Vec<symgcad::output::Axis>,
    sections: Vec<Arc<NormalizedRootSection>>,
    branches: Arc<Vec<BranchReceipt>>,
    coverage: Arc<Vec<FactorReceipt>>,
    rewrites: Arc<Vec<FactorRewriteReceipt>>,
    root_inputs: Vec<(Symbol, Vec<Atom>)>,
    map: fastsecdec_sectors::SectorMap,
    orientation: i8,
    images: Vec<Atom>,
    measure: Atom,
    receipts: Vec<FactorReceipt>,
    numerators: Vec<NumeratorTransport>,
    terms: Vec<NormalizedTerm>,
    roots: Scope,
}
impl ResolvedPairChart {
    pub fn cell_index(&self) -> usize {
        self.cell
    }
    /// Source interval/fibre piece, complete half-cube, native sector ordinal.
    pub fn endpoint_path(&self) -> [usize; 3] {
        self.path
    }
    pub fn coordinate_order(&self) -> &[Symbol] {
        &self.coordinate_order
    }
    pub fn source_axes(&self) -> &[symgcad::output::Axis] {
        &self.source_axes
    }
    pub fn sections(&self) -> &[Arc<NormalizedRootSection>] {
        &self.sections
    }
    pub fn branches(&self) -> &[BranchReceipt] {
        &self.branches
    }
    pub fn coverage_receipts(&self) -> &[FactorReceipt] {
        &self.coverage
    }
    pub fn rewrite_receipts(&self) -> &[FactorRewriteReceipt] {
        &self.rewrites
    }
    /// Actual callback tag and half/fan pullback of its coefficient schema.
    pub fn root_inputs(&self) -> &[(Symbol, Vec<Atom>)] {
        &self.root_inputs
    }
    pub fn sector_map(&self) -> &fastsecdec_sectors::SectorMap {
        &self.map
    }
    pub fn orientation(&self) -> i8 {
        self.orientation
    }
    /// Actual anchor-ratio images, in the prepared source coordinate order.
    pub fn images(&self) -> &[Atom] {
        &self.images
    }
    pub fn positive_measure(&self) -> &Atom {
        &self.measure
    }
    pub fn factors(&self) -> &[FactorReceipt] {
        &self.receipts
    }
    pub fn numerator_transport(&self) -> &[NumeratorTransport] {
        &self.numerators
    }
    pub fn terms(&self) -> &[NormalizedTerm] {
        &self.terms
    }
    pub fn roots(&self) -> &Scope {
        &self.roots
    }
}

pub struct ResolvedProjectivePairFamily {
    owner: Arc<PositiveProjectiveQuotient>,
    coordinates: Vec<Symbol>,
    regulators: Vec<Symbol>,
    charts: Vec<ResolvedPairChart>,
    strip: ConvergenceStrip,
    witness: meromorphic::MeromorphicWitness,
    roots: Scope,
}
impl ResolvedProjectivePairFamily {
    pub fn admit(
        owner: Arc<PositiveProjectiveQuotient>,
        coordinates: Vec<Symbol>,
        proposals: Vec<CellProposal>,
        limits: Limits,
        settings: OptimizationSettings,
        mut observer: impl FnMut(Progress) -> ControlFlow<()>,
    ) -> Result<Self> {
        admission::admit(
            owner,
            coordinates,
            proposals,
            limits,
            settings,
            &mut observer,
        )
    }
    /// Exact native selected constant for proposal construction only. This does
    /// not issue a map or a complete-cell certificate.
    pub fn selected_constant(
        owner: &PositiveProjectiveQuotient,
        reference: ConstantReference,
        limits: &Limits,
    ) -> Result<Atom> {
        geometry::selected_constant(owner, reference, limits)
    }
    pub fn owner(&self) -> &Arc<PositiveProjectiveQuotient> {
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
    pub fn charts(&self) -> &[ResolvedPairChart] {
        &self.charts
    }
    pub fn convergence_strip(&self) -> &ConvergenceStrip {
        &self.strip
    }
    pub fn prefactor_witness(&self) -> &meromorphic::MeromorphicWitness {
        &self.witness
    }
    pub fn callback_scope(&self) -> Scope {
        self.roots.clone()
    }
}
