//! Positive-coordinate anchor gauge for an actual homogeneous projective density.
use super::{FactorOrigin, ProjectiveError};
use crate::parametric::{ParametricDomain, ParametricIntegrand, ParametricTerm, PolynomialFactor};
use std::sync::Arc;
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    id::{Pattern, Replacement},
};

/// The full positive orthant of ratios, not a bounded source-sector chart.
///
/// If S=1+sum(y), normalized simplex images are (1,y)/S. Their delta-gauge
/// Jacobian is S^-n. Every admitted source term has degree -n, so the complete
/// density is exactly its anchor=1 specialization with ratio measure dy.
/// Positive S preserves all original branches. This gives no endpoint or
/// infinity regularity, and does not license an arbitrary native CAD result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PositiveProjectivePreparation {
    input: Arc<ParametricIntegrand>,
    anchor_index: usize,
    coordinates: Vec<Symbol>,
    images: Vec<Atom>,
    normalized_images: Vec<Atom>,
    normalization: Atom,
    terms: Vec<ParametricTerm>,
    factor_origins: Vec<Vec<FactorOrigin>>,
    factor_degrees: Vec<Vec<Option<u32>>>,
    strict_positive: Vec<Atom>,
    simplex_jacobian: Atom,
    measure_jacobian: Atom,
}

impl PositiveProjectivePreparation {
    /// Consume the genuine complete density. Coordinate and factor orders are
    /// retained; zero-exponent factors remain declared but do not supply degree.
    pub fn anchor(
        input: Arc<ParametricIntegrand>,
        anchor_index: usize,
    ) -> Result<Self, ProjectiveError> {
        let invalid = |s: &str| ProjectiveError::InvalidAnchor(s.into());
        if input.domain() != ParametricDomain::ProjectiveSimplex {
            return Err(invalid(
                "positive anchor requires the native projective delta measure",
            ));
        }
        let parameters = input.parameters();
        let Some(&anchor) = parameters.get(anchor_index) else {
            return Err(invalid(
                "anchor is outside the original coordinate inventory",
            ));
        };
        let coordinates = parameters
            .iter()
            .enumerate()
            .filter_map(|(i, s)| (i != anchor_index).then_some(*s))
            .collect::<Vec<_>>();
        let normalization = Atom::one() + coordinates.iter().map(|s| Atom::var(*s)).sum::<Atom>();
        let images = parameters
            .iter()
            .enumerate()
            .map(|(i, s)| {
                if i == anchor_index {
                    Atom::one()
                } else {
                    Atom::var(*s)
                }
            })
            .collect::<Vec<_>>();
        let normalized_images = images.iter().map(|image| image / &normalization).collect();
        let replacement = [Replacement::new(
            Pattern::Literal(Atom::var(anchor)),
            Pattern::Literal(Atom::one()),
        )];
        let mut terms = Vec::with_capacity(input.terms().len());
        let mut factor_origins = Vec::with_capacity(input.terms().len());
        let mut factor_degrees = Vec::with_capacity(input.terms().len());
        for term in input.terms() {
            let mut degree = term.monomial_powers().iter().sum::<Atom>();
            let mut degrees = Vec::with_capacity(term.factors().len());
            let mut origins = Vec::with_capacity(term.factors().len());
            let mut factors = Vec::with_capacity(term.factors().len());
            for (factor_index, factor) in term.factors().iter().enumerate() {
                let factor_degree = if factor.exponent().is_zero() {
                    None
                } else {
                    let degree =
                        crate::parametric::homogeneity::degree(factor.polynomial(), parameters)
                            .or_else(|| {
                                let support = factor.support(parameters).ok()?;
                                let mut degree = None;
                                for row in support.exponents() {
                                    let current = row.iter().try_fold(0u32, |sum, power| {
                                        let power = u32::try_from(power.to_i64()?).ok()?;
                                        sum.checked_add(power)
                                    })?;
                                    if degree.is_some_and(|old| old != current) {
                                        return None;
                                    }
                                    degree = Some(current);
                                }
                                degree
                            })
                            .ok_or_else(|| {
                                invalid("active source factor lacks exact homogeneous degree")
                            })?;
                    Some(degree)
                };
                if let Some(d) = factor_degree {
                    degree += Atom::num(d) * factor.exponent();
                }
                degrees.push(factor_degree);
                origins.push(FactorOrigin::OriginalFactor { factor_index });
                factors.push(
                    PolynomialFactor::new(
                        factor.polynomial().replace_multiple(&replacement),
                        factor.exponent().clone(),
                        factor.role(),
                    )
                    .with_semantics(factor.semantics()),
                );
            }
            if !(degree + Atom::num(parameters.len())).expand().is_zero() {
                return Err(invalid(
                    "complete source term is not of degree minus its dimension",
                ));
            }
            terms.push(ParametricTerm::new(
                term.prefactor().clone(),
                term.monomial_powers()
                    .iter()
                    .enumerate()
                    .filter_map(|(i, p)| (i != anchor_index).then_some(p.clone()))
                    .collect(),
                factors,
            ));
            factor_origins.push(origins);
            factor_degrees.push(degrees);
        }
        let strict_positive = coordinates.iter().map(|s| Atom::var(*s)).collect();
        let simplex_jacobian = normalization.pow(-Atom::num(parameters.len()));
        Ok(Self {
            input,
            anchor_index,
            coordinates,
            images,
            normalized_images,
            normalization,
            terms,
            factor_origins,
            factor_degrees,
            strict_positive,
            simplex_jacobian,
            measure_jacobian: Atom::one(),
        })
    }
    pub fn input(&self) -> &Arc<ParametricIntegrand> {
        &self.input
    }
    pub fn anchor_index(&self) -> usize {
        self.anchor_index
    }
    pub fn coordinates(&self) -> &[Symbol] {
        &self.coordinates
    }
    /// Original-coordinate images in the anchor gauge; the anchor image is1.
    pub fn images(&self) -> &[Atom] {
        &self.images
    }
    /// Distinct map into the original normalized simplex, including all n coordinates.
    pub fn normalized_images(&self) -> &[Atom] {
        &self.normalized_images
    }
    pub fn normalization(&self) -> &Atom {
        &self.normalization
    }
    pub fn terms(&self) -> &[ParametricTerm] {
        &self.terms
    }
    pub fn factor_origins(&self) -> &[Vec<FactorOrigin>] {
        &self.factor_origins
    }
    pub fn factor_degrees(&self) -> &[Vec<Option<u32>>] {
        &self.factor_degrees
    }
    pub fn strict_positive(&self) -> &[Atom] {
        &self.strict_positive
    }
    /// Jacobian of the normalized map in the existing affine delta gauge.
    pub fn simplex_jacobian(&self) -> &Atom {
        &self.simplex_jacobian
    }
    /// Ratio measure after exact complete-density homogeneity cancellation.
    pub fn measure_jacobian(&self) -> &Atom {
        &self.measure_jacobian
    }
}
