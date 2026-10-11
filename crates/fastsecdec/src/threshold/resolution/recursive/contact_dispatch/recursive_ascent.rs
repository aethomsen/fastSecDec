use super::*;

/// Local original-ancestor center from an actual dimension-decreased problem.
/// This is admissibility evidence, never selected-global maximum authority.
#[derive(Clone, Debug)]
pub struct RecursiveContactCenter {
    branch: Arc<ContactDispatchBranch>,
    lower: Arc<DispatchedContactCenter>,
    source_open: Arc<LocalizedCoverHistory>,
    cover: Arc<PhysicalSupportCover>,
    localization: Arc<LocalizedCoverHistory>,
    matched: Arc<MatchedPhysicalSupport>,
    original: Arc<MarkedIdeal>,
    ideal: Ideal,
    normals: Vec<Poly>,
    clearings: Vec<UnitClearing>,
    ancestors: Vec<DispatchAncestorOrder>,
}
impl RecursiveContactCenter {
    pub fn branch(&self) -> &Arc<ContactDispatchBranch> {
        &self.branch
    }
    pub fn lower(&self) -> &Arc<DispatchedContactCenter> {
        &self.lower
    }
    pub fn source_open(&self) -> &Arc<LocalizedCoverHistory> {
        &self.source_open
    }
    pub fn cover(&self) -> &Arc<PhysicalSupportCover> {
        &self.cover
    }
    pub fn localization(&self) -> &Arc<LocalizedCoverHistory> {
        &self.localization
    }
    pub fn matched(&self) -> &Arc<MatchedPhysicalSupport> {
        &self.matched
    }
    pub fn source(&self) -> &Arc<MarkedIdeal> {
        &self.original
    }
    pub fn frame(&self) -> &Arc<EtaleFrame> {
        self.matched.ambient()
    }
    pub fn history(&self) -> &Arc<ResolutionHistory> {
        self.localization.history()
    }
    pub fn ideal(&self) -> &Ideal {
        &self.ideal
    }
    pub fn normals(&self) -> &[Poly] {
        &self.normals
    }
    pub fn clearings(&self) -> &[UnitClearing] {
        &self.clearings
    }
    pub fn ancestors(&self) -> &[DispatchAncestorOrder] {
        &self.ancestors
    }
    pub(crate) fn prepare(
        original_child: (Arc<ContactDispatchBranch>, &Arc<MarkedIdeal>),
        lower: Arc<DispatchedContactCenter>,
        source_open: Arc<LocalizedCoverHistory>,
        cover: Arc<PhysicalSupportCover>,
        localization: Arc<LocalizedCoverHistory>,
        matched: Arc<MatchedPhysicalSupport>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let (branch, child_source) = original_child;
        let origin = branch.owner().origin();
        let (original_embedding, first, _) = cover
            .embedding()
            .source_open()
            .ok_or(Error::Invalid("recursive ascent source-open embedding"))?;
        let (actual_cover, actual_open, _) = matched
            .support()
            .physical_localization()
            .ok_or(Error::Invalid("recursive ascent physical restriction"))?;
        let pulled = matched
            .pulled_cover()
            .ok_or(Error::Invalid("recursive ascent exact pulled cover"))?;
        if !Arc::ptr_eq(original_embedding, branch.embedding())
            || !Arc::ptr_eq(first, &source_open)
            || !Arc::ptr_eq(source_open.open().cover(), branch.owner().physical_cover())
            || source_open.open().index()
                != branch
                    .owner()
                    .contact_open_index(branch.contact().candidate_index())?
            || !Arc::ptr_eq(source_open.open().source(), origin.auxiliary().history())
            || !Arc::ptr_eq(
                lower
                    .components()
                    .branch()
                    .owner()
                    .origin()
                    .auxiliary()
                    .source(),
                child_source,
            )
            || !Arc::ptr_eq(
                lower
                    .components()
                    .branch()
                    .owner()
                    .origin()
                    .auxiliary()
                    .frame(),
                branch.contact().contact(),
            )
            || !Arc::ptr_eq(pulled.original(), lower.components().cover().full_cover())
            || !Arc::ptr_eq(actual_cover, &cover)
            || !Arc::ptr_eq(actual_open, &localization)
            || !Arc::ptr_eq(localization.open().source(), source_open.history())
            || !Arc::ptr_eq(matched.lower(), lower.localization())
            || !Arc::ptr_eq(matched.lower_map().source(), lower.frame())
        {
            return Err(Error::Invalid(
                "recursive ascent actual original/lower/source association",
            ));
        }
        let parent = origin.auxiliary().source();
        let at = source_open.open().extension().ideal(parent.ideal(), b)?;
        let original = Arc::new(MarkedIdeal::new(
            localization.open().extension().ideal(&at, b)?,
            parent.mark(),
            b,
        )?);
        let super::super::matched_ascent::MatchedCenterGeometry {
            ideal,
            normals,
            clearings,
            child,
        } = super::super::matched_ascent::lift_matched_center(
            &matched,
            &original,
            lower.ideal(),
            lower.normals(),
            b,
        )?;
        let ancestors = super::ascent::verify_ancestors(
            &branch,
            &localization,
            super::ascent::DispatchRestriction::Matched(matched.clone()),
            &ideal,
            &child,
            b,
        )?;
        Ok(Arc::new(Self {
            branch,
            lower,
            source_open,
            cover,
            localization,
            matched,
            original,
            ideal,
            normals,
            clearings,
            ancestors,
        }))
    }
}
