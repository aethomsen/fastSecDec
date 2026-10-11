pub use super::super::elimination::EliminationEvidence;
pub(super) use super::super::elimination::{auxiliary_ring, native_basis, remap};
use super::super::geometry::generators::VerifiedGeneratorPresentation;
use super::super::prepared_membership::PreparedLocalZero;
use super::super::{
    Budget, Error, Ideal, MarkedIdeal, QuotientNormalizer, VerifiedQuotientNormalization,
};
use super::RegularAlgebra;
use std::sync::Arc;
type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug)]
pub struct VerifiedAnnihilator {
    owner: Arc<RegularAlgebra>,
    source: Arc<Ideal>,
    annihilator: Arc<Ideal>,
    evidence: EliminationEvidence,
    presentation: Arc<VerifiedGeneratorPresentation>,
}
impl VerifiedAnnihilator {
    pub fn owner(&self) -> &Arc<RegularAlgebra> {
        &self.owner
    }
    pub fn source(&self) -> &Arc<Ideal> {
        &self.source
    }
    pub fn annihilator(&self) -> &Arc<Ideal> {
        &self.annihilator
    }
    pub fn evidence(&self) -> &EliminationEvidence {
        &self.evidence
    }
    /// Equivalent discovery input; source() remains the original ideal.
    pub fn selected_source(&self) -> &Arc<MarkedIdeal> {
        self.presentation.selected()
    }
    pub fn source_normalization(&self) -> &Arc<VerifiedQuotientNormalization> {
        self.presentation.normalization()
    }
    /// Independent candidate admission recomputes the original elimination;
    /// product identities or a caller-supplied Groebner basis are insufficient.
    pub fn verify_candidate(
        &self,
        candidate: &Ideal,
        namespace: &str,
        budget: &mut Budget,
    ) -> Result<bool> {
        if candidate.ring() != self.owner.local().ring() {
            return Err(Error::Invalid("component candidate ring"));
        }
        budget.reserve_slots(candidate.generators().len())?;
        for f in candidate.generators() {
            self.owner.local().supports(f)?;
            budget.poly(f)?;
        }
        let fresh =
            produce_annihilator(self.owner.clone(), self.source.clone(), namespace, budget)?;
        let normalizer = QuotientNormalizer::prepare(self.owner.local().clone(), budget)?;
        let actual = MarkedIdeal::new((*fresh.annihilator).clone(), 1, budget)?;
        let candidate = MarkedIdeal::new(candidate.clone(), 1, budget)?;
        normalizer.verify_candidate(&actual, &candidate, budget)
    }
}
pub(super) fn produce_annihilator(
    owner: Arc<RegularAlgebra>,
    source: Arc<Ideal>,
    namespace: &str,
    budget: &mut Budget,
) -> Result<Arc<VerifiedAnnihilator>> {
    let local = owner.local();
    if source.ring() != local.ring() {
        return Err(Error::Invalid("component source ring"));
    }
    for f in source.generators() {
        local.supports(f)?;
        budget.poly(f)?;
    }
    budget.reserve_slots(source.generators().len())?;
    let marked = Arc::new(MarkedIdeal::new((*source).clone(), 1, budget)?);
    let presentation = VerifiedGeneratorPresentation::prepare(local.clone(), marked, budget)?;
    let selected = presentation.selected().ideal();
    let count = selected.generators().len();
    let extended = auxiliary_ring(local.ring(), count, namespace, budget)?;
    let mut relation = extended.one();
    for (i, f) in selected.generators().iter().enumerate() {
        let mapped = remap(f, &extended, budget)?;
        relation = relation - budget.mul(&extended.coordinate(i)?, &mapped)?;
        budget.poly(&relation)?;
    }
    let count_relations = local
        .ideal()
        .generators()
        .len()
        .checked_add(local.unit_relations().len())
        .and_then(|n| n.checked_add(1))
        .ok_or(Error::ResourceIncomplete("component relation count"))?;
    budget.reserve_slots(count_relations)?;
    let mut equations = local
        .ideal()
        .generators()
        .iter()
        .chain(local.unit_relations())
        .map(|f| remap(f, &extended, budget))
        .collect::<Result<Vec<_>>>()?;
    equations.push(relation);
    let evidence = native_basis(extended, count, equations, budget)?;
    let normalizer = presentation.normalization().normalizer();
    budget.reserve_slots(evidence.basis().generators().len())?;
    let mut eliminated = Vec::new();
    for f in evidence.basis().generators() {
        if (0..count).any(|i| f.contains(i)) {
            continue;
        }
        let original = remap(f, local.ring(), budget)?;
        budget.charge(1)?;
        let reduced = original.reduce(normalizer.relation_basis().generators());
        local.supports(&reduced)?;
        budget.poly(&reduced)?;
        eliminated.push(reduced);
    }
    let annihilator = Arc::new(Ideal::new(local.ring().clone(), eliminated, budget)?);
    let products = annihilator
        .generators()
        .len()
        .checked_mul(source.generators().len())
        .ok_or(Error::ResourceIncomplete("annihilator identity count"))?;
    budget.count(products)?;
    let mut zero = PreparedLocalZero::new(local, budget)?;
    for a in annihilator.generators() {
        for f in source.generators() {
            let product = budget.mul(a, f)?;
            if !zero.zero(&product, budget)? {
                return Err(Error::Invalid(
                    "native annihilator product in original quotient",
                ));
            }
        }
    }
    Ok(Arc::new(VerifiedAnnihilator {
        owner,
        source,
        annihilator,
        evidence,
        presentation,
    }))
}
