//! Closed scalar density on one genuinely issued endpoint chart.
//!
//! This sufficient class requires whole-box residual units and a common open
//! epsilon strip. It retains the full regulator family for native symbolic
//! subtraction; it is neither a complete cell cover nor an integral result.
mod program;
#[cfg(test)]
mod tests;
use super::{
    Error, FactorLink, IssuedEndpointChart, Result,
    factors::{self, MarginProof, ProofBox, RetainedMargin},
};
use crate::threshold::maps::regular::Budget;
use crate::{
    generation::endpoint_power_with_regulators,
    parametric::FactorSemantics,
    threshold::{
        phase::causal_negative_phase,
        regularization::{
            ConvergenceStrip,
            meromorphic::{self, MeromorphicPrefactor, MeromorphicWitness},
        },
    },
};
pub use program::RootBindings;
use std::{ops::ControlFlow, sync::Arc};
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    evaluate::OptimizationSettings,
};

#[derive(Clone, Debug)]
pub struct Limits {
    pub factors: factors::Limits,
    pub prefactors: meromorphic::Limits,
    pub max_terms: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            factors: factors::Limits::default(),
            prefactors: meromorphic::Limits::default(),
            max_terms: 100_000,
        }
    }
}
/// A proof applies to the signed numerator and positive denominator of this
/// exact chart factor, not to a polynomial with coincident display names.
#[derive(Debug)]
pub struct ResidualUnit {
    identity: usize,
    sign: i8,
    numerator: MarginProof,
    denominator: MarginProof,
}
impl ResidualUnit {
    pub fn identity(&self) -> usize {
        self.identity
    }
    pub fn sign(&self) -> i8 {
        self.sign
    }
    pub fn numerator_proof(&self) -> &MarginProof {
        &self.numerator
    }
    pub fn denominator_proof(&self) -> &MarginProof {
        &self.denominator
    }
}
#[derive(Debug)]
pub struct ClosedTerm {
    source_index: usize,
    prefactor: Atom,
    regular_body: Atom,
    powers: Vec<Atom>,
}
impl ClosedTerm {
    pub fn source_index(&self) -> usize {
        self.source_index
    }
    /// Original meromorphic prefactor times its complete native causal phase.
    pub fn prefactor(&self) -> &Atom {
        &self.prefactor
    }
    /// Smooth positive-factor powers with genuine selected-root slots retained.
    pub fn regular_body(&self) -> &Atom {
        &self.regular_body
    }
    pub fn powers(&self) -> &[Atom] {
        &self.powers
    }
}
type EvidenceProvider<'a> = dyn FnMut(&Atom) -> Result<Option<Arc<RetainedMargin>>> + 'a;

