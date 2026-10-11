use super::*;

/// A real child dispatcher with its outer source and complete G-cover retained.
/// The separate outer-center work is required before continuation can finish.
#[derive(Clone, Debug)]
pub struct CompanionContactDispatcher {
    origin: Arc<CompanionDispatchOrigin>,
    child: FiniteContactDispatcher,
    outer_cover: Arc<PhysicalSupportCover>,
    outer: Vec<ContactAmbientWork>,
    namespace: String,
    branches: Vec<Option<CompanionOuterFrontier>>,
}
impl CompanionContactDispatcher {
    pub(in super::super) fn prepare(
        coefficient: Arc<CompanionCoefficientChart>,
        child_source: &Arc<MarkedIdeal>,
        child: FiniteContactDispatcher,
        namespace: String,
        b: &mut Budget,
    ) -> Result<Self> {
        let origin = CompanionDispatchOrigin::prepare(
            coefficient,
            child_source,
            child.cover().origin().clone(),
            b,
        )?;
        let outer_cover = origin.lift_cover(child.cover().physical_cover().clone(), b)?;
        b.reserve_slots(outer_cover.opens().len())?;
        b.reserve_slots(child.branches().len())?;
        let branches = vec![None; child.branches().len()];
        Ok(Self {
            branches,
            origin,
            child,
            outer: vec![ContactAmbientWork::Pending; outer_cover.opens().len()],
            outer_cover,
            namespace,
        })
    }
    pub fn origin(&self) -> &Arc<CompanionDispatchOrigin> {
        &self.origin
    }
    pub fn child(&self) -> &FiniteContactDispatcher {
        &self.child
    }
    pub fn outer_cover(&self) -> &Arc<PhysicalSupportCover> {
        &self.outer_cover
    }
    pub fn outer(&self) -> &[ContactAmbientWork] {
        &self.outer
    }
    /// This only accounts for the whole outer open. It cannot establish a
    /// terminal factor, zero density, residual drop, or selected global tail.
    pub fn advance_outer(
        &mut self,
        index: usize,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        if !matches!(
            self.outer
                .get(index)
                .ok_or(Error::Invalid("companion outer open index"))?,
            ContactAmbientWork::Pending
        ) {
            return Ok(ContactDispatchAdvance::Accepted);
        }
        let receipt = match self.outer_cover.localize(
            self.origin.history().clone(),
            index,
            &format!("{}_outer{index}", self.namespace),
            b,
        ) {
            Ok(CoverLocalizationProduction::Incomplete { reason, .. })
            | Err(Error::ResourceIncomplete(reason)) => {
                return Ok(ContactDispatchAdvance::Incomplete { reason });
            }
            Ok(receipt) => receipt,
            Err(e) => return Err(e),
        };
        self.outer[index] = ContactAmbientWork::Accounted(Box::new(receipt));
        Ok(ContactDispatchAdvance::Progress)
    }
    pub fn advance_child_ambient(
        &mut self,
        index: usize,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        self.child.advance_ambient(index, b)
    }
    pub fn advance_child_branch(
        &mut self,
        index: usize,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        self.child.advance_branch(index, b)
    }
    pub fn advance_child_component(
        &mut self,
        branch: usize,
        index: usize,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        self.child.advance_component(branch, index, b)
    }
    pub fn branches(&self) -> &[Option<CompanionOuterFrontier>] {
        &self.branches
    }
    /// Retained inventory only: deferred G complements are still physical work.
    pub fn complete_inventory(&self) -> bool {
        self.child.complete_inventory()
            && self
                .outer
                .iter()
                .all(|v| matches!(v, ContactAmbientWork::Accounted(_)))
            && self
                .child
                .branches()
                .iter()
                .enumerate()
                .all(|(i, w)| match w {
                    ContactBranchWork::Empty(_) => true,
                    ContactBranchWork::Components(_) => self.branches[i]
                        .as_ref()
                        .is_some_and(CompanionOuterFrontier::complete_inventory),
                    ContactBranchWork::Pending => false,
                    ContactBranchWork::Recursive(_) => false,
                })
    }
    pub fn prepare_outer_branch(
        &mut self,
        index: usize,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        if self
            .branches
            .get(index)
            .ok_or(Error::Invalid("outer dispatch branch index"))?
            .is_some()
        {
            return Ok(ContactDispatchAdvance::Accepted);
        }
        let child = match self
            .child
            .branches()
            .get(index)
            .ok_or(Error::Invalid("outer dispatch child index"))?
        {
            ContactBranchWork::Components(v) => v,
            ContactBranchWork::Empty(_) => return Ok(ContactDispatchAdvance::Accepted),
            ContactBranchWork::Pending => {
                return Err(Error::Invalid("outer dispatch child branch pending"));
            }
            ContactBranchWork::Recursive(_) => {
                return Ok(ContactDispatchAdvance::Incomplete {
                    reason: "recursive companion outer ascent pending",
                });
            }
        };
        let next = match CompanionOuterFrontier::prepare(
            self.origin.clone(),
            child,
            format!("{}_lift{index}", self.namespace),
            b,
        ) {
            Ok(v) => v,
            Err(Error::ResourceIncomplete(reason)) => {
                return Ok(ContactDispatchAdvance::Incomplete { reason });
            }
            Err(e) => return Err(e),
        };
        self.branches[index] = Some(next);
        Ok(ContactDispatchAdvance::Progress)
    }
    pub fn advance_outer_component(
        &mut self,
        branch: usize,
        index: usize,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        self.branches
            .get_mut(branch)
            .ok_or(Error::Invalid("outer dispatch branch index"))?
            .as_mut()
            .ok_or(Error::Invalid("outer dispatch branch unprepared"))?
            .advance(index, b)
    }
}
