use super::composite_tree::LowerCenter;
use super::*;

/// Original-ancestor admissibility reached through an issued lower cover tree.
/// It is neither a selected global maximum nor completed physical resolution.
#[derive(Clone, Debug)]
pub struct CompositeContactCenter {
    context: Arc<CompositeContext>,
    lower: LowerCenter,
    cover: Arc<PhysicalSupportCover>,
    localization: Arc<LocalizedCoverHistory>,
    matched: Arc<MatchedPhysicalSupport>,
    source: Arc<MarkedIdeal>,
    ideal: Ideal,
    normals: Vec<Poly>,
    clearings: Vec<UnitClearing>,
    ancestors: Vec<DispatchAncestorOrder>,
}
impl CompositeContactCenter {
    pub fn branch(&self) -> &Arc<ContactDispatchBranch> {
        &self.context.branch
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
        &self.source
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
    pub fn lower_history(&self) -> &Arc<ResolutionHistory> {
        self.lower.localization().history()
    }
    pub fn lower_owner(&self) -> &Arc<ContactDispatchCover> {
        &self.context.lower_owner
    }
    pub(super) fn prepare(
        context: Arc<CompositeContext>,
        lower: LowerCenter,
        cover: Arc<PhysicalSupportCover>,
        localization: Arc<LocalizedCoverHistory>,
        matched: Arc<MatchedPhysicalSupport>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let (actual_cover, actual_open, _) = matched
            .support()
            .physical_localization()
            .ok_or(Error::Invalid("composite center physical restriction"))?;
        if !Arc::ptr_eq(actual_cover, &cover)
            || !Arc::ptr_eq(actual_open, &localization)
            || !Arc::ptr_eq(matched.lower(), lower.localization())
            || !matched.support().descends_from(context.branch.embedding())
            || !localization
                .history()
                .same_root(context.source_open.history())
            || localization.history().stage() != context.source_open.history().stage()
        {
            return Err(Error::Invalid("composite center actual original/lower map"));
        }
        let original = context.branch.owner().origin().auxiliary().source();
        b.reserve_slots(original.ideal().generators().len())?;
        let ideal = Ideal::new(
            matched.ambient().local().ring().clone(),
            original
                .ideal()
                .generators()
                .iter()
                .map(|p| {
                    matched
                        .support()
                        .pull_refinement(context.branch.embedding(), p, true, b)
                })
                .collect::<Result<Vec<_>>>()?,
            b,
        )?;
        let source = Arc::new(MarkedIdeal::new(ideal, original.mark(), b)?);
        let super::super::matched_ascent::MatchedCenterGeometry {
            ideal,
            normals,
            clearings,
            child,
        } = super::super::matched_ascent::lift_matched_center(
            &matched,
            &source,
            lower.ideal(),
            lower.normals(),
            b,
        )?;
        let ancestors = super::ascent::verify_ancestors(
            &context.branch,
            &localization,
            super::ascent::DispatchRestriction::Composite {
                root: context.branch.embedding().clone(),
                matched: matched.clone(),
            },
            &ideal,
            &child,
            b,
        )?;
        Ok(Arc::new(Self {
            context,
            lower,
            cover,
            localization,
            matched,
            source,
            ideal,
            normals,
            clearings,
            ancestors,
        }))
    }
}

#[derive(Clone, Debug)]
pub(super) struct CompositeContext {
    pub(super) branch: Arc<ContactDispatchBranch>,
    pub(super) source_open: Arc<LocalizedCoverHistory>,
    pub(super) lower_owner: Arc<ContactDispatchCover>,
}
