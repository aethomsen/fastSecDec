//! A proof-preserving auxiliary presentation, never an unmarked ideal relabel.
use super::*;
#[derive(Clone, Debug)]
enum PresentationProof {
    Normalization,
    PrincipalPower {
        origin: Arc<AuxiliaryRecursionOrigin>,
        power: Arc<PrincipalPowerPresentation>,
    },
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn presentation_refuses_foreign_source_normalization_and_history() {
        let mut b = Budget::new(Limits::default());
        let center = super::super::power_tests::cusp_center(&mut b);
        let level = center.child().lift_receipt().unwrap().level();
        let original = level.presentation();
        assert!(original.power().is_some());
        let foreign_source = Arc::new(original.original_source().as_ref().clone());
        assert!(matches!(
            original.verify(&level.frame, &foreign_source),
            Err(Error::Invalid(_))
        ));
        let wrong_mark = Arc::new(
            MarkedIdeal::new(
                original.original_source().ideal().clone(),
                original.effective_source().mark(),
                &b,
            )
            .unwrap(),
        );
        assert!(matches!(
            original.verify(&level.frame, &wrong_mark),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            original.original_maximum_order(usize::MAX),
            Err(Error::ResourceIncomplete(_))
        ));
        let mut changed = original.as_ref().clone();
        let NormalizationOutcome::Complete(repeated) = original
            .normalization
            .normalizer()
            .normalize(original.original_source().clone(), &mut b)
            .unwrap()
        else {
            panic!()
        };
        changed.normalization = Arc::new(*repeated);
        assert!(matches!(
            changed.verify(&level.frame, level.source()),
            Err(Error::Invalid(_))
        ));
        let mut changed = original.as_ref().clone();
        let foreign =
            AuxiliaryRecursionOrigin::prepare(level.frame.clone(), level.source.clone(), &mut b)
                .unwrap();
        changed.proof = PresentationProof::PrincipalPower {
            origin: foreign,
            power: original.power().unwrap().clone(),
        };
        assert!(matches!(
            changed.verify(&level.frame, level.source()),
            Err(Error::Invalid(_))
        ));
    }
}
#[derive(Clone, Debug)]
pub struct RecursivePresentation {
    normalization: Arc<VerifiedQuotientNormalization>,
    proof: PresentationProof,
}
impl RecursivePresentation {
    pub(super) fn prepare(
        normalization: Arc<VerifiedQuotientNormalization>,
        origin: Option<Arc<AuxiliaryRecursionOrigin>>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let proof = if let Some(origin) = origin {
            if !Arc::ptr_eq(origin.source(), normalization.source())
                || !Arc::ptr_eq(origin.frame().local(), normalization.normalizer().local())
                || !Arc::ptr_eq(origin.history().ledger().frame(), origin.frame())
                || origin.history().stage() != 0
                || !origin.history().chart_path().is_empty()
                || !origin.history().births().is_empty()
                || !origin.history().ledger().divisors().is_empty()
            {
                return Err(Error::Invalid("initial principal presentation origin"));
            }
            match produce_principal_power_presentation(
                origin.history().clone(),
                normalization.normalized().clone(),
                b,
            )? {
                PrincipalPowerOutcome::Reduced(power) => {
                    PresentationProof::PrincipalPower { origin, power }
                }
                PrincipalPowerOutcome::Unchanged(_) => PresentationProof::Normalization,
                PrincipalPowerOutcome::Incomplete { reason, .. } => {
                    return Err(Error::ResourceIncomplete(reason));
                }
            }
        } else {
            PresentationProof::Normalization
        };
        Ok(Arc::new(Self {
            normalization,
            proof,
        }))
    }
    pub fn normalization(&self) -> &Arc<VerifiedQuotientNormalization> {
        &self.normalization
    }
    pub fn original_source(&self) -> &Arc<MarkedIdeal> {
        self.normalization.source()
    }
    pub fn normalized_source(&self) -> &Arc<MarkedIdeal> {
        self.normalization.normalized()
    }
    pub fn effective_source(&self) -> &Arc<MarkedIdeal> {
        self.power()
            .map_or(self.normalization.normalized(), |p| p.reduced())
    }
    pub fn power(&self) -> Option<&Arc<PrincipalPowerPresentation>> {
        match &self.proof {
            PresentationProof::Normalization => None,
            PresentationProof::PrincipalPower { power, .. } => Some(power),
        }
    }
    pub fn origin(&self) -> Option<&Arc<AuxiliaryRecursionOrigin>> {
        match &self.proof {
            PresentationProof::Normalization => None,
            PresentationProof::PrincipalPower { origin, .. } => Some(origin),
        }
    }
    pub fn original_maximum_order(&self, effective: usize) -> Result<usize> {
        effective
            .checked_mul(self.power().map_or(1, |p| p.power()))
            .ok_or(Error::ResourceIncomplete(
                "principal presentation maximum order",
            ))
    }
    pub(crate) fn verify(&self, frame: &Arc<EtaleFrame>, source: &Arc<MarkedIdeal>) -> Result<()> {
        if !Arc::ptr_eq(self.original_source(), source)
            || !Arc::ptr_eq(self.normalization.normalizer().local(), frame.local())
        {
            return Err(Error::Invalid("recursive presentation source/frame"));
        }
        if let PresentationProof::PrincipalPower { origin, power } = &self.proof
            && (!Arc::ptr_eq(origin.source(), source)
                || !Arc::ptr_eq(origin.frame(), frame)
                || !Arc::ptr_eq(power.history(), origin.history())
                || !Arc::ptr_eq(power.source(), self.normalization.normalized())
                || power.power().checked_mul(power.reduced().mark()) != Some(power.source().mark()))
        {
            return Err(Error::Invalid("recursive principal presentation owner"));
        }
        Ok(())
    }
}
