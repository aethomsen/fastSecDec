//! Partial affine-projective prefixes with owned constant and normalized roots.
//! No complete-cell, physical face, density or integration authority is issued.
mod admission;
mod constant;
pub mod cover;
pub mod program;
#[cfg(test)]
mod tests;

use super::super::{CellMap, coordinate_symbols};
use super::{Budget, Error, Result, SectionProposal, algebra, native, witness};
pub use admission::AffineNormalizedSection;
pub use constant::ConstantEndpoint;
use std::sync::Arc;
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    domains::rational::Rational,
};

#[derive(Debug)]
pub enum AffinePrefixStep {
    ConstantInterval {
        lower: Box<ConstantEndpoint>,
        upper: Box<ConstantEndpoint>,
        power: u32,
        image: Atom,
        diagonal: Atom,
    },
    NormalizedUpper {
        section: Arc<AffineNormalizedSection>,
        image: Atom,
        diagonal: Atom,
    },
}
impl AffinePrefixStep {
    pub fn image(&self) -> &Atom {
        match self {
            Self::ConstantInterval { image, .. } | Self::NormalizedUpper { image, .. } => image,
        }
    }
    pub fn diagonal(&self) -> &Atom {
        match self {
            Self::ConstantInterval { diagonal, .. } | Self::NormalizedUpper { diagonal, .. } => {
                diagonal
            }
        }
    }
}

/// An immutable open bijection onto exactly the retained original CAD prefix.
/// Later axes and all physical endpoint obligations remain unissued.
#[derive(Debug)]
pub struct AffineNormalizedPrefix {
    source: Arc<CellMap>,
    previous: Option<Arc<Self>>,
    step: Option<AffinePrefixStep>,
    len: usize,
}
impl AffineNormalizedPrefix {
    pub fn new(source: Arc<CellMap>, b: &Budget) -> Result<Arc<Self>> {
        b.check()?;
        if source
            .decomposition()
            .request()
            .projective_preparation()
            .is_none()
            || source.axes().is_empty()
            || source.axes().len() > 64
            || source
                .axes()
                .iter()
                .enumerate()
                .any(|(i, a)| a.unit_index() != Some(i))
        {
            return Err(Error::Unsupported("affine prefix requires an affine-projective source with 1..64 integration-only axes".into()));
        }
        if source
            .native_axes()
            .iter()
            .flat_map(|a| [&a.lower, &a.upper])
            .flatten()
            .any(|bound| bound.index_domain != symgcad::output::RootIndexDomain::Real)
        {
            return Err(Error::Unsupported(
                "affine normalized prefixes currently require distinct-real native selectors"
                    .into(),
            ));
        }
        Ok(Arc::new(Self {
            source,
            previous: None,
            step: None,
            len: 0,
        }))
    }
    pub fn source(&self) -> &Arc<CellMap> {
        &self.source
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn previous(&self) -> Option<&Arc<Self>> {
        self.previous.as_ref()
    }
    pub fn step(&self) -> Option<&AffinePrefixStep> {
        self.step.as_ref()
    }
    pub fn steps(&self) -> Vec<&AffinePrefixStep> {
        let mut steps = Vec::with_capacity(self.len);
        let mut p = self;
        while let Some(previous) = &p.previous {
            steps.push(p.step.as_ref().expect("issued prefix"));
            p = previous;
        }
        steps.reverse();
        steps
    }
    pub fn images(&self) -> Vec<Atom> {
        self.steps()
            .into_iter()
            .map(|s| s.image().clone())
            .collect()
    }
    pub fn diagonal_product(&self) -> Atom {
        self.steps()
            .into_iter()
            .map(|s| s.diagonal().clone())
            .product()
    }
    pub(super) fn slots(&self) -> Vec<(Symbol, [Rational; 2])> {
        let mut slots = self.source.unit_coordinates()[..self.len]
            .iter()
            .map(|s| (*s, [Rational::zero(), Rational::one()]))
            .collect::<Vec<_>>();
        for step in self.steps() {
            match step {
                AffinePrefixStep::ConstantInterval { lower, upper, .. } => {
                    for c in [lower, upper] {
                        if let Some(s) = c.symbol() {
                            slots.push((s, c.enclosure()));
                        }
                    }
                }
                AffinePrefixStep::NormalizedUpper { section, .. } => slots.push((
                    section.root_symbol(),
                    [
                        section.bracket().lower.clone(),
                        section.bracket().upper.clone(),
                    ],
                )),
            }
        }
        slots
    }
    fn append(self: &Arc<Self>, step: AffinePrefixStep, b: &Budget) -> Result<Arc<Self>> {
        b.check()?;
        if self.len >= self.source.axes().len() {
            return Err(Error::Invalid(
                "all original source axes already have a prefix image".into(),
            ));
        }
        Ok(Arc::new(Self {
            source: self.source.clone(),
            previous: Some(self.clone()),
            step: Some(step),
            len: self.len + 1,
        }))
    }
    /// Map the complete actual first interval using a checked positive integer.
    /// Algebraic constants are owned root slots, never caller inputs.
    pub fn constant_interval(self: &Arc<Self>, power: u32, b: &mut Budget) -> Result<Arc<Self>> {
        if !self.is_empty() {
            return Err(Error::Unsupported(
                "constant interval currently starts the original affine prefix".into(),
            ));
        }
        if power == 0 || power as usize > b.limits.max_degree {
            return Err(Error::ResourceIncomplete(
                "positive constant-interval power allowance".into(),
            ));
        }
        let (lower, upper) = constant::prepare_pair(&self.source, b)?;
        let u = Atom::var(self.source.unit_coordinates()[0]);
        let width = upper.expression() - lower.expression();
        let image = lower.expression() + &width * u.pow(power);
        let diagonal = Atom::num(power) * width * u.pow(power - 1);
        self.append(
            AffinePrefixStep::ConstantInterval {
                lower: Box::new(lower),
                upper: Box::new(upper),
                power,
                image,
                diagonal,
            },
            b,
        )
    }
    /// Add the actual next selected upper section. The initial class requires
    /// a rational constant lower endpoint and a positive normalized bracket.
    pub fn normalized_upper(
        self: &Arc<Self>,
        proposal: SectionProposal,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let section = admission::prepare(self.clone(), proposal, b)?;
        let scale = algebra::pull(
            section.scale(),
            &section.chart().parameters,
            &section.chart().parameter_images,
        );
        let width = scale * Atom::var(section.root_symbol());
        let image = Atom::num(section.lower().clone())
            + &width * Atom::var(self.source.unit_coordinates()[self.len]);
        self.append(
            AffinePrefixStep::NormalizedUpper {
                section,
                image,
                diagonal: width,
            },
            b,
        )
    }
    pub(super) fn occupied(&self) -> std::collections::BTreeSet<Symbol> {
        let mut used = coordinate_symbols(self.source.decomposition().request());
        used.extend(self.source.unit_coordinates());
        used.extend(self.slots().iter().map(|(s, _)| *s));
        for step in self.steps() {
            if let AffinePrefixStep::NormalizedUpper { section, .. } = step {
                used.extend(&section.chart().parameters);
            }
        }
        used
    }
}
