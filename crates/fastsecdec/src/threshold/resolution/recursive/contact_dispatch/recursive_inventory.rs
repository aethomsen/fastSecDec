//! Original-contact inventory around genuine recursively issued lower problems.
use super::lower_anchor::LowerBranchAnchor;
use super::*;

#[derive(Clone, Debug)]
pub enum RecursiveAscentBranch {
    Pending,
    DeferredEmptyContact(ContactBranchProduction),
    Components(Box<RecursiveComponentLift>),
    Composite(Box<CompositeCoverLift>),
    DeferredRecursiveChild(RecursiveBranchAscent),
}
#[derive(Clone, Debug)]
pub struct RecursiveAscentFrontier {
    branch: Arc<ContactDispatchBranch>,
    child_source: Arc<MarkedIdeal>,
    lower: Arc<FiniteContactDispatcher>,
    lower_complete: bool,
    accepted_lower: Vec<Option<LowerBranchAnchor>>,
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
        lower.reserve_snapshot(b)?;
        b.reserve_slots(
            lower
                .branches()
                .len()
                .checked_mul(2)
                .ok_or(Error::ResourceIncomplete("recursive ascent slot count"))?,
        )?;
        Ok(Self::Finite(Box::new(RecursiveAscentFrontier {
            branch: branch.clone(),
            child_source: child.source().clone(),
            lower: Arc::new(lower.clone()),
            lower_complete: lower.complete_inventory(),
            accepted_lower: vec![None; lower.branches().len()],
            source_open,
            embedding,
            branches: vec![RecursiveAscentBranch::Pending; lower.branches().len()],
            namespace,
        })))
    }
}
impl RecursiveAscentFrontier {
    /// Count copied frontier arrays. Geometric owners and the initial/final
    /// lower snapshot are shared Arcs, so they are not copied per accepted lift.
    pub(super) fn reserve_snapshot(&self, b: &mut Budget) -> Result<()> {
        b.reserve_slots(
            self.branches
                .len()
                .checked_add(self.accepted_lower.len())
                .ok_or(Error::ResourceIncomplete("recursive ascent snapshot slots"))?,
        )?;
        for branch in &self.branches {
            b.charge(1)?;
            match branch {
                RecursiveAscentBranch::Components(c) => b.reserve_slots(c.work().len())?,
                RecursiveAscentBranch::Composite(c) => c.reserve_snapshot(b)?,
                RecursiveAscentBranch::DeferredRecursiveChild(RecursiveBranchAscent::Finite(f)) => {
                    f.reserve_snapshot(b)?
                }
                _ => {}
            }
        }
        Ok(())
    }
    pub fn branch(&self) -> &Arc<ContactDispatchBranch> {
        &self.branch
    }
    /// Retained source inventory at admission (or final completion), not live
    /// child progress. Read the actual RecursiveContactBranch::lower for status.
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
        self.lower_complete
            && self.branches.iter().all(|w| match w {
                RecursiveAscentBranch::Pending => false,
                RecursiveAscentBranch::DeferredEmptyContact(_) => true,
                RecursiveAscentBranch::Components(f) => f.complete_inventory(),
                RecursiveAscentBranch::Composite(f) => f.complete_inventory(),
                RecursiveAscentBranch::DeferredRecursiveChild(f) => f.complete_inventory(),
            })
    }
    pub(super) fn advance_branch(
        &mut self,
        index: usize,
        live: &FiniteContactDispatcher,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        match self.verify_live(live, b) {
            Ok(()) => {}
            Err(Error::ResourceIncomplete(reason)) => {
                return Ok(ContactDispatchAdvance::Incomplete { reason });
            }
            Err(e) => return Err(e),
        }
        if !matches!(
            self.branches
                .get(index)
                .ok_or(Error::Invalid("recursive ascent branch index"))?,
            RecursiveAscentBranch::Pending
        ) {
            if !self.lower_complete && live.complete_inventory() {
                if let Err(e) = live.reserve_snapshot(b) {
                    return match e {
                        Error::ResourceIncomplete(reason) => {
                            Ok(ContactDispatchAdvance::Incomplete { reason })
                        }
                        e => Err(e),
                    };
                }
                self.lower = Arc::new(live.clone());
                self.lower_complete = true;
                return Ok(ContactDispatchAdvance::Progress);
            }
            return Ok(ContactDispatchAdvance::Accepted);
        }
        let result = (|| {
            let anchor = LowerBranchAnchor::capture(
                live.branches()
                    .get(index)
                    .ok_or(Error::Invalid("recursive current lower index"))?,
            )
            .ok_or(Error::ResourceIncomplete(
                "recursive selected lower construction unfinished",
            ))?;
            let work = match live
                .branches()
                .get(index)
                .ok_or(Error::Invalid("recursive ascent lower inventory index"))?
            {
                ContactBranchWork::Empty(r) => {
                    RecursiveAscentBranch::DeferredEmptyContact(r.clone())
                }
                ContactBranchWork::Components(f) => {
                    b.reserve_slots(f.work().len())?;
                    RecursiveAscentBranch::Components(Box::new(RecursiveComponentLift::prepare(
                        self.branch.clone(),
                        self.child_source.clone(),
                        self.source_open.clone(),
                        self.embedding.clone(),
                        Arc::new(f.clone()),
                        format!("{}_branch{index}", self.namespace),
                        b,
                    )?))
                }
                ContactBranchWork::Recursive(child) => {
                    let ascent = child
                        .ascent()
                        .ok_or(Error::Invalid("composite accepted lower ascent absent"))?;
                    if matches!(ascent, RecursiveBranchAscent::Finite(_)) {
                        RecursiveAscentBranch::Composite(Box::new(CompositeCoverLift::first(
                            self.branch.clone(),
                            &self.child_source,
                            self.source_open.clone(),
                            self.embedding.clone(),
                            (live, index),
                            format!("{}_branch{index}_composite", self.namespace),
                            b,
                        )?))
                    } else {
                        RecursiveAscentBranch::DeferredRecursiveChild(ascent.clone())
                    }
                }
                ContactBranchWork::Pending => {
                    return Err(Error::ResourceIncomplete("recursive lower branch pending"));
                }
            };
            let completed_snapshot = if !self.lower_complete && live.complete_inventory() {
                live.reserve_snapshot(b)?;
                Some(Arc::new(live.clone()))
            } else {
                None
            };
            Ok((work, anchor, completed_snapshot))
        })();
        match result {
            Ok((work, anchor, completed_snapshot)) => {
                self.branches[index] = work;
                self.accepted_lower[index] = Some(anchor);
                if let Some(snapshot) = completed_snapshot {
                    self.lower = snapshot;
                    self.lower_complete = true;
                }
                Ok(ContactDispatchAdvance::Progress)
            }
            Err(Error::ResourceIncomplete(reason)) => {
                Ok(ContactDispatchAdvance::Incomplete { reason })
            }
            Err(e) => Err(e),
        }
    }
    pub(super) fn verify_live(&self, live: &FiniteContactDispatcher, b: &mut Budget) -> Result<()> {
        b.charge(1)?;
        if !Arc::ptr_eq(live.cover(), self.lower.cover())
            || live.branches().len() != self.branches.len()
            || self.accepted_lower.len() != self.branches.len()
            || live.ambient().len() != self.lower.ambient().len()
            || !Arc::ptr_eq(
                live.cover().origin().auxiliary().source(),
                &self.child_source,
            )
            || !Arc::ptr_eq(
                live.cover().origin().auxiliary().history(),
                self.lower.cover().origin().auxiliary().history(),
            )
            || (self.lower_complete && !live.complete_inventory())
        {
            return Err(Error::Invalid(
                "recursive live lower source/history/inventory regression",
            ));
        }
        for (i, anchor) in self.accepted_lower.iter().enumerate() {
            if let Some(anchor) = anchor {
                b.charge(1)?;
                if !LowerBranchAnchor::capture(&live.branches()[i])
                    .is_some_and(|now| anchor.same(&now))
                {
                    return Err(Error::Invalid(
                        "recursive accepted lower owner replacement/regression",
                    ));
                }
            }
        }
        Ok(())
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
    pub(super) fn composite_mut(&mut self, branch: usize) -> Result<&mut CompositeCoverLift> {
        match self.branches.get_mut(branch) {
            Some(RecursiveAscentBranch::Composite(c)) => Ok(c),
            _ => Err(Error::Invalid("recursive composite frontier not prepared")),
        }
    }
}

#[cfg(test)]
#[path = "composite_tests.rs"]
mod composite_tests;
