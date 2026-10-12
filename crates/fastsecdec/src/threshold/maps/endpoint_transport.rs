//! Exact endpoint coordinate/factor transport on an issued open cell.
//!
//! Singular geometry is restricted to exact QQ coefficients; symbolic density
//! numerator/prefactor coefficients remain lazy source records.
//! Complete transport means complete identities and term accounting, never
//! closed-face regularity. Polynomial numerator bodies remain lazy and pending;
//! in particular evaluating a divergent numerator at infinity and multiplying
//! by zero is not licensed by the extracted powers.
mod chart;
mod cover;
pub use cover::{
    ChildBoundary, CoverLeaf, EndpointChartCover, HalfCube, IssuedEndpointChart, SplitInterval,
};
mod clearing;
pub mod closed;
pub mod factors;
pub mod numerator;
#[cfg(test)]
mod tests;
pub use super::regular::{Error, Result};
use super::{
    normalized::{NormalizedPrefix, NormalizedSection, PrefixStep},
    regular::{Budget, Limits as AlgebraLimits},
};
use crate::parametric::FactorRole;
pub use chart::{EndpointChart, TermPowers};
pub use clearing::{ClearedFactor, ExtractedFactor};
use clearing::{Ring, pull};
use fastsecdec_sectors::{
    Decomposition, DecompositionOptions, ParametricDomain, PolynomialSupport, SectorError,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::ControlFlow,
    sync::Arc,
};
use symbolica::atom::{Atom, Symbol};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaceSide {
    Lower,
    Upper,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FaceAxis {
    pub unit: Symbol,
    pub side: FaceSide,
}
#[derive(Clone, Debug)]
pub struct Limits {
    pub algebra: AlgebraLimits,
    pub sectors: DecompositionOptions,
    /// Bounds retained source links plus all chart/term/axis power entries.
    pub max_entries: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            algebra: AlgebraLimits::default(),
            sectors: DecompositionOptions {
                max_support_pairs: 100_000,
                max_rays: 4096,
                max_sectors: 1024,
            },
            max_entries: 1_000_000,
        }
    }
}
struct Work<'a> {
    limits: &'a Limits,
    budget: Budget,
    callback: &'a mut dyn FnMut() -> ControlFlow<()>,
}
impl Work<'_> {
    fn check(&mut self) -> Result<()> {
        self.budget.check()?;
        if (self.callback)().is_break() {
            return Err(Error::Cancelled);
        }
        Ok(())
    }
    fn entries(&mut self, n: usize) -> Result<()> {
        self.check()?;
        if n > self.limits.max_entries {
            return Err(Error::ResourceIncomplete(
                "transport retained-entry allowance".into(),
            ));
        }
        Ok(())
    }
}
fn native(e: impl std::fmt::Display) -> Error {
    Error::Native(e.to_string())
}
fn sector_error(e: SectorError) -> Error {
    match e {
        SectorError::Cancelled => Error::Cancelled,
        SectorError::ResourceLimit { .. } => Error::ResourceIncomplete(e.to_string()),
        _ => native(e),
    }
}

/// Explicit source-factor disposition; an inactive declaration is retained
/// without inventing a nonzero geometric factor or dropping its source row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FactorLink {
    Cleared(usize),
    LazyPolynomial,
    Inactive,
}
impl FactorLink {
    pub fn identity(self) -> Option<usize> {
        match self {
            Self::Cleared(i) => Some(i),
            _ => None,
        }
    }
}
/// Original prepared term/factor/coordinate indices. A missing coordinate link
/// denotes exactly a zero original exponent, not an omitted density term.
#[derive(Debug)]
pub struct TermLinks {
    coordinates: Vec<Option<usize>>,
    factors: Vec<FactorLink>,
}
impl TermLinks {
    pub fn coordinates(&self) -> &[Option<usize>] {
        &self.coordinates
    }
    pub fn factors(&self) -> &[FactorLink] {
        &self.factors
    }
}
#[derive(Debug)]
pub struct RootRelation {
    section: Arc<NormalizedSection>,
    equation: Atom,
}
impl RootRelation {
    pub fn section(&self) -> &Arc<NormalizedSection> {
        &self.section
    }
    /// Equation after the admitted coefficient chart, in parent units/root slots.
    pub fn equation(&self) -> &Atom {
        &self.equation
    }
}

