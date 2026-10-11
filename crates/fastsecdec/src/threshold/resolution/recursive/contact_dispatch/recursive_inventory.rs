//! Original-contact inventory around genuine recursively issued lower problems.
use super::*;

#[derive(Clone, Debug)]
pub enum RecursiveAscentBranch {
    Pending,
    DeferredEmptyContact(ContactBranchProduction),
    Components(Box<RecursiveComponentLift>),
}
#[derive(Clone, Debug)]
pub struct RecursiveAscentFrontier {
    branch: Arc<ContactDispatchBranch>,
    child_source: Arc<MarkedIdeal>,
    lower: Arc<FiniteContactDispatcher>,
    source_open: Arc<LocalizedCoverHistory>,
    embedding: Arc<SupportEmbedding>,
    branches: Vec<RecursiveAscentBranch>,
    namespace: String,
}
#[derive(Clone, Debug)]
pub enum RecursiveBranchAscent {
    /// Completed auxiliary cosupport only; the original physical open remains.
    DeferredEmptyChild(RecursiveOutcome),
    EmptyPrincipal(Box<CoverLocalizationProduction>),
    EmptySupport(Arc<EmptyAmbientSupport>),
    Finite(Box<RecursiveAscentFrontier>),
}
impl RecursiveBranchAscent {
    pub fn complete_inventory(&self) -> bool {
        match self {
            Self::Finite(f) => f.complete_inventory(),
            Self::DeferredEmptyChild(_) | Self::EmptyPrincipal(_) | Self::EmptySupport(_) => true,
        }
    }
    pub(super) fn prepare(
        child: &RecursiveContactBranch,
        namespace: String,
        b: &mut Budget,
    ) -> Result<Self> {
        if let RecursiveOutcome::EmptyMarkedCosupport { .. } = child.child().outcome() {
            return Ok(Self::DeferredEmptyChild(child.child().outcome().clone()));
        }
        let lower = child.lower().ok_or(Error::ResourceIncomplete(
            "recursive ascent requires admitted finite lower inventory",
        ))?;
        if !lower.complete_inventory() {
            return Err(Error::ResourceIncomplete(
                "recursive ascent lower construction unfinished",
            ));
        }
        let branch = child.branch();
        let index = branch
            .owner()
            .contact_open_index(branch.contact().candidate_index())?;
        let source_open = match branch
            .owner()
            .localize(index, &format!("{namespace}_source"), b)?
        {
            CoverLocalizationProduction::Complete(o) => Arc::new(*o),
            r @ CoverLocalizationProduction::Empty { .. } => {
                return Ok(Self::EmptyPrincipal(Box::new(r)));
            }
            CoverLocalizationProduction::Incomplete { reason, .. } => {
                return Err(Error::ResourceIncomplete(reason));
            }
        };
        let embedding = match branch.embedding().restrict_ambient_open(
            source_open.clone(),
            &format!("{namespace}_support"),
            b,
        )? {
            AmbientSupportRestriction::Support(s) => s,
            AmbientSupportRestriction::EmptySupport(r) => return Ok(Self::EmptySupport(r)),
        };
        // Clone only accepted immutable evidence, with source inventory counted
        // before allocation. All original lower siblings remain retained.
        b.reserve_slots(
            lower
                .branches()
                .len()
                .checked_add(lower.ambient().len())
                .ok_or(Error::ResourceIncomplete("recursive ascent inventory size"))?,
        )?;
        Ok(Self::Finite(Box::new(RecursiveAscentFrontier {
            branch: branch.clone(),
            child_source: child.source().clone(),
            lower: Arc::new(lower.clone()),
            source_open,
            embedding,
            branches: vec![RecursiveAscentBranch::Pending; lower.branches().len()],
            namespace,
        })))
    }
}
impl RecursiveAscentFrontier {
    pub fn branch(&self) -> &Arc<ContactDispatchBranch> {
        &self.branch
    }
    pub fn lower(&self) -> &Arc<FiniteContactDispatcher> {
        &self.lower
    }
    pub fn source_open(&self) -> &Arc<LocalizedCoverHistory> {
        &self.source_open
    }
    pub fn embedding(&self) -> &Arc<SupportEmbedding> {
        &self.embedding
    }
    pub fn branches(&self) -> &[RecursiveAscentBranch] {
        &self.branches
    }
    pub fn complete_inventory(&self) -> bool {
        self.branches.iter().all(|w| match w {
            RecursiveAscentBranch::Pending => false,
            RecursiveAscentBranch::DeferredEmptyContact(_) => true,
            RecursiveAscentBranch::Components(f) => f.complete_inventory(),
        })
    }
    pub(super) fn advance_branch(
        &mut self,
        index: usize,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        if !matches!(
            self.branches
                .get(index)
                .ok_or(Error::Invalid("recursive ascent branch index"))?,
            RecursiveAscentBranch::Pending
        ) {
            return Ok(ContactDispatchAdvance::Accepted);
        }
        let result = (|| {
            Ok(
                match self
                    .lower
                    .branches()
                    .get(index)
                    .ok_or(Error::Invalid("recursive ascent lower inventory index"))?
                {
                    ContactBranchWork::Empty(r) => {
                        RecursiveAscentBranch::DeferredEmptyContact(r.clone())
                    }
                    ContactBranchWork::Components(f) => {
                        b.reserve_slots(f.work().len())?;
                        RecursiveAscentBranch::Components(Box::new(
                            RecursiveComponentLift::prepare(
                                self.branch.clone(),
                                self.child_source.clone(),
                                self.source_open.clone(),
                                self.embedding.clone(),
                                Arc::new(f.clone()),
                                format!("{}_branch{index}", self.namespace),
                                b,
                            )?,
                        ))
                    }
                    ContactBranchWork::Recursive(_) => {
                        return Err(Error::ResourceIncomplete(
                            "recursive composite-open ascent remains unadmitted",
                        ));
                    }
                    ContactBranchWork::Pending => {
                        return Err(Error::Invalid(
                            "recursive ascent missing accepted lower branch",
                        ));
                    }
                },
            )
        })();
        match result {
            Ok(work) => {
                self.branches[index] = work;
                Ok(ContactDispatchAdvance::Progress)
            }
            Err(Error::ResourceIncomplete(reason)) => {
                Ok(ContactDispatchAdvance::Incomplete { reason })
            }
            Err(e) => Err(e),
        }
    }
    pub(super) fn advance_open(
        &mut self,
        branch: usize,
        index: usize,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        let Some(RecursiveAscentBranch::Components(f)) = self.branches.get_mut(branch) else {
            return Err(Error::Invalid("recursive ascent prepared lower component"));
        };
        f.advance(index, b)
    }
}
