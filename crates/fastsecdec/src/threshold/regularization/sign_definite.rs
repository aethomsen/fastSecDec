//! Whole-source causal phase extraction for coefficient-sign densities.
//!
//! Native domain certificates prove signs on the original positive orthant.
//! Endpoint subtraction is still required; this owner is not a generated
//! threshold artifact or a replacement for closed-face resolution.
use super::{Result, invalid, unsupported};
use crate::parametric::{
    FactorRole, FactorSemantics, ParametricIntegrand, ParametricTerm, PolynomialFactor,
};
use crate::threshold::{
    gcad::VerifiedDecomposition, phase::causal_negative_phase, projective::FactorOrigin,
};
use std::sync::Arc;
use symbolica::{
    atom::{Atom, AtomCore},
    prelude::{PolyVariable, Q},
};
use symgcad::{
    algebra::Algebra,
    domain::{
        CertificateMethod, CertificateOptions, DomainCertificate, certify_nonzero, derive_domain,
        verify_certificate,
    },
};

/// Exact association of one original factor with its orthant sign proof.
#[derive(Debug)]
pub struct SignedSourceFactor {
    pub term: usize,
    pub factor: usize,
    pub certificate: DomainCertificate,
}

/// A source-bound identity retaining all epsilon-dependent lower-lip phases.
/// Only exact, fixed kinematics and same-sign rational coefficients are admitted.
pub struct SignDefiniteSource {
    decomposition: Arc<VerifiedDecomposition>,
    positive: ParametricIntegrand,
    factors: Vec<SignedSourceFactor>,
}

