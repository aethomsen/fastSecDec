//! Causal phases of verified open cells, before endpoint continuation.
//!
//! This view neither resolves a closed face nor proves that separate cell
//! integrals possess a common analytic continuation.

use symbolica::atom::{Atom, AtomCore, Symbol};

use super::gcad::{GcadError, Result, SignedFactor, VerifiedCell};
use crate::{generation::endpoint_power_with_regulators, parametric::FactorSemantics};

/// Branch data borrowed from a verified cell. Phases belong to individual
/// density terms; sharing a geometric polynomial never combines their powers.
#[derive(Debug)]
pub struct CausalCell<'a> {
    cell: VerifiedCell<'a>,
    term_phases: Vec<Atom>,
}

impl<'a> CausalCell<'a> {
    /// Retain all physical and auxiliary regulator dependence in each phase.
    /// The physical regulator is first; subsequent regulators are explicit.
    pub fn new(cell: VerifiedCell<'a>, regulators: &[Symbol]) -> Result<Self> {
        let request = cell.decomposition().request();
        if regulators.first() != Some(&request.input().regulator()) {
            return Err(GcadError::Invalid(
                "cell phases require the original physical regulator first".into(),
            ));
        }
        if regulators.iter().any(|r| {
            request.input().parameters().contains(r)
                || request.domain().coordinates().contains(r)
                || request.kinematics().runtime_parameters.contains(r)
                || request.kinematics().exact_values.contains_key(r)
        }) {
            return Err(GcadError::Invalid(
                "regulators cannot also be integration coordinates or kinematic parameters".into(),
            ));
        }
        // Validate the inventory even when the cell has no signed factors.
        endpoint_power_with_regulators(&Atom::Zero, regulators)
            .map_err(|e| GcadError::Invalid(e.to_string()))?;
        let mut term_phases = vec![Atom::num(1); request.prepared_terms().len()];
        for (factor, sign) in cell.signed_factors() {
            if !matches!(sign, -1 | 1) {
                return Err(GcadError::Unsupported(format!(
                    "cell {} factor {}:{} lacks a nonzero open-cell sign",
                    cell.index(),
                    factor.term_index,
                    factor.factor_index
                )));
            }
            let exponent = request.kinematics().specialize_exact(&factor.exponent);
            if sign < 0 && !exponent.is_zero() {
                if factor.semantics != FactorSemantics::Causal {
                    return Err(GcadError::Unsupported(format!(
                        "negative cell factor {}:{} needs explicit causal branch semantics",
                        factor.term_index, factor.factor_index
                    )));
                }
                term_phases[factor.term_index] *= causal_negative_phase(&exponent, regulators)?;
            }
        }
        Ok(Self { cell, term_phases })
    }

    pub fn cell(&self) -> VerifiedCell<'a> {
        self.cell
    }

    /// Multipliers to apply once to the corresponding prepared term prefactor.
    /// They contain the complete regulator-dependent causal phase only.
    pub fn term_phases(&self) -> &[Atom] {
        &self.term_phases
    }

    /// Signed factors retain their association with the full original density.
    /// Polynomial numerator factors are untouched by branch extraction.
    pub fn magnitudes(&self) -> impl ExactSizeIterator<Item = CellMagnitude<'a>> {
        self.cell
            .signed_factors()
            .map(|(factor, sign)| CellMagnitude { factor, sign })
    }
}

/// Exact lower-lip phase arithmetic only. The caller must supply the actual
/// negative causal-factor authority and validated source regulator roles.
pub(crate) fn causal_negative_phase(exponent: &Atom, regulators: &[Symbol]) -> Result<Atom> {
    let (constant, _) = endpoint_power_with_regulators(exponent, regulators)
        .map_err(|e| GcadError::Unsupported(e.to_string()))?;
    let constant = Atom::num(constant);
    // Principal (-1)^(-q0) gives exp(-i*pi*q0), including the native exact
    // integer/half-integer simplifications. Keep all regulator dependence.
    Ok(Atom::num(-1).pow(-&constant)
        * (-Atom::i() * Atom::var(Symbol::PI) * (exponent - constant)).exp())
}

/// A positive interior magnitude, subject to the owning cell's kinematic
/// bindings and chamber. No positivity claim is made on a closed face.
#[derive(Clone, Copy, Debug)]
pub struct CellMagnitude<'a> {
    factor: &'a SignedFactor,
    sign: i8,
}

impl<'a> CellMagnitude<'a> {
    pub fn factor(&self) -> &'a SignedFactor {
        self.factor
    }
    pub fn sign(&self) -> i8 {
        self.sign
    }
    /// A native symbolic expression in the prepared coordinates and original
    /// parameter symbols. Runtime/exact bindings remain the owner's settings.
    pub fn polynomial(&self) -> Atom {
        Atom::num(self.sign as i64) * &self.factor.prepared_polynomial
    }
}

#[cfg(test)]
mod tests;
