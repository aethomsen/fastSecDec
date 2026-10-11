//! Same-map controlled transforms of the retained auxiliary power identity.
//! Original physical factors and marks never use this presentation reduction.
use super::super::{
    coefficient::{SupportedMarkedTransform, controlled_supported, transform_supported},
    helpers::unit as is_unit,
    support::{StrictContactSupport, StrictSupportOpen},
};
use super::*;

#[derive(Clone, Debug)]
pub struct PowerLevelTransform {
    presentation: Arc<RecursivePresentation>,
    original: Arc<SupportedMarkedTransform>,
    normalized: Arc<SupportedMarkedTransform>,
    effective: Arc<SupportedMarkedTransform>,
    original_companion: Arc<SupportedMarkedTransform>,
    effective_companion: Arc<SupportedMarkedTransform>,
    unit: Arc<SupportedMarkedTransform>,
    normalizer: Arc<QuotientNormalizer>,
}
impl PowerLevelTransform {
    pub fn presentation(&self) -> &Arc<RecursivePresentation> {
        &self.presentation
    }
    pub fn original(&self) -> &Arc<SupportedMarkedTransform> {
        &self.original
    }
    pub fn normalized(&self) -> &Arc<SupportedMarkedTransform> {
        &self.normalized
    }
    pub fn effective(&self) -> &Arc<SupportedMarkedTransform> {
        &self.effective
    }
    pub fn original_companion(&self) -> &Arc<SupportedMarkedTransform> {
        &self.original_companion
    }
    pub fn effective_companion(&self) -> &Arc<SupportedMarkedTransform> {
        &self.effective_companion
    }
    pub fn unit(&self) -> &Arc<SupportedMarkedTransform> {
        &self.unit
    }
    pub fn normalizer(&self) -> &Arc<QuotientNormalizer> {
        &self.normalizer
    }
    pub(super) fn prepare(
        level: &Arc<OriginalLowerLevel>,
        support: Arc<StrictContactSupport>,
        open: Arc<StrictSupportOpen>,
        original: Arc<SupportedMarkedTransform>,
        original_companion: Arc<SupportedMarkedTransform>,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let presentation = level
            .center()
            .lift_receipt()
            .ok_or(Error::Invalid("power transform lacks original level"))?
            .level()
            .presentation()
            .clone();
        presentation.verify(level.center().frame(), level.center().source())?;
        let power = presentation
            .power()
            .ok_or(Error::Invalid("power transform lacks power receipt"))?;
        if !Arc::ptr_eq(power.history(), level.history())
            || !Arc::ptr_eq(original.source(), presentation.original_source())
            || !Arc::ptr_eq(
                original_companion.source(),
                level
                    .companion()
                    .ok_or(Error::Invalid("power original companion"))?,
            )
            || !Arc::ptr_eq(original.support(), &support)
            || !Arc::ptr_eq(original_companion.support(), &support)
            || !Arc::ptr_eq(original.open(), &open)
            || !Arc::ptr_eq(original_companion.open(), &open)
        {
            return Err(Error::Invalid("power transform original ownership"));
        }
        let normalized = controlled_supported(
            support.clone(),
            open.clone(),
            presentation.normalized_source().clone(),
            &format!("{namespace}_normalized"),
            b,
        )?;
        let effective = controlled_supported(
            support.clone(),
            open.clone(),
            presentation.effective_source().clone(),
            &format!("{namespace}_effective"),
            b,
        )?;
        let effective_companion = controlled_supported(
            support.clone(),
            open.clone(),
            level
                .effective_companion()
                .ok_or(Error::Invalid("power effective companion"))?
                .clone(),
            &format!("{namespace}_effective_G"),
            b,
        )?;
        let unit_source = Arc::new(MarkedIdeal::new(
            Ideal::new(
                power.source().ideal().ring().clone(),
                vec![power.unit().clone()],
                b,
            )?,
            1,
            b,
        )?);
        // Ordinary total pullback of the existing unit, not a controlled mark-1 transform.
        let unit = transform_supported(
            support,
            open.clone(),
            unit_source,
            0,
            &format!("{namespace}_unit"),
            b,
        )?;
        let local = open.frame().local();
        let normalizer = QuotientNormalizer::prepare(local.clone(), b)?;
        if !normalizer.verify_candidate(original.target(), normalized.target(), b)? {
            return Err(Error::Invalid(
                "transformed original normalization identity",
            ));
        }
        let unit_value = single(&unit, power.unit())?;
        if !is_unit(local, &unit_value.quotient, b)? {
            return Err(Error::Invalid("power identity lost original unit"));
        }
        for (p, g) in [
            (&normalized, &effective),
            (&original_companion, &effective_companion),
        ] {
            if p.source().mark()
                != g.source()
                    .mark()
                    .checked_mul(power.power())
                    .ok_or(Error::ResourceIncomplete("power transform mark"))?
            {
                return Err(Error::Invalid("power transform marks"));
            }
            let p = single(p, &power.source().ideal().generators()[0])?;
            let g = single(g, power.base())?;
            for (left, right, u) in [
                (&p.total, &g.total, &unit_value.total),
                (&p.quotient, &g.quotient, &unit_value.quotient),
            ] {
                let powered = b.power(right, power.power())?;
                let product = b.mul(u, &powered)?;
                if !local.zero(&(left - &product), b)? {
                    return Err(Error::Invalid("same-map controlled power identity"));
                }
            }
        }
        Ok(Arc::new(Self {
            presentation,
            original,
            normalized,
            effective,
            original_companion,
            effective_companion,
            unit,
            normalizer,
        }))
    }
    pub(super) fn associated(
        &self,
        level: &OriginalLowerLevel,
        original: &Arc<SupportedMarkedTransform>,
        companion: &Arc<SupportedMarkedTransform>,
    ) -> bool {
        level
            .center()
            .lift_receipt()
            .is_some_and(|r| Arc::ptr_eq(r.level().presentation(), &self.presentation))
            && Arc::ptr_eq(original, &self.original)
            && Arc::ptr_eq(companion, &self.original_companion)
            && self
                .presentation
                .origin()
                .is_some_and(|o| Arc::ptr_eq(o.history(), level.history()))
    }
}
fn single<'a>(
    transform: &'a SupportedMarkedTransform,
    key: &Poly,
) -> Result<&'a super::super::coefficient::SupportedGenerator> {
    if transform.generators().len() != 1 || transform.generators()[0].source != *key {
        return Err(Error::Invalid("power transform source generator key"));
    }
    Ok(&transform.generators()[0])
}
