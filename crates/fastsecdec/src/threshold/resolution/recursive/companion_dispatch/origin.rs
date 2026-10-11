use super::*;

#[derive(Clone, Debug)]
pub struct CompanionDispatchOrigin {
    coefficient: Arc<CompanionCoefficientChart>,
    child: Arc<ContactDispatchOrigin>,
    original: Arc<MarkedIdeal>,
    embedding: Arc<SupportEmbedding>,
}
impl CompanionDispatchOrigin {
    pub(in super::super) fn prepare(
        coefficient: Arc<CompanionCoefficientChart>,
        child_source: &Arc<MarkedIdeal>,
        child: Arc<ContactDispatchOrigin>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let q = coefficient.contact();
        let open = coefficient.source();
        let lower = child.auxiliary();
        if !Arc::ptr_eq(lower.source(), child_source)
            || !Arc::ptr_eq(lower.frame(), q.contact())
            || child_source.ideal() != coefficient.coefficient().coefficient().ideal()
            || child_source.mark() != coefficient.coefficient().coefficient().mark()
            || !Arc::ptr_eq(q.source(), open.contact())
            || q.source().source().as_ref() != open.companion().ideal()
            || q.source().algebraic_maximum_order() != open.companion().mark()
            || !Arc::ptr_eq(
                q.source().frame(),
                open.restriction().history().ledger().frame(),
            )
            || !Arc::ptr_eq(
                coefficient.coefficient().history(),
                open.restriction().history(),
            )
        {
            return Err(Error::Invalid(
                "companion dispatch actual original construction",
            ));
        }
        let source = open.order().current_source();
        let original = Arc::new(MarkedIdeal::new(
            open.restriction()
                .open()
                .extension()
                .ideal(source.ideal(), b)?,
            source.mark(),
            b,
        )?);
        let embedding = SupportEmbedding::contact(q.clone(), b)?;
        Ok(Arc::new(Self {
            coefficient,
            child,
            original,
            embedding,
        }))
    }
    pub fn coefficient(&self) -> &Arc<CompanionCoefficientChart> {
        &self.coefficient
    }
    pub fn child(&self) -> &Arc<ContactDispatchOrigin> {
        &self.child
    }
    pub fn original(&self) -> &Arc<MarkedIdeal> {
        &self.original
    }
    pub fn companion(&self) -> &Arc<MarkedIdeal> {
        self.coefficient.source().companion()
    }
    pub fn history(&self) -> &Arc<ResolutionHistory> {
        self.coefficient.source().restriction().history()
    }
    pub fn embedding(&self) -> &Arc<SupportEmbedding> {
        &self.embedding
    }
    /// Coverage uses the ACTIVE companion mark. Below-G complements can still
    /// meet the original source cosupport and retain physical work.
    pub fn lift_cover(
        &self,
        lower: Arc<VerifiedOpenCover>,
        b: &mut Budget,
    ) -> Result<Arc<PhysicalSupportCover>> {
        self.embedding
            .physical_cover(self.companion().clone(), lower, b)
    }
}
