//! Source-bound positive-projective quotient of native homogeneous evidence.
use super::{AliasRole, GcadError, Result, SourceProvenance, SymbolAlias};
use crate::{
    parametric::{FactorRole, FactorSemantics},
    threshold::projective::PositiveProjectivePreparation,
};
use std::{collections::BTreeSet, sync::Arc};
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    id::{Pattern, Replacement},
    prelude::{Q, Rational},
};
use symgcad::{
    algebra::{Algebra, Poly},
    domain::{DomainCertificate, certify_nonzero, derive_domain, verify_certificate},
    output::{SolveResult, Status},
    problem::NormalizedProblem,
    verify::Verification,
};

/// Branch authority for one retained original factor. No missing split index is
/// synthesized for a positive factor or a regular polynomial numerator.
#[derive(Clone, Debug)]
pub enum ProjectiveFactorAuthority {
    Inactive,
    Polynomial,
    Split {
        index: usize,
        original_scale: Rational,
        quotient_scale: Rational,
    },
    Positive {
        original: Box<DomainCertificate>,
        quotient: Box<DomainCertificate>,
    },
}

/// One actual native proof and its genuine reduced result, with the complete
/// original density retained. This is generic-open geometry, not endpoint or
/// numerical integration authority. All original/reduced ordinals remain.
/// Singular geometry is fixed over QQ; retained numerator and prefactor Atoms
/// may have symbolic coefficients. No executable coefficient binding follows.
#[derive(Debug)]
pub struct PositiveProjectiveQuotient {
    provenance: SourceProvenance,
    preparation: Arc<PositiveProjectivePreparation>,
    original: Arc<SolveResult>,
    verification: Verification,
    aliases: Vec<SymbolAlias>,
    factors: Vec<Vec<ProjectiveFactorAuthority>>,
    occupied_symbols: BTreeSet<Symbol>,
}

