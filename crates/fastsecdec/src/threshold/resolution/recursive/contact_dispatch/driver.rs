use super::*;
use symbolica::symbol;
#[derive(Clone, Debug)]
pub enum ContactBranchWork {
    Pending,
    Empty(ContactBranchProduction),
    Components(ContactComponentFrontier),
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
}
