//! Complete original-source cover accounting for one actual lower component
//! frontier. Deferred entries retain physical density and are not resolutions.
use super::*;

#[derive(Clone, Debug)]
pub enum RecursiveLiftWork {
    Pending,
    EmptyPrincipal(Box<CoverLocalizationProduction>),
    EmptySupport(PhysicalSupportRestriction),
    DeferredBelowOriginal(Arc<LocalizedCoverHistory>),
    DeferredLower {
        localization: Arc<LocalizedCoverHistory>,
        lower: Box<ContactComponentWork>,
    },
    Center(Arc<RecursiveContactCenter>),
}
#[derive(Clone, Debug)]
pub struct RecursiveComponentLift {
    branch: Arc<ContactDispatchBranch>,
    child_source: Arc<MarkedIdeal>,
    source_open: Arc<LocalizedCoverHistory>,
    lower: Arc<ContactComponentFrontier>,
    pulled: Arc<PulledSupportCover>,
    cover: Arc<PhysicalSupportCover>,
    work: Vec<RecursiveLiftWork>,
    namespace: String,
}
impl RecursiveComponentLift {
    pub(super) fn prepare(
        branch: Arc<ContactDispatchBranch>,
        child_source: Arc<MarkedIdeal>,
        source_open: Arc<LocalizedCoverHistory>,
        embedding: Arc<SupportEmbedding>,
        lower: Arc<ContactComponentFrontier>,
        namespace: String,
        b: &mut Budget,
    ) -> Result<Self> {
        let lower_origin = lower.owner().branch().owner().origin();
        let (actual_parent, actual_open, _) = embedding
            .source_open()
            .ok_or(Error::Invalid("recursive lift initial source restriction"))?;
        if !lower.complete_inventory()
            || !Arc::ptr_eq(lower_origin.auxiliary().source(), &child_source)
            || !Arc::ptr_eq(lower_origin.auxiliary().frame(), branch.contact().contact())
            || !Arc::ptr_eq(actual_parent, branch.embedding())
            || !Arc::ptr_eq(actual_open, &source_open)
        {
            return Err(Error::Invalid(
                "recursive lift genuine lower/original ownership",
            ));
        }
        let pulled = PulledSupportCover::prepare(
            embedding.clone(),
            lower.owner().cover().full_cover().clone(),
            b,
        )?;
        let original = branch.owner().origin().auxiliary().source();
        let current = Arc::new(MarkedIdeal::new(
            source_open.open().extension().ideal(original.ideal(), b)?,
            original.mark(),
            b,
        )?);
        let cover = embedding.physical_cover(current, pulled.pulled().clone(), b)?;
        b.reserve_slots(cover.opens().len())?;
        Ok(Self {
            branch,
            child_source,
            source_open,
            lower,
            pulled,
            work: vec![RecursiveLiftWork::Pending; cover.opens().len()],
            cover,
            namespace,
        })
    }
    pub fn lower(&self) -> &Arc<ContactComponentFrontier> {
        &self.lower
    }
    pub fn pulled_cover(&self) -> &Arc<PulledSupportCover> {
        &self.pulled
    }
    pub fn cover(&self) -> &Arc<PhysicalSupportCover> {
        &self.cover
    }
    pub fn source_open(&self) -> &Arc<LocalizedCoverHistory> {
        &self.source_open
    }
    pub fn work(&self) -> &[RecursiveLiftWork] {
        &self.work
    }
    pub fn complete_inventory(&self) -> bool {
        self.work
            .iter()
            .all(|w| !matches!(w, RecursiveLiftWork::Pending))
    }
    pub(super) fn advance(
        &mut self,
        index: usize,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        if !matches!(
            self.work
                .get(index)
                .ok_or(Error::Invalid("recursive lift open index"))?,
            RecursiveLiftWork::Pending
        ) {
            return Ok(ContactDispatchAdvance::Accepted);
        }
        match self.one(index, b) {
            Ok(w) => {
                self.work[index] = w;
                Ok(ContactDispatchAdvance::Progress)
            }
            Err(Error::ResourceIncomplete(reason)) => {
                Ok(ContactDispatchAdvance::Incomplete { reason })
            }
            Err(e) => Err(e),
        }
    }
    fn one(&self, index: usize, b: &mut Budget) -> Result<RecursiveLiftWork> {
        let open = match self.cover.localize(
            self.source_open.history().clone(),
            index,
            &format!("{}_open{index}", self.namespace),
            b,
        )? {
            CoverLocalizationProduction::Complete(o) => Arc::new(*o),
            r @ CoverLocalizationProduction::Empty { .. } => {
                return Ok(RecursiveLiftWork::EmptyPrincipal(Box::new(r)));
            }
            CoverLocalizationProduction::Incomplete { reason, .. } => {
                return Err(Error::ResourceIncomplete(reason));
            }
        };
        let lower_index = match self
            .cover
            .opens()
            .get(index)
            .ok_or(Error::Invalid("recursive lift covered index"))?
        {
            PhysicalSupportOpen::BelowMark { .. } => {
                return Ok(RecursiveLiftWork::DeferredBelowOriginal(open));
            }
            PhysicalSupportOpen::Lower { index, .. } => *index,
        };
        let lower = self
            .lower
            .work()
            .get(lower_index)
            .ok_or(Error::Invalid("recursive lift original lower index"))?;
        let ContactComponentWork::Center(center) = lower else {
            if matches!(lower, ContactComponentWork::Pending) {
                return Err(Error::Invalid(
                    "recursive lift accepted unfinished lower work",
                ));
            }
            return Ok(RecursiveLiftWork::DeferredLower {
                localization: open,
                lower: Box::new(lower.clone()),
            });
        };
        let matched = match self.cover.restrict_pulled_lower(
            open.clone(),
            center.localization().clone(),
            self.pulled.clone(),
            &format!("{}_match{index}", self.namespace),
            b,
        )? {
            MatchedPhysicalRestriction::Support(s) => s,
            MatchedPhysicalRestriction::EmptySupport(r) => {
                return Ok(RecursiveLiftWork::EmptySupport(r));
            }
        };
        Ok(RecursiveLiftWork::Center(RecursiveContactCenter::prepare(
            (self.branch.clone(), &self.child_source),
            center.clone(),
            self.source_open.clone(),
            self.cover.clone(),
            open,
            matched,
            b,
        )?))
    }
}