pub struct ClosedScalarChart {
    chart: IssuedEndpointChart,
    proof_box: ProofBox,
    units: Vec<ResidualUnit>,
    measure: (MarginProof, MarginProof),
    terms: Vec<ClosedTerm>,
    strip: ConvergenceStrip,
    prefactors: MeromorphicWitness,
    roots: RootBindings,
}
impl ClosedScalarChart {
    pub fn chart(&self) -> &IssuedEndpointChart {
        &self.chart
    }
    pub fn proof_box(&self) -> &ProofBox {
        &self.proof_box
    }
    pub fn units(&self) -> &[ResidualUnit] {
        &self.units
    }
    pub fn measure_proofs(&self) -> (&MarginProof, &MarginProof) {
        (&self.measure.0, &self.measure.1)
    }
    pub fn terms(&self) -> &[ClosedTerm] {
        &self.terms
    }
    pub fn strip(&self) -> &ConvergenceStrip {
        &self.strip
    }
    pub fn prefactor_witness(&self) -> &MeromorphicWitness {
        &self.prefactors
    }
    pub fn roots(&self) -> &RootBindings {
        &self.roots
    }
    pub fn regulator(&self) -> Symbol {
        self.chart
            .parent_prefix()
            .source()
            .decomposition()
            .preparation()
            .input()
            .regulator()
    }
    pub fn prepare(
        chart: IssuedEndpointChart,
        limits: &Limits,
        settings: OptimizationSettings,
        progress: impl FnMut() -> ControlFlow<()>,
    ) -> Result<Arc<Self>> {
        Self::prepare_inner(chart, limits, settings, progress, None)
    }
    /// Admit caller-retained native proofs against obligations derived from this
    /// genuine chart. Missing evidence remains unsupported; this entrypoint does
    /// not perform discovery. Positive constants retain exact Rational admission.
    ///
    /// The callback only selects a candidate. Native polynomial equality, frame,
    /// domain inclusion and certificate replay establish its authority here.
    pub fn prepare_with_unit_evidence(
        chart: IssuedEndpointChart,
        limits: &Limits,
        settings: OptimizationSettings,
        progress: impl FnMut() -> ControlFlow<()>,
        mut evidence: impl FnMut(&Atom) -> Result<Option<Arc<RetainedMargin>>>,
    ) -> Result<Arc<Self>> {
        Self::prepare_inner(chart, limits, settings, progress, Some(&mut evidence))
    }
    fn prepare_inner(
        chart: IssuedEndpointChart,
        limits: &Limits,
        settings: OptimizationSettings,
        mut progress: impl FnMut() -> ControlFlow<()>,
        mut evidence: Option<&mut EvidenceProvider<'_>>,
    ) -> Result<Arc<Self>> {
        let budget = Budget::new(limits.factors.algebra.clone());
        let mut check = || {
            budget.check()?;
            if progress().is_break() {
                Err(Error::Cancelled)
            } else {
                Ok(())
            }
        };
        check()?;
        if limits.factors.fallback_margin <= 0 {
            return Err(Error::Invalid(
                "closed residual margin must be positive".into(),
            ));
        }
        let source = chart.parent_prefix().source().decomposition().preparation();
        if source.terms().len() > limits.max_terms {
            return Err(Error::ResourceIncomplete(
                "closed scalar term allowance".into(),
            ));
        }
        let eps = source.input().regulator();
        let active = factors::active_factors(&chart)?;
        let mut signs = vec![1; chart.chart().factors().len()];
        for factor in &active {
            signs[factor.identity()] = factor.expected_sign();
        }
        let domain = ProofBox::whole_chart(&chart, &budget)?;
        let mut require = |p: &Atom| {
            if let Some(supply) = evidence.as_deref_mut()
                && symbolica::domains::rational::Rational::try_from(p.as_view()).is_err()
            {
                let candidate = supply(p)?.ok_or_else(|| {
                    Error::Unsupported("retained whole-box unit evidence unavailable".into())
                })?;
                return domain.admit_retained(p, candidate, &budget);
            }
            domain.propose(p,&limits.factors.fallback_margin,&budget)?.ok_or_else(||Error::Unsupported("closed scalar residual lacks a positive whole-box margin; refinement remains required".into()))
        };
        let mut units = Vec::new();
        for (identity, (factor, &sign)) in chart.chart().factors().iter().zip(&signs).enumerate() {
            check()?;
            units.push(ResidualUnit {
                identity,
                sign,
                numerator: require(&(Atom::num(sign) * factor.numerator()))?,
                denominator: require(factor.denominator())?,
            });
        }
        check()?;
        let measure = (
            require(chart.chart().measure().numerator())?,
            require(chart.chart().measure().denominator())?,
        );
        let mut terms = Vec::new();
        let mut meromorphic = Vec::new();
        let mut strip = ConvergenceStrip::default();
        if source.terms().len() != chart.chart().terms().len()
            || source.terms().len() != chart.transport().term_links().len()
        {
            return Err(Error::Invalid(
                "closed scalar full source inventory differs".into(),
            ));
        }
        for (index, ((term, links), powers)) in source
            .terms()
            .iter()
            .zip(chart.transport().term_links())
            .zip(chart.chart().terms())
            .enumerate()
        {
            check()?;
            if powers.source_index() != index {
                return Err(Error::Invalid(
                    "closed scalar original term order differs".into(),
                ));
            }
            let mut body =
                chart.chart().measure().numerator() / chart.chart().measure().denominator();
            let mut prefactor = term.prefactor().clone();
            let mut phase = Atom::one();
            for (link, exponent) in links.coordinates().iter().zip(term.monomial_powers()) {
                if let Some(identity) = link {
                    let f = &chart.chart().factors()[*identity];
                    body *= (f.numerator() / f.denominator()).pow(exponent);
                }
            }
            for (factor, link) in term.factors().iter().zip(links.factors()) {
                endpoint_power_with_regulators(factor.exponent(), &[eps])
                    .map_err(|e| Error::Unsupported(e.to_string()))?;
                match *link {
                    FactorLink::Inactive => {}
                    FactorLink::LazyPolynomial => {
                        if source
                            .coordinates()
                            .iter()
                            .any(|s| factor.polynomial().get_all_symbols(true).contains(s))
                        {
                            return Err(Error::Unsupported("closed scalar class requires coordinate-independent numerator; use the retained numerator coefficient owner for a physical numerator".into()));
                        }
                        prefactor *= factor.polynomial().pow(factor.exponent());
                    }
                    FactorLink::Cleared(identity) => {
                        let f = &chart.chart().factors()[identity];
                        let sign = signs[identity];
                        if sign < 0 {
                            if factor.semantics() != FactorSemantics::Causal {
                                return Err(Error::Unsupported(
                                    "negative closed factor requires native causal semantics"
                                        .into(),
                                ));
                            }
                            phase *= causal_negative_phase(factor.exponent(), &[eps])
                                .map_err(|e| Error::Native(e.to_string()))?;
                        }
                        body *= (Atom::num(sign) * f.numerator() / f.denominator())
                            .pow(factor.exponent());
                    }
                }
            }
            for power in powers.powers() {
                strip
                    .admit(power, eps)
                    .map_err(|e| Error::Unsupported(e.to_string()))?;
            }
            meromorphic.push(
                MeromorphicPrefactor::admit(prefactor.clone(), eps, limits.prefactors)
                    .map_err(prefactor_error)?,
            );
            terms.push(ClosedTerm {
                source_index: index,
                prefactor: prefactor * phase,
                regular_body: body,
                powers: powers.powers().to_vec(),
            });
        }
        let mut interrupted = None;
        let witness = MeromorphicWitness::construct(
            meromorphic,
            strip.lower(),
            strip.upper(),
            limits.prefactors,
            |_| match check() {
                Ok(()) => ControlFlow::Continue(()),
                Err(e) => {
                    interrupted = Some(e);
                    ControlFlow::Break(())
                }
            },
        );
        if let Some(e) = interrupted {
            return Err(e);
        }
        let prefactors = witness.map_err(prefactor_error)?;
        check()?;
        let roots = program::prepare(&chart, settings, &budget)?;
        check()?;
        Ok(Arc::new(Self {
            chart,
            proof_box: domain,
            units,
            measure,
            terms,
            strip,
            prefactors,
            roots,
        }))
    }
}

fn prefactor_error(e: meromorphic::Error) -> Error {
    match e {
        meromorphic::Error::Unsupported(s) => Error::Unsupported(s.into()),
        meromorphic::Error::Invalid(s) => Error::Invalid(s.into()),
        meromorphic::Error::ResourceIncomplete(s) => Error::ResourceIncomplete(s.into()),
        meromorphic::Error::Cancelled => Error::Cancelled,
        meromorphic::Error::Native(s) => Error::Native(s),
    }
}
