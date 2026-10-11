use super::*;
use symbolica::symbol;
#[derive(Clone, Debug)]
pub enum ContactBranchWork {
    Pending,
    Empty(ContactBranchProduction),
    Components(ContactComponentFrontier),
    Recursive(Box<RecursiveContactBranch>),
}
#[derive(Clone, Debug)]
pub enum ContactAmbientWork {
    Pending,
    Accounted(Box<CoverLocalizationProduction>),
}
/// Continuation of an actual interrupted first-center recursion. It computes
/// all admitted terminal contact branches and their original-source centers;
/// it does not compare recursive tails or issue a selected global maximum.
#[derive(Clone, Debug)]
pub struct FiniteContactDispatcher {
    cover: Arc<ContactDispatchCover>,
    branches: Vec<ContactBranchWork>,
    ambient: Vec<ContactAmbientWork>,
    namespace: String,
    pub(in super::super) recursive_coefficients: bool,
}
impl FiniteContactDispatcher {
    pub(in super::super) fn prepare(
        origin: Arc<ContactDispatchOrigin>,
        namespace: String,
        b: &mut Budget,
    ) -> Result<Self> {
        let cover = ContactDispatchCover::prepare(origin, b)?;
        b.reserve_slots(cover.origin().cover().candidates().len())?;
        b.reserve_slots(cover.physical_cover().opens().len())?;
        Ok(Self {
            branches: vec![ContactBranchWork::Pending; cover.origin().cover().candidates().len()],
            ambient: vec![ContactAmbientWork::Pending; cover.physical_cover().opens().len()],
            cover,
            namespace,
            recursive_coefficients: false,
        })
    }
    pub fn cover(&self) -> &Arc<ContactDispatchCover> {
        &self.cover
    }
    pub fn branches(&self) -> &[ContactBranchWork] {
        &self.branches
    }
    pub fn ambient(&self) -> &[ContactAmbientWork] {
        &self.ambient
    }
    pub fn complete_inventory(&self) -> bool {
        self.ambient
            .iter()
            .all(|w| matches!(w, ContactAmbientWork::Accounted(_)))
            && self.branches.iter().all(|w| match w {
                ContactBranchWork::Pending => false,
                ContactBranchWork::Empty(_) => true,
                ContactBranchWork::Components(f) => f.complete_inventory(),
                // Child completion alone is not original-ancestor ascent.
                ContactBranchWork::Recursive(q) => q.construction_inventory_complete(),
            })
    }
    pub fn advance_ambient(
        &mut self,
        index: usize,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        if !matches!(
            self.ambient
                .get(index)
                .ok_or(Error::Invalid("dispatch ambient index"))?,
            ContactAmbientWork::Pending
        ) {
            return Ok(ContactDispatchAdvance::Accepted);
        }
        let receipt =
            match self
                .cover
                .localize(index, &format!("{}_ambient{index}", self.namespace), b)
            {
                Ok(CoverLocalizationProduction::Incomplete { reason, .. })
                | Err(Error::ResourceIncomplete(reason)) => {
                    return Ok(ContactDispatchAdvance::Incomplete { reason });
                }
                Ok(r) => r,
                Err(e) => return Err(e),
            };
        self.ambient[index] = ContactAmbientWork::Accounted(Box::new(receipt));
        Ok(ContactDispatchAdvance::Progress)
    }
    /// Prepare the actual contact and its complete component cover atomically.
    pub fn advance_branch(
        &mut self,
        index: usize,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        if !matches!(
            self.branches
                .get(index)
                .ok_or(Error::Invalid("dispatch branch index"))?,
            ContactBranchWork::Pending
        ) {
            return Ok(ContactDispatchAdvance::Accepted);
        }
        let work = match self.one_branch(index, b) {
            Ok(v) => v,
            Err(Error::ResourceIncomplete(reason)) => {
                return Ok(ContactDispatchAdvance::Incomplete { reason });
            }
            Err(e) => return Err(e),
        };
        self.branches[index] = work;
        Ok(ContactDispatchAdvance::Progress)
    }
    fn one_branch(&self, index: usize, b: &mut Budget) -> Result<ContactBranchWork> {
        let namespace = format!("{}_branch{index}", self.namespace);
        let branch = match ContactDispatchBranch::prepare(
            self.cover.clone(),
            index,
            [
                symbol!(format!("{namespace}::coordinate")),
                symbol!(format!("{namespace}::inverse")),
            ],
            b,
        )? {
            ContactBranchProduction::Branch(q) => q,
            receipt @ ContactBranchProduction::EmptyContact { .. } => {
                return Ok(ContactBranchWork::Empty(receipt));
            }
            ContactBranchProduction::Incomplete { evidence, .. } => match evidence {
                ContactProduction::Incomplete { reason, .. } => {
                    return Err(Error::ResourceIncomplete(reason));
                }
                _ => return Err(Error::Invalid("dispatch incomplete contact evidence")),
            },
        };
        if self.recursive_coefficients && !branch.contact().contact().free_axes().is_empty() {
            return Ok(ContactBranchWork::Recursive(Box::new(
                RecursiveContactBranch::prepare(branch, namespace, b)?,
            )));
        }
        let components =
            ContactTerminalComponents::prepare(branch, &format!("{namespace}_components"), b)?;
        Ok(ContactBranchWork::Components(
            ContactComponentFrontier::new(components, namespace, b)?,
        ))
    }
    pub fn advance_component(
        &mut self,
        branch: usize,
        index: usize,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        match self
            .branches
            .get_mut(branch)
            .ok_or(Error::Invalid("dispatch component branch index"))?
        {
            ContactBranchWork::Components(frontier) => frontier.advance(index, b),
            _ => Err(Error::Invalid(
                "dispatch component requires actual prepared branch",
            )),
        }
    }
    pub fn recursive_branch(&self, index: usize) -> Option<&RecursiveContactBranch> {
        match self.branches.get(index) {
            Some(ContactBranchWork::Recursive(q)) => Some(q),
            _ => None,
        }
    }
    /// Advance the genuine lower constructor once; accepted parent state is
    /// unchanged on resource exhaustion.
    pub fn advance_recursive_child(
        &mut self,
        index: usize,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        let Some(ContactBranchWork::Recursive(child)) = self.branches.get_mut(index) else {
            return Err(Error::Invalid("recursive child branch index/owner"));
        };
        match child.advance_child(b) {
            Err(Error::ResourceIncomplete(reason)) => {
                Ok(ContactDispatchAdvance::Incomplete { reason })
            }
            other => other,
        }
    }
    pub fn advance_nested_ambient(
        &mut self,
        path: &[usize],
        index: usize,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        self.nested_mut(path)?.advance_ambient(index, b)
    }
    pub fn advance_nested_branch(
        &mut self,
        path: &[usize],
        index: usize,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        self.nested_mut(path)?.advance_branch(index, b)
    }
    pub fn advance_nested_component(
        &mut self,
        path: &[usize],
        branch: usize,
        index: usize,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        self.nested_mut(path)?.advance_component(branch, index, b)
    }
    pub fn advance_nested_child(
        &mut self,
        path: &[usize],
        index: usize,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        self.nested_mut(path)?.advance_recursive_child(index, b)
    }
    pub fn prepare_recursive_ascent(
        &mut self,
        branch: usize,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        self.recursive_mut(branch)?.prepare_ascent(b)
    }
    pub fn advance_recursive_ascent_branch(
        &mut self,
        branch: usize,
        lower: usize,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        self.recursive_mut(branch)?
            .ascent_mut()?
            .advance_branch(lower, b)
    }
    pub fn advance_recursive_ascent_open(
        &mut self,
        branch: usize,
        lower: usize,
        index: usize,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        self.recursive_mut(branch)?
            .ascent_mut()?
            .advance_open(lower, index, b)
    }
    pub fn prepare_nested_ascent(
        &mut self,
        path: &[usize],
        branch: usize,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        self.nested_mut(path)?.prepare_recursive_ascent(branch, b)
    }
    pub fn advance_nested_ascent_branch(
        &mut self,
        path: &[usize],
        branch: usize,
        lower: usize,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        self.nested_mut(path)?
            .advance_recursive_ascent_branch(branch, lower, b)
    }
    pub fn advance_nested_ascent_open(
        &mut self,
        path: &[usize],
        branch: usize,
        lower: usize,
        index: usize,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        self.nested_mut(path)?
            .advance_recursive_ascent_open(branch, lower, index, b)
    }
    fn recursive_mut(&mut self, index: usize) -> Result<&mut RecursiveContactBranch> {
        match self.branches.get_mut(index) {
            Some(ContactBranchWork::Recursive(q)) => Ok(q),
            _ => Err(Error::Invalid("recursive branch index/owner")),
        }
    }
    fn nested_mut(&mut self, path: &[usize]) -> Result<&mut Self> {
        let Some((&first, rest)) = path.split_first() else {
            return Ok(self);
        };
        let Some(ContactBranchWork::Recursive(child)) = self.branches.get_mut(first) else {
            return Err(Error::Invalid("nested recursive branch owner"));
        };
        child.lower_mut()?.nested_mut(rest)
    }
}
