//! Compact affine elimination of the native projective delta constraint.
//!
//! This preparation has a simplex domain, not a unit cube or positive orthant.
//! It preserves the original integral and does not certify endpoint regularity.

use std::sync::Arc;

use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    id::{Pattern, Replacement},
};

use crate::parametric::{
    FactorRole, FactorSemantics, ParametricDomain, ParametricIntegrand, ParametricTerm,
    PolynomialFactor,
};

/// Association with the original term, before its projective delta constraint
/// is eliminated. The term index is retained separately by the prepared owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FactorOrigin {
    OriginalFactor { factor_index: usize },
    OriginalMonomial { parameter_index: usize },
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectiveError {
    #[error("invalid positive projective preparation: {0}")]
    InvalidAnchor(String),
    #[error("invalid affine projective preparation: {0}")]
    Invalid(String),
}

/// A proved affine delta-gauge preparation of one complete native density.
///
/// The remaining original coordinates retain their order. Their positive open
/// simplex has `x_d = 1 - sum(x_i, i != d) > 0`. The measure multiplier is one:
/// the square change of variables from `(remaining x, c=1-sum(x))` to all
/// original coordinates has determinant of absolute value one, and integration
/// against `delta(c)` removes `c`. This is not Euclidean surface area measure.
///
/// Construction preserves every original term, factor role and branch, without
/// expanding the full density. It adds no regulator: an auxiliary family must
/// either already satisfy the original projective homogeneity or be introduced
/// after this gauge with its own continuation proof.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AffineProjectivePreparation {
    input: Arc<ParametricIntegrand>,
    eliminated_index: usize,
    coordinates: Vec<Symbol>,
    images: Vec<Atom>,
    terms: Vec<ParametricTerm>,
    factor_origins: Vec<Vec<FactorOrigin>>,
    strict_positive: Vec<Atom>,
    measure_jacobian: Atom,
}

impl AffineProjectivePreparation {
    /// Eliminate the last coordinate in the native input's declared order.
    pub fn last_coordinate(input: &ParametricIntegrand) -> Result<Self, ProjectiveError> {
        let index = input.parameters().len().checked_sub(1).ok_or_else(|| {
            ProjectiveError::Invalid("a projective simplex needs a coordinate".into())
        })?;
        Self::eliminate(input, index)
    }

    pub fn eliminate(
        input: &ParametricIntegrand,
        eliminated_index: usize,
    ) -> Result<Self, ProjectiveError> {
        if input.domain() != ParametricDomain::ProjectiveSimplex {
            return Err(ProjectiveError::Invalid(
                "the original density must declare the native projective delta measure".into(),
            ));
        }
        let Some(&eliminated) = input.parameters().get(eliminated_index) else {
            return Err(ProjectiveError::Invalid(
                "eliminated coordinate is outside the original parameter order".into(),
            ));
        };
        let coordinates = input
            .parameters()
            .iter()
            .copied()
            .filter(|s| *s != eliminated)
            .collect::<Vec<_>>();
        let complement = Atom::num(1) - coordinates.iter().map(|s| Atom::var(*s)).sum::<Atom>();
        let images = input
            .parameters()
            .iter()
            .map(|s| {
                if *s == eliminated {
                    complement.clone()
                } else {
                    Atom::var(*s)
                }
            })
            .collect();
        let replacements = [Replacement::new(
            Pattern::Literal(Atom::var(eliminated)),
            Pattern::Literal(complement.clone()),
        )];
        let mut terms = Vec::with_capacity(input.terms().len());
        let mut factor_origins = Vec::with_capacity(input.terms().len());
        for term in input.terms() {
            let mut origins = Vec::with_capacity(term.factors().len() + 1);
            let mut factors = term
                .factors()
                .iter()
                .enumerate()
                .map(|(factor_index, factor)| {
                    origins.push(FactorOrigin::OriginalFactor { factor_index });
                    PolynomialFactor::new(
                        factor.polynomial().replace_multiple(&replacements),
                        factor.exponent().clone(),
                        factor.role(),
                    )
                    .with_semantics(factor.semantics())
                })
                .collect::<Vec<_>>();
            let eliminated_power = &term.monomial_powers()[eliminated_index];
            if !eliminated_power.is_zero() {
                factors.push(
                    PolynomialFactor::new(
                        complement.clone(),
                        eliminated_power.clone(),
                        FactorRole::Singularity,
                    )
                    .with_semantics(FactorSemantics::Positive),
                );
                origins.push(FactorOrigin::OriginalMonomial {
                    parameter_index: eliminated_index,
                });
            }
            terms.push(ParametricTerm::new(
                term.prefactor().clone(),
                term.monomial_powers()
                    .iter()
                    .enumerate()
                    .filter_map(|(i, power)| (i != eliminated_index).then_some(power.clone()))
                    .collect(),
                factors,
            ));
            factor_origins.push(origins);
        }
        let mut strict_positive = coordinates
            .iter()
            .map(|s| Atom::var(*s))
            .collect::<Vec<_>>();
        strict_positive.push(complement);
        Ok(Self {
            input: Arc::new(input.clone()),
            eliminated_index,
            coordinates,
            images,
            terms,
            factor_origins,
            strict_positive,
            measure_jacobian: Atom::num(1),
        })
    }

    pub fn input(&self) -> &ParametricIntegrand {
        &self.input
    }
    pub(crate) fn input_owner(&self) -> &Arc<ParametricIntegrand> {
        &self.input
    }
    pub fn eliminated_index(&self) -> usize {
        self.eliminated_index
    }
    pub fn coordinates(&self) -> &[Symbol] {
        &self.coordinates
    }
    /// Images in the original input's coordinate order.
    pub fn images(&self) -> &[Atom] {
        &self.images
    }
    pub fn terms(&self) -> &[ParametricTerm] {
        &self.terms
    }
    /// Origin for every prepared factor, in term and factor order.
    pub fn factor_origins(&self) -> &[Vec<FactorOrigin>] {
        &self.factor_origins
    }
    pub fn strict_positive(&self) -> &[Atom] {
        &self.strict_positive
    }
    /// Zero sets of these same polynomials are the faces of the closed simplex.
    /// This retains the boundary geometry; it does not certify density limits.
    pub fn boundary_polynomials(&self) -> &[Atom] {
        &self.strict_positive
    }
    pub fn measure_jacobian(&self) -> &Atom {
        &self.measure_jacobian
    }
}

#[cfg(test)]
mod tests;

mod anchor;
pub use anchor::PositiveProjectivePreparation;
