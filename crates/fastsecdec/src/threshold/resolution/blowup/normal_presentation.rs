//! Optional native presentation simplification of an already issued center.
//! Equality of generators does not certify a regular parameter system; the
//! unchanged adaptation producer still checks all minors and the whole cover.
use super::super::*;
use super::general::CheckedRecursiveCenter;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct CenterNormalPresentation {
    source: Arc<CheckedRecursiveCenter>,
    normalization: Arc<VerifiedQuotientNormalization>,
    selected: Arc<MarkedIdeal>,
    candidate_checks: usize,
}
impl CenterNormalPresentation {
    pub fn source(&self) -> &Arc<CheckedRecursiveCenter> {
        &self.source
    }
    pub fn normalization(&self) -> &Arc<VerifiedQuotientNormalization> {
        &self.normalization
    }
    pub fn selected(&self) -> &Arc<MarkedIdeal> {
        &self.selected
    }
    pub fn candidate_checks(&self) -> usize {
        self.candidate_checks
    }
}

pub(super) fn prepare(
    source: Arc<CheckedRecursiveCenter>,
    b: &mut Budget,
) -> Result<Arc<CenterNormalPresentation>, Error> {
    let local = source.history().ledger().frame().local();
    let center = Arc::new(MarkedIdeal::new(source.ideal().clone(), 1, b)?);
    let normalizer = QuotientNormalizer::prepare(local.clone(), b)?;
    let normalization = match normalizer.normalize(center, b)? {
        NormalizationOutcome::Complete(receipt) => Arc::new(*receipt),
        NormalizationOutcome::Incomplete { reason, .. } => {
            return Err(Error::ResourceIncomplete(reason));
        }
    };
    b.reserve_slots(normalization.normalized().ideal().generators().len())?;
    let mut selected = normalization.normalized().ideal().generators().to_vec();
    let mut candidate_checks = 0usize;
    // Native canonical order is deterministic. This bounded greedy operation
    // need not find a minimal generating set; no success is inferred from its
    // size. Every removal is checked in the original localized quotient.
    for index in (0..selected.len()).rev() {
        b.reserve_slots(selected.len())?;
        let mut candidate = selected.clone();
        candidate.remove(index);
        let proposal = MarkedIdeal::new(Ideal::new(local.ring().clone(), candidate, b)?, 1, b)?;
        candidate_checks = candidate_checks
            .checked_add(1)
            .ok_or(Error::ResourceIncomplete("center presentation check count"))?;
        if normalizer.verify_candidate(normalization.source(), &proposal, b)? {
            b.reserve_slots(proposal.ideal().generators().len())?;
            selected = proposal.ideal().generators().to_vec();
        }
    }
    if selected.is_empty() || selected.len() > source.history().ledger().frame().free_axes().len() {
        return Err(Error::ResourceIncomplete(
            "center presentation needs other generators",
        ));
    }
    let selected = Arc::new(MarkedIdeal::new(
        Ideal::new(local.ring().clone(), selected, b)?,
        1,
        b,
    )?);
    if !normalizer.verify_candidate(normalization.source(), &selected, b)? {
        return Err(Error::Invalid("center presentation final equality"));
    }
    candidate_checks = candidate_checks
        .checked_add(1)
        .ok_or(Error::ResourceIncomplete("center presentation check count"))?;
    Ok(Arc::new(CenterNormalPresentation {
        source,
        normalization,
        selected,
        candidate_checks,
    }))
}