/// Full native fan, all source-indexed terms and exact factor identities.
/// Geometry, original selectors, factors and divisor ancestry share one parent.
#[derive(Debug)]
pub struct EndpointTransport {
    parent: Arc<NormalizedPrefix>,
    faces: Vec<FaceAxis>,
    selected: Vec<usize>,
    target_units: Vec<Symbol>,
    source_images: Vec<Atom>,
    roots: Vec<RootRelation>,
    identities: Vec<ClearedFactor>,
    measure: usize,
    links: Vec<TermLinks>,
    supports: Vec<PolynomialSupport>,
    support_indices: Vec<usize>,
    geometry: Decomposition,
    charts: Vec<EndpointChart>,
}
impl EndpointTransport {
    pub fn parent_prefix(&self) -> &Arc<NormalizedPrefix> {
        &self.parent
    }
    pub fn faces(&self) -> &[FaceAxis] {
        &self.faces
    }
    pub fn target_units(&self) -> &[Symbol] {
        &self.target_units
    }
    /// Genuine preparation order, obtained by named join to the CAD axes.
    pub fn source_images(&self) -> &[Atom] {
        &self.source_images
    }
    pub fn root_relations(&self) -> &[RootRelation] {
        &self.roots
    }
    pub fn identities(&self) -> &[ClearedFactor] {
        &self.identities
    }
    pub fn measure_index(&self) -> usize {
        self.measure
    }
    pub fn term_links(&self) -> &[TermLinks] {
        &self.links
    }
    pub fn supports(&self) -> &[PolynomialSupport] {
        &self.supports
    }
    pub fn support_indices(&self) -> &[usize] {
        &self.support_indices
    }
    pub fn geometry(&self) -> &Decomposition {
        &self.geometry
    }
    pub fn charts(&self) -> &[EndpointChart] {
        &self.charts
    }
    /// All native charts and exact term identities are present. Closed faces
    /// and lazy numerator endpoint behavior remain unresolved in every chart.
    pub fn transport_complete(&self) -> bool {
        self.charts.len() == self.geometry.sectors.len()
    }

