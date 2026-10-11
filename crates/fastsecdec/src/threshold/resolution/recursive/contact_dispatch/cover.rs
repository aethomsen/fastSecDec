use super::*;
/// A complete ambient cover, with every lower maximum-order complement and the
/// captured ancestor covers retained. This is not a selected-center certificate.
#[derive(Clone, Debug)]
pub struct ContactDispatchCover {
    origin: Arc<ContactDispatchOrigin>,
    current: Arc<VerifiedOpenCover>,
    ancestors: Arc<[Arc<PhysicalSupportCover>]>,
}
impl ContactDispatchCover {
    pub fn prepare(origin: Arc<ContactDispatchOrigin>, b: &mut Budget) -> Result<Arc<Self>> {
        let stage = origin.cover();
        let count = stage
            .candidates()
            .len()
            .checked_add(stage.cover().support().generators().len())
            .ok_or(Error::ResourceIncomplete(
                "contact dispatch full cover size",
            ))?;
        b.reserve_slots(count)?;
        let mut opens = stage.cover().opens().to_vec();
        opens.extend(stage.cover().support().generators().iter().cloned());
        let current = Arc::new(
            OpenCoverCertificate {
                algebra: stage.frame().local().clone(),
                support: Ideal::new(stage.frame().local().ring().clone(), vec![], b)?,
                opens,
            }
            .verify(b)?,
        );
        b.reserve_slots(origin.levels().len())?;
        let mut ancestors = Vec::new();
        let mut lower = current.clone();
        for level in origin.levels().iter().rev() {
            let embedding = SupportEmbedding::contact(level.contact().clone(), b)?;
            // The actual original mark at this level is retained. This is a
            // coverage proof, not a replacement by the maximum-order companion.
            let cover = embedding.physical_cover(level.source().clone(), lower, b)?;
            lower = cover.full_cover().clone();
            ancestors.push(cover);
        }
        if !Arc::ptr_eq(lower.algebra(), origin.auxiliary().frame().local()) {
            return Err(Error::Invalid(
                "contact dispatch physical cover original frame",
            ));
        }
        Ok(Arc::new(Self {
            origin,
            current,
            ancestors: Arc::from(ancestors),
        }))
    }
    pub fn origin(&self) -> &Arc<ContactDispatchOrigin> {
        &self.origin
    }
    pub fn current(&self) -> &Arc<VerifiedOpenCover> {
        &self.current
    }
    /// Ordered from the closest retained contact back to the original ambient.
    pub fn ancestors(&self) -> &[Arc<PhysicalSupportCover>] {
        &self.ancestors
    }
    pub fn physical_cover(&self) -> &Arc<VerifiedOpenCover> {
        self.ancestors
            .last()
            .map_or(&self.current, |c| c.full_cover())
    }
    /// Uses the original auxiliary history. No lower-frame history is minted.
    pub fn localize(
        &self,
        index: usize,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<CoverLocalizationProduction> {
        localize_verified_cover_open(
            self.origin.auxiliary().history().clone(),
            self.physical_cover().clone(),
            index,
            namespace,
            b,
        )
    }
}
