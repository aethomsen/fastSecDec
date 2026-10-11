use super::*;
/// The actual interrupted recursive construction, including all earlier contact
/// levels. It is not a freely supplied contact-cover or selected-tail proof.
#[derive(Clone, Debug)]
pub struct ContactDispatchOrigin {
    auxiliary: Arc<AuxiliaryRecursionOrigin>,
    levels: Arc<[Arc<RecursiveLevel>]>,
    cover: Arc<ProducedContactCover>,
    presentation: Arc<RecursivePresentation>,
}
impl ContactDispatchOrigin {
    pub(in super::super) fn prepare(
        auxiliary: Arc<AuxiliaryRecursionOrigin>,
        levels: &[Arc<RecursiveLevel>],
        cover: Arc<ProducedContactCover>,
        presentation: Arc<RecursivePresentation>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        b.charge(1)?;
        b.reserve_slots(levels.len())?;
        if !Arc::ptr_eq(auxiliary.history().ledger().frame(), auxiliary.frame())
            || auxiliary.history().stage() != 0
            || !auxiliary.history().chart_path().is_empty()
            || !auxiliary.history().births().is_empty()
            || !auxiliary.history().ledger().divisors().is_empty()
        {
            return Err(Error::Invalid(
                "contact dispatch original auxiliary history",
            ));
        }
        let mut frame = auxiliary.frame();
        let mut source = auxiliary.source().as_ref();
        for (index, level) in levels.iter().enumerate() {
            b.charge(1)?;
            let q = level.contact();
            if !Arc::ptr_eq(&level.frame, frame)
                || level.source().ideal() != source.ideal()
                || level.source().mark() != source.mark()
                || (index == 0 && !Arc::ptr_eq(level.source(), auxiliary.source()))
                || !Arc::ptr_eq(q.source().frame(), frame)
                || q.source().source().as_ref() != level.presentation().effective_source().ideal()
                || q.contact().free_axes().len().checked_add(1) != Some(frame.free_axes().len())
            {
                return Err(Error::Invalid(
                    "contact dispatch original prefix association",
                ));
            }
            level.presentation().verify(frame, level.source())?;
            frame = q.contact();
            source = q.differential_coefficient();
        }
        if !Arc::ptr_eq(cover.frame(), frame)
            || presentation.original_source().ideal() != source.ideal()
            || presentation.original_source().mark() != source.mark()
            || (levels.is_empty()
                && !Arc::ptr_eq(presentation.original_source(), auxiliary.source()))
            || cover.source().as_ref() != presentation.effective_source().ideal()
            || cover.algebraic_maximum_order() < presentation.effective_source().mark()
        {
            return Err(Error::Invalid(
                "contact dispatch current source association",
            ));
        }
        presentation.verify(frame, presentation.original_source())?;
        // Overflow is not silently promoted to an invariant coordinate.
        presentation.original_maximum_order(cover.algebraic_maximum_order())?;
        Ok(Arc::new(Self {
            auxiliary,
            levels: Arc::from(levels),
            cover,
            presentation,
        }))
    }
    pub fn auxiliary(&self) -> &Arc<AuxiliaryRecursionOrigin> {
        &self.auxiliary
    }
    pub fn levels(&self) -> &[Arc<RecursiveLevel>] {
        &self.levels
    }
    pub fn cover(&self) -> &Arc<ProducedContactCover> {
        &self.cover
    }
    pub fn presentation(&self) -> &Arc<RecursivePresentation> {
        &self.presentation
    }
}
