//! One genuinely constructed lower coefficient problem. Its empty auxiliary
//! history is distinct from all captured physical ancestor histories.
use super::*;

#[derive(Clone, Debug)]
pub enum RecursiveContactChildState {
    Preparing,
    EmptyMarkedCosupport,
    ZeroIdeal,
    DirectCenter,
    ComponentOpensRequired,
    FiniteContact,
}

#[derive(Clone, Debug)]
pub struct RecursiveContactBranch {
    branch: Arc<ContactDispatchBranch>,
    source: Arc<MarkedIdeal>,
    child: BoundaryFreeFirstCenter,
    lower: Option<Box<FiniteContactDispatcher>>,
    namespace: String,
    ascent: Option<RecursiveBranchAscent>,
}

impl RecursiveContactBranch {
    pub(super) fn prepare(
        branch: Arc<ContactDispatchBranch>,
        namespace: String,
        b: &mut Budget,
    ) -> Result<Self> {
        b.charge(1)?;
        let q = branch.contact();
        if q.contact().free_axes().is_empty()
            || q.contact().free_axes().len().checked_add(1)
                != Some(q.source().frame().free_axes().len())
            || !Arc::ptr_eq(q.source(), branch.owner().origin().cover())
            || !Arc::ptr_eq(branch.embedding().frame(), q.contact())
        {
            return Err(Error::Invalid("recursive contact actual coefficient edge"));
        }
        // This Arc is issued ONCE by the real coefficient constructor. It is
        // shared by the child and checked again before forwarding any result.
        let source = Arc::new(q.differential_coefficient().clone());
        let child =
            BoundaryFreeFirstCenter::from_contact_branch(&branch, source.clone(), &namespace)?;
        Ok(Self {
            branch,
            source,
            child,
            lower: None,
            namespace,
            ascent: None,
        })
    }
    pub fn branch(&self) -> &Arc<ContactDispatchBranch> {
        &self.branch
    }
    pub fn source(&self) -> &Arc<MarkedIdeal> {
        &self.source
    }
    pub fn child(&self) -> &BoundaryFreeFirstCenter {
        &self.child
    }
    pub fn lower(&self) -> Option<&FiniteContactDispatcher> {
        self.lower.as_deref()
    }
    pub fn state(&self) -> RecursiveContactChildState {
        if self.lower.is_some() {
            return RecursiveContactChildState::FiniteContact;
        }
        match self.child.outcome() {
            RecursiveOutcome::Pending => RecursiveContactChildState::Preparing,
            RecursiveOutcome::EmptyMarkedCosupport { .. } => {
                RecursiveContactChildState::EmptyMarkedCosupport
            }
            RecursiveOutcome::TopLevelZeroIdeal { .. } => RecursiveContactChildState::ZeroIdeal,
            RecursiveOutcome::Center(_) => RecursiveContactChildState::DirectCenter,
            RecursiveOutcome::ComponentOpensRequired(_) => {
                RecursiveContactChildState::ComponentOpensRequired
            }
            RecursiveOutcome::MultipleContactOpensRequired { .. } => {
                RecursiveContactChildState::Preparing
            }
        }
    }
    /// Completes only the LOWER work. Original-ancestor cover/ascent must still
    /// finish before the parent construction can be complete.
    pub fn child_inventory_complete(&self) -> bool {
        match self.state() {
            RecursiveContactChildState::EmptyMarkedCosupport
            | RecursiveContactChildState::ZeroIdeal
            | RecursiveContactChildState::DirectCenter => true,
            RecursiveContactChildState::FiniteContact => {
                self.lower.as_ref().is_some_and(|q| q.complete_inventory())
            }
            _ => false,
        }
    }
    pub(super) fn advance_child(&mut self, b: &mut Budget) -> Result<ContactDispatchAdvance> {
        if self.lower.is_some() {
            return Ok(ContactDispatchAdvance::Accepted);
        }
        let mut next = self.clone();
        match next.child.advance(b)? {
            RecursiveAdvance::Incomplete { reason } => {
                // The existing child explicitly publishes a checked component
                // cover as an incomplete continuation. Retain that issued
                // evidence; ordinary resource failures leave the old state.
                if matches!(
                    next.child.outcome(),
                    RecursiveOutcome::ComponentOpensRequired(_)
                ) && next.child.accepted_steps() > self.child.accepted_steps()
                {
                    *self = next;
                }
                return Ok(ContactDispatchAdvance::Incomplete { reason });
            }
            RecursiveAdvance::Progress => {}
            RecursiveAdvance::Complete => {
                if let Some(mut lower) = next
                    .child
                    .finite_contact_dispatcher(format!("{}_finite", next.namespace), b)?
                {
                    let auxiliary = lower.cover().origin().auxiliary();
                    if !Arc::ptr_eq(auxiliary.source(), &next.source)
                        || !Arc::ptr_eq(auxiliary.frame(), next.branch.contact().contact())
                    {
                        return Err(Error::Invalid("recursive contact issued child source"));
                    }
                    lower.recursive_coefficients = true;
                    next.lower = Some(Box::new(lower));
                }
            }
        }
        *self = next;
        Ok(ContactDispatchAdvance::Progress)
    }
    pub fn ascent(&self) -> Option<&RecursiveBranchAscent> {
        self.ascent.as_ref()
    }
    pub fn construction_inventory_complete(&self) -> bool {
        self.child_inventory_complete()
            && self.ascent.as_ref().is_some_and(|a| a.complete_inventory())
    }
    pub(super) fn prepare_ascent(&mut self, b: &mut Budget) -> Result<ContactDispatchAdvance> {
        if self.ascent.is_some() {
            return Ok(ContactDispatchAdvance::Accepted);
        }
        match RecursiveBranchAscent::prepare(self, format!("{}_ascent", self.namespace), b) {
            Ok(a) => {
                self.ascent = Some(a);
                Ok(ContactDispatchAdvance::Progress)
            }
            Err(Error::ResourceIncomplete(reason)) => {
                Ok(ContactDispatchAdvance::Incomplete { reason })
            }
            Err(e) => Err(e),
        }
    }
    pub(super) fn ascent_mut(&mut self) -> Result<&mut RecursiveAscentFrontier> {
        match self.ascent.as_mut() {
            Some(RecursiveBranchAscent::Finite(f)) => Ok(f),
            _ => Err(Error::Invalid(
                "recursive ascent finite frontier not issued",
            )),
        }
    }
    pub(super) fn lower_mut(&mut self) -> Result<&mut FiniteContactDispatcher> {
        self.lower
            .as_deref_mut()
            .ok_or(Error::Invalid("recursive lower dispatcher not issued"))
    }
}
