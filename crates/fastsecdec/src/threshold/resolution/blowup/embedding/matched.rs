//! Exact common presentation for one matched lower/physical principal open.
use super::super::refinement::frame::{JoinedFrameProduction, join};
use super::*;

#[derive(Clone, Debug)]
pub struct MatchedPhysicalSupport {
    support: Arc<SupportEmbedding>,
    lower: Arc<LocalizedCoverHistory>,
    frame: Arc<EtaleFrame>,
    extension: RingExtension,
    support_map: super::super::refinement::SupportFrameMap,
    lower_map: super::super::refinement::SupportFrameMap,
    lower_unit: Poly,
    pulled_cover: Option<Arc<PulledSupportCover>>,
}
impl MatchedPhysicalSupport {
    pub fn support(&self) -> &Arc<SupportEmbedding> {
        &self.support
    }
    pub fn lower(&self) -> &Arc<LocalizedCoverHistory> {
        &self.lower
    }
    pub fn frame(&self) -> &Arc<EtaleFrame> {
        &self.frame
    }
    pub fn ambient(&self) -> &Arc<EtaleFrame> {
        self.support.ambient()
    }
    pub fn extension(&self) -> &RingExtension {
        &self.extension
    }
    pub fn support_map(&self) -> &super::super::refinement::SupportFrameMap {
        &self.support_map
    }
    pub fn lower_map(&self) -> &super::super::refinement::SupportFrameMap {
        &self.lower_map
    }
    pub fn pulled_cover(&self) -> Option<&Arc<PulledSupportCover>> {
        self.pulled_cover.as_ref()
    }
    pub fn lower_unit(&self) -> &Poly {
        &self.lower_unit
    }
}

#[derive(Clone, Debug)]
pub enum MatchedPhysicalRestriction {
    Support(Arc<MatchedPhysicalSupport>),
    EmptySupport(PhysicalSupportRestriction),
}

impl PhysicalSupportCover {
    /// Consume the same lower-cover index on its issued outer lift. The lower
    /// open must already cover the whole restricted contact support; a new
    /// inverse which would delete part of that support is refused.
    pub fn restrict_matched_lower(
        self: &Arc<Self>,
        outer: Arc<LocalizedCoverHistory>,
        lower: Arc<LocalizedCoverHistory>,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<MatchedPhysicalRestriction> {
        self.restrict_matched(outer, lower, None, namespace, b)
    }
    /// Match the original lower index through a complete, issued cover pullback.
    /// No caller-supplied same-index list can replace this source association.
    pub fn restrict_pulled_lower(
        self: &Arc<Self>,
        outer: Arc<LocalizedCoverHistory>,
        lower: Arc<LocalizedCoverHistory>,
        pulled: Arc<PulledSupportCover>,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<MatchedPhysicalRestriction> {
        self.restrict_matched(outer, lower, Some(pulled), namespace, b)
    }
    fn restrict_matched(
        self: &Arc<Self>,
        outer: Arc<LocalizedCoverHistory>,
        lower: Arc<LocalizedCoverHistory>,
        pulled: Option<Arc<PulledSupportCover>>,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<MatchedPhysicalRestriction> {
        let Some(PhysicalSupportOpen::Lower { index, .. }) = self.opens().get(outer.open().index())
        else {
            return Err(Error::Invalid(
                "matched support requires a lower-cover opening",
            ));
        };
        if !Arc::ptr_eq(outer.open().cover(), self.full_cover()) || *index != lower.open().index() {
            return Err(Error::Invalid("matched support original lower/outer index"));
        }
        let (factor, prefix) = if let Some(p) = &pulled {
            let (parent, _, map) = self.embedding().source_open().ok_or(Error::Invalid(
                "matched pulled support original restriction",
            ))?;
            if !Arc::ptr_eq(p.embedding(), self.embedding())
                || !Arc::ptr_eq(p.pulled(), self.lower())
                || !Arc::ptr_eq(p.original(), lower.open().cover())
                || !Arc::ptr_eq(lower.open().source().ledger().frame(), parent.frame())
            {
                return Err(Error::Invalid("matched pulled support cover/map owners"));
            }
            (
                map.pull(lower.open().factor(), b)?,
                parent.ambient().local().ring().clone(),
            )
        } else {
            if !Arc::ptr_eq(lower.open().cover(), self.lower())
                || !Arc::ptr_eq(
                    lower.open().source().ledger().frame(),
                    self.embedding().frame(),
                )
            {
                return Err(Error::Invalid(
                    "matched support original lower/outer owners",
                ));
            }
            (
                lower.open().factor().clone(),
                self.embedding().ambient().local().ring().clone(),
            )
        };
        let support = match self.restrict_support(outer, &format!("{namespace}_outer"), b)? {
            PhysicalSupportRestriction::Support(s) => s,
            empty @ PhysicalSupportRestriction::EmptySupport { .. } => {
                return Ok(MatchedPhysicalRestriction::EmptySupport(empty));
            }
        };
        let (_, _, map) = support
            .physical_localization()
            .ok_or(Error::Invalid("matched support missing source map"))?;
        let lower_unit = map.pull(&factor, b)?;
        if !super::super::helpers::unit(support.frame().local(), &lower_unit, b)? {
            return Err(Error::ResourceIncomplete(
                "matched lower open needs further support coverage",
            ));
        }
        // Both presentations share the original OUTER coordinate prefix. The
        // outer localization can place its inverse before the old contact's
        // auxiliary slots, so the larger contact ring is not a valid prefix.
        let joined = match join(
            support.frame().clone(),
            lower.history().ledger().frame().clone(),
            &prefix,
            namespace,
            b,
        )? {
            JoinedFrameProduction::Complete(joined) => joined,
            JoinedFrameProduction::Empty { .. } => {
                return Err(Error::Invalid(
                    "proved unit lower open became empty on support",
                ));
            }
        };
        let extension = super::nested::extension_to(
            support.ambient().local().ring(),
            joined.frame.local().ring(),
            b,
        )?;
        super::nested::full_equations(
            support.ambient(),
            &joined.frame,
            &extension,
            support.equations(),
            b,
        )?;
        if support
            .ambient()
            .free_axes()
            .len()
            .checked_sub(joined.frame.free_axes().len())
            != Some(support.codimension())
        {
            return Err(Error::Invalid(
                "matched support changed relative codimension",
            ));
        }
        Ok(MatchedPhysicalRestriction::Support(Arc::new(
            MatchedPhysicalSupport {
                support,
                lower,
                frame: joined.frame.clone(),
                extension,
                support_map: joined.parent_map.clone(),
                lower_map: joined.child_map.clone(),
                lower_unit,
                pulled_cover: pulled,
            },
        )))
    }
}
