//! A sufficient same-ideal presentation in one immutable localized quotient.
//! This is neither radical extraction nor a minimal/regular generating system.
use super::super::*;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub(crate) struct VerifiedGeneratorPresentation {
    normalization: Arc<VerifiedQuotientNormalization>,
    selected: Arc<MarkedIdeal>,
    candidate_checks: usize,
}
impl VerifiedGeneratorPresentation {
    pub(crate) fn normalization(&self) -> &Arc<VerifiedQuotientNormalization> {
        &self.normalization
    }
    pub(crate) fn selected(&self) -> &Arc<MarkedIdeal> {
        &self.selected
    }
    pub(crate) fn candidate_checks(&self) -> usize {
        self.candidate_checks
    }
    pub(crate) fn prepare(
        local: Arc<LocalizedAlgebra>,
        source: Arc<MarkedIdeal>,
        b: &mut Budget,
    ) -> Result<Arc<Self>, Error> {
        let normalizer = QuotientNormalizer::prepare(local.clone(), b)?;
        let normalization = match normalizer.normalize(source, b)? {
            NormalizationOutcome::Complete(receipt) => Arc::new(*receipt),
            NormalizationOutcome::Incomplete { reason, .. } => {
                return Err(Error::ResourceIncomplete(reason));
            }
        };
        b.reserve_slots(normalization.normalized().ideal().generators().len())?;
        let mut selected = normalization.normalized().ideal().generators().to_vec();
        let mut candidate_checks = 0usize;
        // Use the native deterministic order. Every candidate is compared with
        // the retained ORIGINAL source, including its mark and actual quotient.
        for index in (0..selected.len()).rev() {
            b.reserve_slots(selected.len())?;
            let mut candidate = selected.clone();
            candidate.remove(index);
            let proposal = MarkedIdeal::new(
                Ideal::new(local.ring().clone(), candidate, b)?,
                normalization.source().mark(),
                b,
            )?;
            candidate_checks = candidate_checks
                .checked_add(1)
                .ok_or(Error::ResourceIncomplete(
                    "generator presentation check count",
                ))?;
            if normalization.verify_candidate(&proposal, b)? {
                b.reserve_slots(proposal.ideal().generators().len())?;
                selected = proposal.ideal().generators().to_vec();
            }
        }
        let selected = Arc::new(MarkedIdeal::new(
            Ideal::new(local.ring().clone(), selected, b)?,
            normalization.source().mark(),
            b,
        )?);
        if !normalization.verify_candidate(&selected, b)? {
            return Err(Error::Invalid("generator presentation final equality"));
        }
        candidate_checks = candidate_checks
            .checked_add(1)
            .ok_or(Error::ResourceIncomplete(
                "generator presentation check count",
            ))?;
        Ok(Arc::new(Self {
            normalization,
            selected,
            candidate_checks,
        }))
    }
}

#[cfg(test)]
mod tests;
