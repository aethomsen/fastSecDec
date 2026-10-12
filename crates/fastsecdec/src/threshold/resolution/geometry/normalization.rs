use super::super::{Budget, Error, Ideal, LocalizedAlgebra, MarkedIdeal, Poly};
use std::sync::Arc;
type Result<T> = std::result::Result<T, Error>;

/// Caller-owned native relation basis for exactly one immutable localization.
/// F4 internals require an external resource guard; no internal worker is owned.
#[derive(Clone, Debug)]
pub struct QuotientNormalizer {
    local: Arc<LocalizedAlgebra>,
    relations: Ideal,
    basis: Arc<Ideal>,
}
#[derive(Clone, Debug, Default)]
pub struct NormalizationProgress {
    pub stage: &'static str,
    pub original_basis: Option<Arc<Ideal>>,
    pub candidate: Option<Arc<MarkedIdeal>>,
    pub candidate_basis: Option<Arc<Ideal>>,
    pub original_generators_checked: usize,
    pub candidate_generators_checked: usize,
    pub operations: usize,
    pub ideal_slots: usize,
}
#[derive(Clone, Debug)]
pub struct VerifiedQuotientNormalization {
    normalizer: Arc<QuotientNormalizer>,
    source: Arc<MarkedIdeal>,
    normalized: Arc<MarkedIdeal>,
    progress: NormalizationProgress,
}
impl VerifiedQuotientNormalization {
    pub fn normalizer(&self) -> &Arc<QuotientNormalizer> {
        &self.normalizer
    }
    pub fn source(&self) -> &Arc<MarkedIdeal> {
        &self.source
    }
    pub fn normalized(&self) -> &Arc<MarkedIdeal> {
        &self.normalized
    }
    pub fn progress(&self) -> &NormalizationProgress {
        &self.progress
    }
    /// Verify a proposal against this receipt's exact retained original source
    /// and localization. The original basis is already checked and immutable;
    /// candidate construction and both directions of membership are repeated.
    pub fn verify_candidate(&self, candidate: &MarkedIdeal, budget: &mut Budget) -> Result<bool> {
        self.normalizer.admit_candidate(&self.source, candidate)?;
        let original = self.progress.original_basis.as_ref().ok_or(Error::Invalid(
            "complete normalization has no original basis",
        ))?;
        self.normalizer
            .verify_against_original(&self.source, candidate, original, budget)
    }
}
#[derive(Clone, Debug)]
pub enum NormalizationOutcome {
    Complete(Box<VerifiedQuotientNormalization>),
    Incomplete {
        normalizer: Arc<QuotientNormalizer>,
        source: Arc<MarkedIdeal>,
        reason: &'static str,
        progress: NormalizationProgress,
    },
}
fn basis(ideal: &Ideal, budget: &mut Budget) -> Result<Arc<Ideal>> {
    budget.count(ideal.generators().len())?;
    for f in ideal.generators() {
        budget.poly(f)?;
    }
    let native = super::super::native_basis::checked(ideal.generators(), budget)?;
    Ok(Arc::new(Ideal::new(ideal.ring().clone(), native, budget)?))
}
fn reduces_to_zero(f: &Poly, basis: &Ideal, budget: &mut Budget) -> Result<bool> {
    budget.poly(f)?;
    budget.charge(1)?;
    let remainder = f.reduce(basis.generators());
    budget.poly(&remainder)?;
    Ok(remainder.is_zero())
}
impl QuotientNormalizer {
    pub fn prepare(local: Arc<LocalizedAlgebra>, budget: &mut Budget) -> Result<Arc<Self>> {
        let units = Ideal::new(
            local.ring().clone(),
            local.unit_relations().to_vec(),
            budget,
        )?;
        let relations = local.ideal().sum(&units, budget)?;
        let basis = basis(&relations, budget)?;
        Ok(Arc::new(Self {
            local,
            relations,
            basis,
        }))
    }
    pub fn local(&self) -> &Arc<LocalizedAlgebra> {
        &self.local
    }
    pub fn relation_basis(&self) -> &Arc<Ideal> {
        &self.basis
    }
    /// Same Q-polynomial ring/guards and identical mark; parameters never enter
    /// a rational-function coefficient field. No resolved/real-fiber claim.
    pub fn normalize(
        self: &Arc<Self>,
        source: Arc<MarkedIdeal>,
        budget: &mut Budget,
    ) -> Result<NormalizationOutcome> {
        if source.ideal().ring() != self.local.ring() {
            return Err(Error::Invalid("normalization ring mismatch"));
        }
        for f in source.ideal().generators() {
            self.local.supports(f)?;
        }
        let mut progress = NormalizationProgress::default();
        let result = self.run(&source, budget, &mut progress);
        progress.operations = budget.operations();
        progress.ideal_slots = budget.ideal_slots();
        match result {
            Ok(normalized) => Ok(NormalizationOutcome::Complete(Box::new(
                VerifiedQuotientNormalization {
                    normalizer: self.clone(),
                    source,
                    normalized,
                    progress,
                },
            ))),
            Err(Error::ResourceIncomplete(reason)) => Ok(NormalizationOutcome::Incomplete {
                normalizer: self.clone(),
                source,
                reason,
                progress,
            }),
            Err(e) => Err(e),
        }
    }
    fn run(
        &self,
        source: &MarkedIdeal,
        budget: &mut Budget,
        progress: &mut NormalizationProgress,
    ) -> Result<Arc<MarkedIdeal>> {
        progress.stage = "original quotient basis";
        let total = self.relations.sum(source.ideal(), budget)?;
        // This original quotient presentation can make direct native Lex F4
        // expensive even when the resulting ideal is the unit ideal. Use the
        // explicit verified strategy here; all other basis calls stay direct.
        let native = super::super::native_basis::checked_via_fglm(total.generators(), budget)?;
        let original = Arc::new(Ideal::new(total.ring().clone(), native, budget)?);
        progress.original_basis = Some(original.clone());
        progress.stage = "native quotient normal forms";
        budget.reserve_slots(original.generators().len())?;
        let mut representatives = Vec::new();
        for f in original.generators() {
            budget.charge(1)?;
            let p = f.reduce(self.basis.generators());
            budget.poly(&p)?;
            self.local.supports(&p)?;
            representatives.push(p);
        }
        let candidate = Arc::new(MarkedIdeal::new(
            Ideal::new(self.local.ring().clone(), representatives, budget)?,
            source.mark(),
            budget,
        )?);
        progress.candidate = Some(candidate.clone());
        progress.stage = "candidate quotient basis";
        let total = self.relations.sum(candidate.ideal(), budget)?;
        let candidate_basis = basis(&total, budget)?;
        progress.candidate_basis = Some(candidate_basis.clone());
        progress.stage = "two-way quotient ideal equality";
        for f in source.ideal().generators() {
            if !reduces_to_zero(f, &candidate_basis, budget)? {
                return Err(Error::Invalid("normalized ideal loses a source generator"));
            }
            progress.original_generators_checked += 1;
        }
        for f in candidate.ideal().generators() {
            if !reduces_to_zero(f, &original, budget)? {
                return Err(Error::Invalid("normalized ideal adds a foreign generator"));
            }
            progress.candidate_generators_checked += 1;
        }
        progress.stage = "complete";
        Ok(candidate)
    }
    /// Independent admission of supplied generators; no normalization heuristic
    /// can replace the same-localization two-way membership check.
    pub fn verify_candidate(
        &self,
        source: &MarkedIdeal,
        candidate: &MarkedIdeal,
        budget: &mut Budget,
    ) -> Result<bool> {
        self.admit_candidate(source, candidate)?;
        let original = basis(&self.relations.sum(source.ideal(), budget)?, budget)?;
        self.verify_against_original(source, candidate, &original, budget)
    }
    fn admit_candidate(&self, source: &MarkedIdeal, candidate: &MarkedIdeal) -> Result<()> {
        if source.mark() != candidate.mark()
            || source.ideal().ring() != self.local.ring()
            || candidate.ideal().ring() != self.local.ring()
        {
            return Err(Error::Invalid("candidate normalization ring/mark"));
        }
        for f in source
            .ideal()
            .generators()
            .iter()
            .chain(candidate.ideal().generators())
        {
            self.local.supports(f)?;
        }
        Ok(())
    }
    fn verify_against_original(
        &self,
        source: &MarkedIdeal,
        candidate: &MarkedIdeal,
        original: &Ideal,
        budget: &mut Budget,
    ) -> Result<bool> {
        if original.ring() != self.local.ring() {
            return Err(Error::Invalid("retained normalization basis ring"));
        }
        // A retained proof avoids allocation/construction, not the current
        // caller's polynomial-size admission for native reduction operands.
        budget.count(original.generators().len())?;
        for polynomial in original.generators() {
            budget.poly(polynomial)?;
        }
        let b = basis(&self.relations.sum(candidate.ideal(), budget)?, budget)?;
        for f in source.ideal().generators() {
            if !reduces_to_zero(f, &b, budget)? {
                return Ok(false);
            }
        }
        for f in candidate.ideal().generators() {
            if !reduces_to_zero(f, original, budget)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
