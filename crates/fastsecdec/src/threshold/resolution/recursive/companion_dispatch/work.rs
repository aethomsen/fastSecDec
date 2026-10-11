use super::*;

#[derive(Clone, Debug)]
pub enum CompanionOuterWork {
    Pending,
    EmptyPrincipal(Box<CoverLocalizationProduction>),
    /// The original I can still be singular here; only G's cosupport is empty.
    DeferredBelowCompanion {
        localization: Arc<LocalizedCoverHistory>,
    },
    /// A completed auxiliary child slot does not remove the ambient density.
    DeferredChild {
        localization: Arc<LocalizedCoverHistory>,
        child_index: usize,
    },
    /// Only the contact support is empty; the ambient density remains work.
    DeferredEmptyContact(MatchedPhysicalRestriction),
    Center(Arc<CompanionDispatchedCenter>),
}
impl CompanionOuterWork {
    /// These outcomes retain actual physical work without issuing a center.
    /// `EmptyPrincipal` alone proves the whole principal open is empty.
    pub fn is_deferred_physical_work(&self) -> bool {
        matches!(
            self,
            Self::DeferredBelowCompanion { .. }
                | Self::DeferredChild { .. }
                | Self::DeferredEmptyContact(_)
        )
    }
}
/// Full outer cover for one actual lower component frontier. Completion means
/// every open has a retained outcome, including all lower-invariant work.
#[derive(Clone, Debug)]
pub struct CompanionOuterFrontier {
    origin: Arc<CompanionDispatchOrigin>,
    child_owner: Arc<ContactTerminalComponents>,
    child_work: Arc<[ContactComponentWork]>,
    cover: Arc<PhysicalSupportCover>,
    work: Vec<CompanionOuterWork>,
    namespace: String,
}
impl CompanionOuterFrontier {
    pub(super) fn prepare(
        origin: Arc<CompanionDispatchOrigin>,
        child: &ContactComponentFrontier,
        namespace: String,
        b: &mut Budget,
    ) -> Result<Self> {
        if !child.complete_inventory()
            || !Arc::ptr_eq(child.owner().branch().owner().origin(), origin.child())
        {
            return Err(Error::Invalid(
                "outer frontier actual completed child association",
            ));
        }
        let cover = origin.lift_cover(child.owner().cover().full_cover().clone(), b)?;
        b.reserve_slots(child.work().len())?;
        b.reserve_slots(cover.opens().len())?;
        Ok(Self {
            origin,
            child_owner: child.owner().clone(),
            child_work: child.work().to_vec().into(),
            work: vec![CompanionOuterWork::Pending; cover.opens().len()],
            cover,
            namespace,
        })
    }
    pub fn origin(&self) -> &Arc<CompanionDispatchOrigin> {
        &self.origin
    }
    pub fn child_owner(&self) -> &Arc<ContactTerminalComponents> {
        &self.child_owner
    }
    pub fn child_work(&self) -> &[ContactComponentWork] {
        &self.child_work
    }
    pub fn cover(&self) -> &Arc<PhysicalSupportCover> {
        &self.cover
    }
    pub fn work(&self) -> &[CompanionOuterWork] {
        &self.work
    }
    pub fn complete_inventory(&self) -> bool {
        self.work
            .iter()
            .all(|w| !matches!(w, CompanionOuterWork::Pending))
    }
    pub fn advance(&mut self, index: usize, b: &mut Budget) -> Result<ContactDispatchAdvance> {
        if !matches!(
            self.work
                .get(index)
                .ok_or(Error::Invalid("outer component index"))?,
            CompanionOuterWork::Pending
        ) {
            return Ok(ContactDispatchAdvance::Accepted);
        }
        let proposed = match self.one(index, b) {
            Ok(v) => v,
            Err(Error::ResourceIncomplete(reason)) => {
                return Ok(ContactDispatchAdvance::Incomplete { reason });
            }
            Err(e) => return Err(e),
        };
        self.work[index] = proposed;
        Ok(ContactDispatchAdvance::Progress)
    }
    fn one(&self, index: usize, b: &mut Budget) -> Result<CompanionOuterWork> {
        let local = match self.cover.localize(
            self.origin.history().clone(),
            index,
            &format!("{}_open{index}", self.namespace),
            b,
        )? {
            CoverLocalizationProduction::Complete(v) => Arc::new(*v),
            empty @ CoverLocalizationProduction::Empty { .. } => {
                return Ok(CompanionOuterWork::EmptyPrincipal(Box::new(empty)));
            }
            CoverLocalizationProduction::Incomplete { reason, .. } => {
                return Err(Error::ResourceIncomplete(reason));
            }
        };
        let lower_index = match self
            .cover
            .opens()
            .get(index)
            .ok_or(Error::Invalid("outer cover inventory index"))?
        {
            PhysicalSupportOpen::Lower { index, .. } => *index,
            PhysicalSupportOpen::BelowMark { .. } => {
                return Ok(CompanionOuterWork::DeferredBelowCompanion {
                    localization: local,
                });
            }
        };
        let lower = match self
            .child_work
            .get(lower_index)
            .ok_or(Error::Invalid("outer child inventory index"))?
        {
            ContactComponentWork::Center(c) => c.clone(),
            ContactComponentWork::Pending => {
                return Err(Error::Invalid("outer child inventory unexpectedly pending"));
            }
            _ => {
                return Ok(CompanionOuterWork::DeferredChild {
                    localization: local,
                    child_index: lower_index,
                });
            }
        };
        let matched = match self.cover.restrict_matched_lower(
            local.clone(),
            lower.localization().clone(),
            &format!("{}_matched{index}", self.namespace),
            b,
        )? {
            MatchedPhysicalRestriction::Support(v) => v,
            empty @ MatchedPhysicalRestriction::EmptySupport(_) => {
                return Ok(CompanionOuterWork::DeferredEmptyContact(empty));
            }
        };
        Ok(CompanionOuterWork::Center(
            CompanionDispatchedCenter::prepare(
                self.origin.clone(),
                lower,
                self.cover.clone(),
                local,
                matched,
                b,
            )?,
        ))
    }
}