impl PositiveProjectiveQuotient {
    /// Derive preparation from the actual admitted graph source and retain that
    /// owner. No separately supplied density can be grafted onto its provenance.
    pub fn verify_graph(
        source: Arc<crate::threshold::represented::graph::ExactRepresentedGraphInput>,
        anchor_index: usize,
        original: Arc<SolveResult>,
    ) -> Result<Self> {
        let preparation = Arc::new(
            PositiveProjectivePreparation::anchor(source.exact().clone(), anchor_index)
                .map_err(|e| GcadError::Invalid(e.to_string()))?,
        );
        let mut owner = Self::verify(preparation, original)?;
        owner.provenance = SourceProvenance::PreparametricGraph(source);
        Ok(owner)
    }
    /// Check exact native-export aliases and complete positive domain, bind all
    /// source splits, then freshly verify the saved original evidence. The caller
    /// owns hard time/memory limits, including decoding and eager native algebra.
    pub fn verify(
        preparation: Arc<PositiveProjectivePreparation>,
        original: Arc<SolveResult>,
    ) -> Result<Self> {
        let names = (0..preparation.input().parameters().len())
            .map(|i| format!("x{i}"))
            .collect::<Vec<_>>();
        let problem = &original.problem;
        let anchor = &names[preparation.anchor_index()];
        if problem.variables != names
            || problem.solver.ordering != "given"
            || problem.effective_order() != original.order
            || problem.solver.homogeneous_anchor.as_ref() != Some(anchor)
            || original.order.first() != Some(anchor)
            || problem.solver.homogeneous_block.len() != names.len()
            || problem
                .solver
                .homogeneous_block
                .iter()
                .collect::<BTreeSet<_>>()
                != names.iter().collect::<BTreeSet<_>>()
            || problem.solver.monotone_negative_target
        {
            return Err(GcadError::Unsupported("positive-projective source needs the complete fixed native x0..xN block, given anchor-first order and full sign split".into()));
        }
        if original.status != Status::CompleteGeneric {
            return Err(GcadError::Incomplete(original.status.clone()));
        }
        let homogeneous = original
            .homogeneous
            .as_ref()
            .ok_or_else(|| GcadError::Invalid("missing native homogeneous evidence".into()))?;
        if &homogeneous.anchor != anchor
            || homogeneous.reduced.order != original.order[1..]
            || homogeneous.reduced.status != Status::CompleteGeneric
        {
            return Err(GcadError::Invalid(
                "native quotient/anchor association differs".into(),
            ));
        }
        let source = symgcad::problem::normalize(problem, &original.order).map_err(native)?;
        full_positive_domain(&source, &original.order)?;
        let quotient =
            symgcad::problem::normalize(&homogeneous.reduced.problem, &homogeneous.reduced.order)
                .map_err(native)?;
        full_positive_domain(&quotient, &homogeneous.reduced.order)?;
        let aliases = names
            .iter()
            .zip(preparation.input().parameters())
            .filter(|(name, _)| *name != anchor)
            .map(|(name, symbol)| SymbolAlias {
                symbol: *symbol,
                name: name.clone(),
                role: AliasRole::IntegrationCoordinate,
            })
            .collect::<Vec<_>>();
        let all_aliases = names
            .iter()
            .zip(preparation.input().parameters())
            .map(|(name, symbol)| (*symbol, name.clone()))
            .collect::<Vec<_>>();
        let mut source_seen = vec![false; source.split.len()];
        let mut factors = Vec::with_capacity(preparation.terms().len());
        for (original_term, term) in preparation.input().terms().iter().zip(preparation.terms()) {
            let mut row = Vec::with_capacity(term.factors().len());
            for (old, current) in original_term.factors().iter().zip(term.factors()) {
                let authority = if old.exponent().is_zero() {
                    ProjectiveFactorAuthority::Inactive
                } else if old.role() == FactorRole::Polynomial {
                    ProjectiveFactorAuthority::Polynomial
                } else {
                    let p = source_poly(old.polynomial(), &all_aliases, &source.algebra)?;
                    let q = source_poly(current.polynomial(), &all_aliases, &quotient.algebra)?;
                    match old.semantics() {
                        FactorSemantics::Positive => ProjectiveFactorAuthority::Positive {
                            original: Box::new(positive(&source, &p)?), quotient: Box::new(positive(&quotient, &q)?),
                        },
                        FactorSemantics::Causal => {
                            if p.is_zero() || q.is_zero() { return Err(GcadError::Invalid("zero causal source factor".into())); }
                            let original_scale = p.content().abs(); let quotient_scale = q.content().abs();
                            let primitive = p.div_coeff(&original_scale);
                            let index = source.split.iter().position(|s| *s == primitive)
                                .ok_or(GcadError::RequestMismatch)?;
                            if quotient.split.get(index) != Some(&q.div_coeff(&quotient_scale)) { return Err(GcadError::RequestMismatch); }
                            source_seen[index] = true;
                            ProjectiveFactorAuthority::Split { index, original_scale, quotient_scale }
                        },
                        FactorSemantics::Generic => return Err(GcadError::Unsupported("generic singular factor lacks a positive or causal projective branch owner".into())),
                    }
                };
                row.push(authority);
            }
            factors.push(row);
        }
        if source_seen.iter().any(|seen| !seen) || source.split.len() != quotient.split.len() {
            return Err(GcadError::RequestMismatch);
        }
        let verification = symgcad::verify::verify(&original)
            .map_err(|e| GcadError::Verification(e.to_string()))?;
        if !verification.verified {
            return Err(GcadError::Verification(
                "native verifier declined original homogeneous evidence".into(),
            ));
        }
        // Source expressions, including a physical numerator, are scanned once
        // per shared owner, never once per retained native cell.
        let mut occupied_symbols = preparation
            .input()
            .parameters()
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        occupied_symbols.insert(preparation.input().regulator());
        for term in preparation
            .input()
            .terms()
            .iter()
            .chain(preparation.terms())
        {
            occupied_symbols.extend(term.prefactor().get_all_symbols(true));
            for power in term.monomial_powers() {
                occupied_symbols.extend(power.get_all_symbols(true));
            }
            for factor in term.factors() {
                occupied_symbols.extend(factor.polynomial().get_all_symbols(true));
                occupied_symbols.extend(factor.exponent().get_all_symbols(true));
            }
        }
        Ok(Self {
            provenance: SourceProvenance::Exact,
            preparation,
            original,
            verification,
            aliases,
            factors,
            occupied_symbols,
        })
    }
    pub fn preparation(&self) -> &Arc<PositiveProjectivePreparation> {
        &self.preparation
    }
    pub fn provenance(&self) -> &SourceProvenance {
        &self.provenance
    }
    pub fn original_result(&self) -> &Arc<SolveResult> {
        &self.original
    }
    pub fn native_result(&self) -> &SolveResult {
        &self
            .original
            .homogeneous
            .as_ref()
            .expect("verified homogeneous owner")
            .reduced
    }
    pub fn verification(&self) -> &Verification {
        &self.verification
    }
    pub fn aliases(&self) -> &[SymbolAlias] {
        &self.aliases
    }
    pub fn factor_authorities(&self) -> &[Vec<ProjectiveFactorAuthority>] {
        &self.factors
    }
    pub(crate) fn occupied_symbols(&self) -> &BTreeSet<Symbol> {
        &self.occupied_symbols
    }
    /// Original and quotient ordinal correspondence was replayed by the native
    /// homogeneous verifier. No merged-cell inventory is substituted.
    pub fn cell_count(&self) -> usize {
        self.native_result().cells.len()
    }
}