impl SignDefiniteSource {
    pub fn admit(
        decomposition: Arc<VerifiedDecomposition>,
        limits: &CertificateOptions,
    ) -> Result<Self> {
        let request = decomposition.request();
        if !request.kinematics().runtime_parameters.is_empty() || decomposition.cells().is_empty() {
            return Err(unsupported(
                "sign-definite source requires fixed kinematics and nonempty verified geometry",
            ));
        }
        let input = request.exact_input();
        let names = (0..input.parameters().len())
            .map(|i| format!("signed_source_axis_{i}"))
            .collect::<Vec<_>>();
        let algebra = Algebra::new(&names).map_err(|e| invalid(&e.to_string()))?;
        let variables = Arc::new(
            input
                .parameters()
                .iter()
                .copied()
                .map(PolyVariable::from)
                .collect::<Vec<_>>(),
        );
        let constraints = names
            .iter()
            .map(|n| algebra.parse(n).map_err(|e| invalid(&e.to_string())))
            .collect::<Result<Vec<_>>>()?;
        let domain =
            derive_domain(&algebra, &constraints, &[]).map_err(|e| invalid(&e.to_string()))?;
        let specialize = |a: &Atom| request.kinematics().specialize_exact(a);
        let mut factors = Vec::new();
        let mut terms = Vec::new();
        for (term_index, term) in input.terms().iter().enumerate() {
            let mut prefactor = specialize(term.prefactor());
            let mut normalized = Vec::new();
            for (factor_index, factor) in term.factors().iter().enumerate() {
                let polynomial = specialize(factor.polynomial());
                let exponent = specialize(factor.exponent());
                if factor.role() == FactorRole::Polynomial {
                    normalized.push(
                        PolynomialFactor::new(polynomial, exponent, factor.role())
                            .with_semantics(factor.semantics()),
                    );
                    continue;
                }
                // Check closure in the actual source roles before assigning
                // native GCAD aliases; an external coefficient must not become
                // an integration variable through a name collision.
                let source: symgcad::algebra::Poly = polynomial
                    .try_to_polynomial(&Q, variables.clone())
                    .map_err(|e| {
                        unsupported(&format!("exact rational source factor required: {e}"))
                    })?;
                if source.variables() != &variables {
                    return Err(unsupported("source factor has undeclared symbols"));
                }
                let p = algebra
                    .parse(&algebra.format(&source))
                    .map_err(|e| invalid(&e.to_string()))?;
                let certificate = certify_nonzero(&algebra, &p, &domain, limits)
                    .map_err(|e| invalid(&e.to_string()))?
                    .ok_or_else(|| unsupported("source factor lacks a uniform orthant sign"))?;
                verify_certificate(&algebra, &p, &constraints, &[], &certificate)
                    .map_err(|e| invalid(&e.to_string()))?;
                if certificate.method != CertificateMethod::ShiftedCoefficients {
                    return Err(unsupported(
                        "this source route requires coefficient-sign endpoint geometry",
                    ));
                }
                if certificate.sign < 0 {
                    if factor.semantics() != FactorSemantics::Causal {
                        return Err(unsupported(
                            "negative source factor lacks causal branch semantics",
                        ));
                    }
                    prefactor *= causal_negative_phase(&exponent, &[input.regulator()])?;
                }
                // Cross-check every GCAD cell against the independently proved
                // original-source sign. Factor origins survive gauge fixing.
                for cell in decomposition.cells() {
                    for (signed, sign) in cell.signed_factors() {
                        if signed.term_index == term_index
                            && signed.origin == (FactorOrigin::OriginalFactor { factor_index })
                            && sign != certificate.sign
                        {
                            return Err(invalid("source sign disagrees with verified cell"));
                        }
                    }
                }
                normalized.push(
                    PolynomialFactor::new(
                        Atom::num(certificate.sign as i64) * polynomial,
                        exponent,
                        factor.role(),
                    )
                    .with_semantics(FactorSemantics::Positive),
                );
                factors.push(SignedSourceFactor {
                    term: term_index,
                    factor: factor_index,
                    certificate,
                });
            }
            terms.push(ParametricTerm::new(
                prefactor,
                term.monomial_powers().iter().map(specialize).collect(),
                normalized,
            ));
        }
        let positive = ParametricIntegrand::new(
            input.parameters().to_vec(),
            input.regulator(),
            input.domain(),
            terms,
        )
        .map_err(|e| invalid(&e.to_string()))?;
        Ok(Self {
            decomposition,
            positive,
            factors,
        })
    }
    pub fn decomposition(&self) -> &Arc<VerifiedDecomposition> {
        &self.decomposition
    }
    /// Exact magnitude density, including its complete causal phase. Callers
    /// still use the native sector and symbolic subtraction stages.
    pub fn positive_input(&self) -> &ParametricIntegrand {
        &self.positive
    }
    pub fn factors(&self) -> &[SignedSourceFactor] {
        &self.factors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        generation::{self, GenerationOptions},
        parametric::ParametricDomain,
        threshold::gcad::{GcadKinematics, GcadRequest, SolverOptions},
    };
    use symbolica::{atom::Symbol, symbol};
    fn source(factor: Atom, semantics: FactorSemantics) -> Arc<VerifiedDecomposition> {
        let (x, eps) = symbol!("sign_source_test::x", "sign_source_test::eps");
        let input = ParametricIntegrand::new(
            vec![x],
            eps,
            ParametricDomain::UnitCube,
            vec![ParametricTerm::new(
                Atom::one() + Atom::num(2) * Atom::i(),
                vec![Atom::Zero],
                vec![
                    PolynomialFactor::new(
                        factor,
                        -Atom::one() + Atom::var(eps),
                        FactorRole::Singularity,
                    )
                    .with_semantics(semantics),
                ],
            )],
        )
        .unwrap();
        Arc::new(
            GcadRequest::unit_cube(
                &input,
                GcadKinematics::default(),
                SolverOptions::default(),
                symgcad::Limits {
                    workers: 1,
                    wall_time_secs: 10.,
                    memory_mib: 1024,
                    ..Default::default()
                },
            )
            .unwrap()
            .solve_verified()
            .unwrap(),
        )
    }
    #[test]
    fn source_sign_phase_keeps_finite_imaginary_coefficient_through_subtraction() {
        let (x, _) = symbol!("sign_source_test::x", "sign_source_test::eps");
        let original = source(-Atom::var(x), FactorSemantics::Causal);
        let separated = SignDefiniteSource::admit(original.clone(), &Default::default()).unwrap();
        assert!(Arc::ptr_eq(separated.decomposition(), &original));
        assert_eq!(separated.factors().len(), 1);
        assert_eq!(separated.factors()[0].certificate.sign, -1);
        let generated = generation::generate(
            separated.positive_input(),
            &GenerationOptions {
                max_order: 0,
                ..Default::default()
            },
            |_| std::ops::ControlFlow::Continue(()),
        )
        .unwrap();
        let expected = [
            -Atom::one() - Atom::num(2) * Atom::i(),
            (-Atom::num(2) + Atom::i()) * Atom::var(Symbol::PI),
        ];
        for (order, value) in [(-1, &expected[0]), (0, &expected[1])] {
            let index = generated.orders().iter().position(|o| *o == order).unwrap();
            assert!(
                (generated.exact_coefficients()[index].clone() - value)
                    .expand()
                    .is_zero()
            );
        }
    }
    #[test]
    fn changing_sign_source_cannot_enter_whole_source_route() {
        let x = symbol!("sign_source_test::x");
        let original = source(Atom::var(x) - Atom::num((1, 2)), FactorSemantics::Causal);
        assert!(SignDefiniteSource::admit(original, &Default::default()).is_err());
    }
    #[test]
    fn negative_generic_branch_is_not_silently_made_causal() {
        let x = symbol!("sign_source_test::x");
        let original = source(-Atom::var(x), FactorSemantics::Generic);
        assert!(SignDefiniteSource::admit(original, &Default::default()).is_err());
    }
}
