//! Admission of unchanged native margin certificates in their original frame.
use super::{Budget, Error, MarginEvidence, MarginProof, ProofBox, Rational, Result, Ring};
use std::{collections::BTreeSet, sync::Arc};
use symbolica::atom::{Atom, AtomCore, Symbol};
use symgcad::{
    algebra::{Poly, rational},
    domain::{CertificateOptions, DomainCertificate, derive_domain, verify_certificate},
};

/// A caller-owned proposal, not an issued proof. The original JSON and ordered
/// native constraints remain intact. `roles[i]` binds the i-th original native
/// variable to an actual chart unit or selected-root slot; display names alone
/// never establish that association.
#[derive(Debug)]
pub struct RetainedMargin {
    roles: Vec<Symbol>,
    constraints: Vec<String>,
    margin: Rational,
    scale: Rational,
    bytes: Arc<[u8]>,
    certificate: DomainCertificate,
}
impl RetainedMargin {
    /// Decode within an explicit byte allowance. All mathematical checks, native
    /// replay and operation-resource admission occur when a chart consumes it.
    pub fn from_json(
        roles: Vec<Symbol>,
        constraints: Vec<String>,
        margin: Rational,
        bytes: Arc<[u8]>,
        max_bytes: usize,
    ) -> Result<Self> {
        let constraint_bytes = constraints
            .iter()
            .try_fold(0usize, |n, c| n.checked_add(c.len()));
        if constraint_bytes
            .and_then(|n| n.checked_add(bytes.len()))
            .is_none_or(|n| n > max_bytes)
        {
            return Err(Error::ResourceIncomplete(
                "retained certificate byte allowance".into(),
            ));
        }
        let certificate: DomainCertificate = serde_json::from_slice(&bytes)
            .map_err(|e| Error::Invalid(format!("retained certificate JSON: {e}")))?;
        if margin <= 0
            || roles.is_empty()
            || roles.len() != certificate.domain.variables.len()
            || roles.iter().collect::<BTreeSet<_>>().len() != roles.len()
        {
            return Err(Error::Invalid(
                "positive retained margin and complete distinct native roles required".into(),
            ));
        }
        Ok(Self {
            roles,
            constraints,
            margin,
            scale: Rational::one(),
            bytes,
            certificate,
        })
    }
    /// Propose an exact positive QQ normalization, without changing the original
    /// certificate, domain, margin or bytes. Admission checks Q = scale * P in
    /// the genuine chart frame and replays P - original_margin unchanged.
    /// Rational resource limits are enforced when the proposal is consumed.
    pub fn with_positive_scale(mut self, scale: Rational) -> Result<Self> {
        if scale <= 0 {
            return Err(Error::Invalid(
                "retained normalization scale must be positive".into(),
            ));
        }
        self.scale = scale;
        Ok(self)
    }
    /// Exact multiplier between the original positive polynomial and the actual
    /// chart obligation. The issued MarginProof retains the actual polynomial
    /// and margin; `margin()` below retains the original certificate margin.
    pub fn scale(&self) -> &Rational {
        &self.scale
    }
    pub fn original_bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn certificate(&self) -> &DomainCertificate {
        &self.certificate
    }
    pub fn roles(&self) -> &[Symbol] {
        &self.roles
    }
    pub fn constraints(&self) -> &[String] {
        &self.constraints
    }
    pub fn margin(&self) -> &Rational {
        &self.margin
    }