fn native(e: impl std::fmt::Display) -> GcadError {
    GcadError::Invalid(e.to_string())
}
fn source_poly(atom: &Atom, aliases: &[(Symbol, String)], algebra: &Algebra) -> Result<Poly> {
    // A foreign coefficient named symgcad::xN must not become a coordinate
    // through alias substitution. This constructor admits fixed QQ geometry.
    if atom
        .get_all_symbols(true)
        .iter()
        .any(|symbol| !aliases.iter().any(|(source, _)| source == symbol))
    {
        return Err(GcadError::Unsupported(
            "unbound coefficient in fixed projective source factor".into(),
        ));
    }
    let value = atom.replace_multiple(aliases.iter().map(|(s, n)| {
        Replacement::new(
            Pattern::Literal(Atom::var(*s)),
            Pattern::Literal(Atom::var(
                Symbol::parse(n, "symgcad").expect("native validated alias"),
            )),
        )
    }));
    value
        .try_to_polynomial::<_, u32>(&Q, algebra.variables.clone())
        .map_err(|e| {
            GcadError::Unsupported(format!(
                "source factor is not exact rational in its original axes: {e}"
            ))
        })
}
fn full_positive_domain(problem: &NormalizedProblem, names: &[String]) -> Result<()> {
    let variables = names
        .iter()
        .map(|name| problem.algebra.parse(name).map_err(native))
        .collect::<Result<Vec<_>>>()?;
    let mut seen = vec![false; names.len()];
    for constraint in &problem.positives {
        if constraint.is_constant() {
            if constraint.get_constant() <= 0 {
                return Err(GcadError::RequestMismatch);
            }
        } else if let Some(i) = variables.iter().position(|p| p == constraint) {
            seen[i] = true;
        } else {
            return Err(GcadError::Unsupported(
                "native domain is not the complete positive source orthant".into(),
            ));
        }
    }
    if seen.iter().any(|s| !s) {
        return Err(GcadError::RequestMismatch);
    }
    Ok(())
}
fn positive(problem: &NormalizedProblem, polynomial: &Poly) -> Result<DomainCertificate> {
    let domain = derive_domain(&problem.algebra, &problem.positives, &[]).map_err(native)?;
    let proof = certify_nonzero(&problem.algebra, polynomial, &domain, &Default::default())
        .map_err(native)?
        .ok_or_else(|| {
            GcadError::Unsupported("native positive source-factor certificate unavailable".into())
        })?;
    if proof.sign != 1 {
        return Err(GcadError::Unsupported(
            "declared positive source factor has nonpositive sign".into(),
        ));
    }
    verify_certificate(
        &problem.algebra,
        polynomial,
        &problem.positives,
        &[],
        &proof,
    )
    .map_err(native)?;
    Ok(proof)
}
