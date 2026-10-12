//! Complete source-owned restrictions of an admitted affine prefix.
//! A cover alone issues neither its next root nor a physical density.
mod equation;
pub mod next;
pub mod program;
#[cfg(test)]
mod tests;
mod transport;
mod witness;

use super::{AffineNormalizedPrefix, AffinePrefixStep, Budget, Error, Result, algebra, native};
use crate::threshold::maps::regular::SectionSide;
use fastsecdec_sectors::DecompositionOptions;
use std::sync::Arc;
use symbolica::atom::{Atom, AtomCore, Symbol};
use symbolica::domains::rational::Rational;
use symgcad::output::RootBound;

pub use equation::{RestrictedRootEquation, UnitReceipt};
pub use transport::{
    AffinePrefixRestriction, DivisorRewrite, DivisorTarget, FactoredDivisorProposal,
};
pub use witness::InteriorWitness;

/// One equation of the genuine previously selected coefficient tower.
/// The dependent symbol is an existing callback output, never a new input.
#[derive(Debug)]
pub struct OwnedRelation {
    dependent: Symbol,
    equation: Atom,
}
impl OwnedRelation {
    pub fn dependent(&self) -> Symbol {
        self.dependent
    }
    pub fn equation(&self) -> &Atom {
        &self.equation
    }
}

/// All native children replace the same parent prefix, with disjoint open
/// interiors and shared boundary seams. No public unchecked constructor.
pub struct AffinePrefixCover {
    parent: Arc<AffineNormalizedPrefix>,
    side: SectionSide,
    bound: RootBound,
    root: Symbol,
    original: Atom,
    pulled: Atom,
    slots: Vec<(Symbol, [Rational; 2])>,
    dependent_symbols: Vec<Symbol>,
    relations: Vec<OwnedRelation>,
    divisors: Vec<Atom>,
    divisor_targets: Vec<DivisorTarget>,
    rewrites: Vec<transport::DivisorRewrite>,
    half_counts: Vec<usize>,
    leaves: Vec<AffinePrefixRestriction>,
}
impl AffinePrefixCover {
    pub fn prepare_next_bound(
        parent: Arc<AffineNormalizedPrefix>,
        side: SectionSide,
        options: &DecompositionOptions,
        budget: &mut Budget,
    ) -> Result<Arc<Self>> {
        Self::prepare_with_divisors(parent, side, &[], options, budget)
    }
    /// Proposals may factor the actual leading/discriminant expression modulo
    /// the retained tower. Exact native recombination is mandatory; all factors
    /// are retained, including constants and presentation coefficients.
    pub fn prepare_with_divisors(
        parent: Arc<AffineNormalizedPrefix>,
        side: SectionSide,
        proposals: &[FactoredDivisorProposal],
        options: &DecompositionOptions,
        budget: &mut Budget,
    ) -> Result<Arc<Self>> {
        transport::prepare(parent, side, proposals, options, budget)
    }
    pub fn parent(&self) -> &Arc<AffineNormalizedPrefix> {
        &self.parent
    }
    pub fn side(&self) -> SectionSide {
        self.side
    }
    pub fn next_bound(&self) -> &RootBound {
        &self.bound
    }
    pub fn root_symbol(&self) -> Symbol {
        self.root
    }
    pub fn original_polynomial(&self) -> &Atom {
        &self.original
    }
    pub fn total_pullback(&self) -> &Atom {
        &self.pulled
    }
    pub fn slots(&self) -> &[(Symbol, [Rational; 2])] {
        &self.slots
    }
    pub fn dependent_symbols(&self) -> &[Symbol] {
        &self.dependent_symbols
    }
    pub fn relations(&self) -> &[OwnedRelation] {
        &self.relations
    }
    pub fn divisors(&self) -> &[Atom] {
        &self.divisors
    }
    pub fn rewrites(&self) -> &[DivisorRewrite] {
        &self.rewrites
    }
    pub fn half_counts(&self) -> &[usize] {
        &self.half_counts
    }
    pub fn leaves(&self) -> &[AffinePrefixRestriction] {
        &self.leaves
    }
    pub fn leaf(self: &Arc<Self>, index: usize) -> Result<AffinePrefixLeaf> {
        if index >= self.leaves.len() {
            return Err(Error::Invalid("prefix cover leaf index".into()));
        }
        Ok(AffinePrefixLeaf {
            cover: self.clone(),
            index,
        })
    }
}

/// An immutable indexed handle retains the complete source-owned cover.
#[derive(Clone)]
pub struct AffinePrefixLeaf {
    cover: Arc<AffinePrefixCover>,
    index: usize,
}
impl AffinePrefixLeaf {
    pub fn cover(&self) -> &Arc<AffinePrefixCover> {
        &self.cover
    }
    pub fn index(&self) -> usize {
        self.index
    }
    pub fn restriction(&self) -> &AffinePrefixRestriction {
        &self.cover.leaves[self.index]
    }
    pub fn prepare_equation(
        &self,
        margin: Rational,
        budget: &mut Budget,
    ) -> Result<Arc<RestrictedRootEquation>> {
        equation::prepare(self.clone(), margin, budget)
    }
    /// A rational original-source point is checked against all actual selected
    /// parent bounds and native certified inverse enclosures for this leaf.
    pub fn interior_witness(
        &self,
        point: Vec<Rational>,
        budget: &mut Budget,
    ) -> Result<InteriorWitness> {
        witness::prepare(self.clone(), point, budget)
    }
}

fn relation_inventory(
    parent: &AffineNormalizedPrefix,
    budget: &Budget,
) -> Result<Vec<OwnedRelation>> {
    let mut result = Vec::new();
    for step in parent.steps() {
        budget.check()?;
        match step {
            AffinePrefixStep::ConstantInterval { lower, upper, .. } => {
                for endpoint in [lower, upper] {
                    if let (Some(dependent), Some(section)) =
                        (endpoint.symbol(), endpoint.section())
                    {
                        // RegularSection::equation already uses the retained
                        // CellMap source roles, rather than raw CAD aliases.
                        let source = section.source().axes()[section.axis()].symbol();
                        result.push(OwnedRelation {
                            dependent,
                            equation: algebra::pull(
                                section.equation(),
                                &[source],
                                &[Atom::var(dependent)],
                            ),
                        });
                    }
                }
            }
            AffinePrefixStep::NormalizedUpper { section, .. } => result.push(OwnedRelation {
                dependent: section.root_symbol(),
                equation: algebra::pull(
                    section.equation(),
                    &section.chart().parameters,
                    &section.chart().parameter_images,
                ),
            }),
        }
    }
    Ok(result)
}