    pub(super) fn replay(&self, target: &Atom, actual: &ProofBox, b: &Budget) -> Result<()> {
        b.check()?;
        b.rational(&self.margin)?;
        b.rational(&self.scale)?;
        if self.scale <= 0 {
            return Err(Error::Invalid(
                "retained normalization scale must be positive".into(),
            ));
        }
        if self.certificate.sign != 1
            || self.certificate.domain.empty
            || !self.certificate.domain.excluded_sign_constraints.is_empty()
            || self.roles.iter().collect::<BTreeSet<_>>()
                != actual.roles.iter().collect::<BTreeSet<_>>()
        {
            return Err(Error::Invalid("retained proof requires the complete actual chart roles and nonempty positive domain".into()));
        }
        allowance(&self.certificate.limits, &b.limits.certificates)?;
        if self.constraints.len() > b.limits.max_terms {
            return Err(Error::ResourceIncomplete(
                "retained constraint inventory allowance".into(),
            ));
        }
        let ring = Ring::with_names(self.roles.clone(), &self.certificate.domain.variables)?;
        let polynomial = ring.polynomial(target, b)?;
        // Bound conversion before native replay reparses the saved target. This
        // equality is exact QQ equality, not a digest or string comparison.
        let original = self.original_polynomial(&ring, &self.certificate.polynomial, b)?;
        let scaled = original.clone().mul_coeff(self.scale.clone());
        for coefficient in &scaled.coefficients {
            b.check()?;
            b.rational(coefficient)?;
        }
        // target is Q - scale*m; comparing it to scale*(P-m) establishes
        // Q=scale*P exactly, with the original native proof still proving P-m.
        if polynomial != scaled {
            return Err(Error::Invalid(
                "retained native target differs from chart obligation".into(),
            ));
        }
        let constraints = self
            .constraints
            .iter()
            .map(|s| self.original_polynomial(&ring, s, b))
            .collect::<Result<Vec<_>>>()?;
        for p in &constraints {
            if p.is_constant() {
                if p.get_constant() <= 0 {
                    return Err(Error::Unsupported(
                        "nonpositive constant retained domain constraint".into(),
                    ));
                }
            } else {
                let support = (0..self.roles.len())
                    .filter(|&i| p.degree(i) > 0)
                    .collect::<Vec<_>>();
                if support.len() != 1 || p.degree(support[0]) != 1 {
                    return Err(Error::Unsupported(
                        "retained domain requires original coordinate-affine constraints".into(),
                    ));
                }
            }
        }
        let domain = derive_domain(&ring.native, &constraints, &[]).map_err(native)?;
        if domain != self.certificate.domain || domain.empty {
            return Err(Error::Invalid(
                "retained original domain/constraint order differs".into(),
            ));
        }
        for (role, bound) in self.roles.iter().zip(&domain.bounds) {
            let index = actual
                .roles
                .iter()
                .position(|s| s == role)
                .expect("checked complete roles");
            let [lo, hi] = &actual.bounds[index];
            if let Some(lower) = &bound.lower {
                let lower = rational(lower).map_err(native)?;
                b.rational(&lower)?;
                if lo < &lower {
                    return Err(Error::Invalid(
                        "actual closed box exceeds retained lower bound".into(),
                    ));
                }
            }
            if let Some(upper) = &bound.upper {
                let upper = rational(upper).map_err(native)?;
                b.rational(&upper)?;
                if hi > &upper {
                    return Err(Error::Invalid(
                        "actual closed box exceeds retained upper bound".into(),
                    ));
                }
            }
        }
        b.check()?;
        verify_certificate(
            &ring.native,
            &original,
            &constraints,
            &[],
            &self.certificate,
        )
        .map_err(native)?;
        b.check()
    }
    fn original_polynomial(&self, ring: &Ring, text: &str, b: &Budget) -> Result<Poly> {
        // Native Atom parsing includes normalization; caller byte limits and the
        // outer resource envelope apply there. Ring then performs its existing
        // bounded preflight before native QQ polynomial conversion.
        let atom =
            Atom::parse(text.replace("**", "^"), "symgcad", Default::default()).map_err(native)?;
        let aliases = ring
            .native
            .variables
            .iter()
            .map(|v| {
                v.to_atom()
                    .as_view()
                    .as_var_view()
                    .expect("native variable")
                    .get_symbol()
            })
            .collect::<Vec<_>>();
        if !atom
            .get_all_symbols(true)
            .is_subset(&aliases.iter().copied().collect())
        {
            return Err(Error::Invalid(
                "foreign symbol in retained native frame".into(),
            ));
        }
        let pulled = crate::threshold::maps::endpoint_transport::clearing::pull(
            &atom,
            &aliases,
            &self
                .roles
                .iter()
                .copied()
                .map(Atom::var)
                .collect::<Vec<_>>(),
        );
        ring.polynomial(&pulled, b)
    }
}
impl ProofBox {
    pub(in crate::threshold::maps::endpoint_transport) fn admit_retained(
        &self,
        polynomial: &Atom,
        candidate: Arc<RetainedMargin>,
        b: &Budget,
    ) -> Result<MarginProof> {
        b.check()?;
        b.rational(&candidate.scale)?;
        b.rational(&candidate.margin)?;
        let margin = &candidate.scale * &candidate.margin;
        b.rational(&margin)?;
        let proof = MarginProof {
            polynomial: polynomial.clone(),
            margin,
            evidence: MarginEvidence::Retained(candidate),
        };
        proof.replay(self, b)?;
        Ok(proof)
    }
}
fn native(e: impl std::fmt::Display) -> Error {
    Error::Native(e.to_string())
}
fn allowance(saved: &CertificateOptions, allowed: &CertificateOptions) -> Result<()> {
    // Compare explicit caller allowances; native verify_certificate still owns
    // its hard admission ceilings. Exhaustive patterns track native API changes.
    fn values(o: &CertificateOptions) -> [usize; 11] {
        let CertificateOptions {
            max_terms,
            polya_order,
            bernstein_depth,
            bernstein_nodes,
            bernstein_coefficients,
            work_limit,
            univariate_sturm_degree,
            square_residual_candidates,
            weighted_square_candidates,
            quadratic_component_candidates,
            quadratic_no_real_roots_candidates,
        } = o;
        [
            *max_terms,
            *polya_order as usize,
            *bernstein_depth,
            *bernstein_nodes,
            *bernstein_coefficients,
            *work_limit,
            *univariate_sturm_degree,
            *square_residual_candidates,
            *weighted_square_candidates,
            *quadratic_component_candidates,
            *quadratic_no_real_roots_candidates,
        ]
    }
    if values(saved)
        .into_iter()
        .zip(values(allowed))
        .any(|(s, a)| s > a)
    {
        return Err(Error::ResourceIncomplete(
            "retained proof exceeds authorized native replay allowances".into(),
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests;