    pub fn prepare(
        parent: Arc<NormalizedPrefix>,
        faces: &[FaceAxis],
        limits: &Limits,
        mut progress: impl FnMut() -> ControlFlow<()>,
    ) -> Result<Arc<Self>> {
        let mut work = Work {
            limits,
            budget: Budget::new(limits.algebra.clone()),
            callback: &mut progress,
        };
        work.check()?;
        if !parent.complete_open_map() {
            return Err(Error::Invalid(
                "endpoint transport requires the complete admitted prefix".into(),
            ));
        }
        let units = parent.source().unit_coordinates();
        if faces.is_empty()
            || faces.len() > units.len()
            || faces.iter().map(|a| a.unit).collect::<BTreeSet<_>>().len() != faces.len()
        {
            return Err(Error::Invalid(
                "distinct nonempty selected unit-face inventory required".into(),
            ));
        }
        let selected = faces
            .iter()
            .map(|a| {
                units.iter().position(|u| *u == a.unit).ok_or_else(|| {
                    Error::Invalid("selected face is not a parent unit coordinate".into())
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let preparation = parent.source().decomposition().preparation();
        let n = units.len();
        let term_count = preparation.terms().len();
        let source_links = preparation
            .terms()
            .iter()
            .try_fold(0usize, |n, t| {
                n.checked_add(t.factors().len() + t.monomial_powers().len())
            })
            .ok_or_else(|| Error::ResourceIncomplete("source link count overflow".into()))?;
        work.entries(source_links)?;
        let images = parent.images();
        let source_images = preparation
            .coordinates()
            .iter()
            .map(|s| {
                parent
                    .source()
                    .axes()
                    .iter()
                    .position(|a| a.symbol() == *s)
                    .map(|i| images[i].clone())
                    .ok_or_else(|| {
                        Error::Invalid("preparation/CAD named coordinate join differs".into())
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        if preparation.coordinates().len() != n
            || preparation
                .coordinates()
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != n
        {
            return Err(Error::Invalid(
                "complete preparation coordinate inventory differs".into(),
            ));
        }
        let roots = parent
            .steps()
            .into_iter()
            .filter_map(|step| match step {
                PrefixStep::Section { section, .. } => Some(RootRelation {
                    section: section.clone(),
                    equation: pull(
                        section.equation(),
                        &section.chart().parameters,
                        &section.chart().parameter_images,
                    ),
                }),
                _ => None,
            })
            .collect::<Vec<_>>();
        let roles = units
            .iter()
            .copied()
            .chain(roots.iter().map(|r| r.section.root_symbol()))
            .collect::<Vec<_>>();
        let mut ring = Ring::new(roles.clone())?;
        let mut occupied = parent.source().decomposition().occupied_symbols().clone();
        occupied.extend(roles);
        occupied.extend(
            roots
                .iter()
                .flat_map(|r| r.section.chart().parameters.iter().copied()),
        );
        ring.reserve(occupied.iter().copied());
        let target_units = (0usize..)
            .map(|namespace| {
                (0..n)
                    .map(|i| symbolica::symbol!(format!("fastsecdec::endpoint_{namespace}::u{i}")))
                    .collect::<Vec<_>>()
            })
            .find(|symbols| symbols.iter().all(|s| !occupied.contains(s)))
            .expect("finite occupied symbol set");
        let mut identities = Vec::new();
        let mut identity_index = BTreeMap::<Atom, usize>::new();
        let mut insert = |mapped: Atom, work: &mut Work<'_>| -> Result<usize> {
            if let Some(&i) = identity_index.get(&mapped) {
                return Ok(i);
            }
            work.entries(identities.len() + source_links + 1)?;
            let cleared = ring.clear(&mapped, units, work)?;
            let i = identities.len();
            identities.push(cleared);
            identity_index.insert(mapped, i);
            Ok(i)
        };
        let measure = insert(
            parent.diagonal_product() * preparation.measure_jacobian(),
            &mut work,
        )?;
        let mut coordinate_ids = vec![None; n];
        for (i, image) in source_images.iter().enumerate() {
            if preparation
                .terms()
                .iter()
                .any(|t| !t.monomial_powers()[i].is_zero())
            {
                coordinate_ids[i] = Some(insert(image.clone(), &mut work)?);
            }
        }
        let mut links = Vec::with_capacity(term_count);
        // Share identical source polynomials before any coordinate substitution.
        let mut factor_ids = BTreeMap::<Atom, usize>::new();
        for (term_index, term) in preparation.terms().iter().enumerate() {
            work.check()?;
            let coordinates = term
                .monomial_powers()
                .iter()
                .zip(&coordinate_ids)
                .map(|(p, i)| if p.is_zero() { None } else { *i })
                .collect();
            let mut factors = Vec::with_capacity(term.factors().len());
            for (factor_index, factor) in term.factors().iter().enumerate() {
                if factor.exponent().is_zero() {
                    if !matches!(
                        parent.source().decomposition().factor_authorities()[term_index]
                            [factor_index],
                        crate::threshold::gcad::ProjectiveFactorAuthority::Inactive
                    ) {
                        return Err(Error::Invalid(
                            "inactive source factor authority differs".into(),
                        ));
                    }
                    factors.push(FactorLink::Inactive);
                    continue;
                }
                if factor.role() == FactorRole::Polynomial {
                    factors.push(FactorLink::LazyPolynomial);
                    continue;
                }
                let index = if let Some(&i) = factor_ids.get(factor.polynomial()) {
                    i
                } else {
                    let i = insert(
                        pull(
                            factor.polynomial(),
                            preparation.coordinates(),
                            &source_images,
                        ),
                        &mut work,
                    )?;
                    factor_ids.insert(factor.polynomial().clone(), i);
                    i
                };
                factors.push(FactorLink::Cleared(index));
            }
            links.push(TermLinks {
                coordinates,
                factors,
            });
        }
        let reflection = units
            .iter()
            .enumerate()
            .map(|(i, u)| {
                if faces
                    .iter()
                    .zip(&selected)
                    .any(|(f, &j)| i == j && f.side == FaceSide::Upper)
                {
                    Atom::one() - Atom::var(*u)
                } else {
                    Atom::var(*u)
                }
            })
            .collect::<Vec<_>>();
        let selected_symbols = faces.iter().map(|a| a.unit).collect::<Vec<_>>();
        let mut supports = Vec::new();
        let mut support_indices = Vec::with_capacity(identities.len());
        for identity in &identities {
            let distance_n = pull(identity.numerator(), units, &reflection);
            // Native QQ admission bounds expansion before AtomField support collection.
            ring.polynomial(&distance_n, &mut work)?;
            let support = crate::parametric::polynomial_support(&distance_n, &selected_symbols)
                .map_err(native)?;
            let index = if let Some(i) = supports.iter().position(|s| s == &support) {
                i
            } else {
                supports.push(support);
                supports.len() - 1
            };
            support_indices.push(index);
        }
        let mut failure = None;
        let geometry = fastsecdec_sectors::decompose(
            ParametricDomain::UnitCube,
            &supports,
            &limits.sectors,
            |_| match work.check() {
                Ok(()) => ControlFlow::Continue(()),
                Err(e) => {
                    failure = Some(e);
                    ControlFlow::Break(())
                }
            },
        );
        if let Some(e) = failure {
            return Err(e);
        }
        let geometry = geometry.map_err(sector_error)?;
        let count = geometry
            .sectors
            .len()
            .checked_mul(term_count)
            .and_then(|v| v.checked_mul(n))
            .and_then(|v| v.checked_add(source_links))
            .and_then(|v| {
                v.checked_add(
                    geometry
                        .sectors
                        .len()
                        .checked_mul(identities.len().checked_mul(n + 2)?)?,
                )
            })
            .ok_or_else(|| {
                Error::ResourceIncomplete("transport inventory count overflow".into())
            })?;
        work.entries(count)?;
        let mut owner = Self {
            parent,
            faces: faces.to_vec(),
            selected,
            target_units,
            source_images,
            roots,
            identities,
            measure,
            links,
            supports,
            support_indices,
            geometry,
            charts: Vec::new(),
        };
        for ordinal in 0..owner.geometry.sectors.len() {
            let chart = chart::prepare(&owner, ordinal, &mut work)?;
            owner.charts.push(chart);
        }
        work.check()?;
        Ok(Arc::new(owner))
    }
}
