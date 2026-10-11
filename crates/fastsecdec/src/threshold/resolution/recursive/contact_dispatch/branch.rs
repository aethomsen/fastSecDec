use super::*;
use symbolica::atom::Symbol;
/// One actual contact construction with its entire original embedding retained.
#[derive(Clone, Debug)]
pub struct ContactDispatchBranch {
    owner: Arc<ContactDispatchCover>,
    contact: Arc<ContactQuotient>,
    embedding: Arc<SupportEmbedding>,
}
#[derive(Clone, Debug)]
pub enum ContactBranchProduction {
    Branch(Arc<ContactDispatchBranch>),
    EmptyContact {
        owner: Arc<ContactDispatchCover>,
        evidence: ContactProduction,
    },
    Incomplete {
        owner: Arc<ContactDispatchCover>,
        evidence: ContactProduction,
    },
}
impl ContactDispatchBranch {
    pub fn prepare(
        owner: Arc<ContactDispatchCover>,
        index: usize,
        fresh: [Symbol; 2],
        b: &mut Budget,
    ) -> Result<ContactBranchProduction> {
        let contact =
            match construct_contact_quotient(owner.origin().cover().clone(), index, fresh, b)? {
                ContactProduction::Constructed(q) => Arc::new(*q),
                evidence @ ContactProduction::EmptyContactOpen { .. } => {
                    return Ok(ContactBranchProduction::EmptyContact { owner, evidence });
                }
                evidence @ ContactProduction::Incomplete { .. } => {
                    return Ok(ContactBranchProduction::Incomplete { owner, evidence });
                }
            };
        let mut embedding = None;
        for q in owner
            .origin()
            .levels()
            .iter()
            .map(|l| l.contact())
            .chain(std::iter::once(&contact))
        {
            b.charge(1)?;
            embedding = Some(match embedding {
                None => SupportEmbedding::contact(q.clone(), b)?,
                Some(parent) => SupportEmbedding::nested(parent, q.clone(), b)?,
            });
        }
        let embedding =
            embedding.ok_or(Error::Invalid("contact dispatch missing actual contact"))?;
        if !Arc::ptr_eq(embedding.ambient(), owner.origin().auxiliary().frame())
            || !Arc::ptr_eq(embedding.frame(), contact.contact())
        {
            return Err(Error::Invalid(
                "contact dispatch original complete embedding",
            ));
        }
        Ok(ContactBranchProduction::Branch(Arc::new(Self {
            owner,
            contact,
            embedding,
        })))
    }
    pub fn owner(&self) -> &Arc<ContactDispatchCover> {
        &self.owner
    }
    pub fn contact(&self) -> &Arc<ContactQuotient> {
        &self.contact
    }
    pub fn embedding(&self) -> &Arc<SupportEmbedding> {
        &self.embedding
    }
}
/// Exact terminal component and full ORIGINAL ambient cover. No lower history
/// is created; localizing uses the genuine original auxiliary history.
#[derive(Clone, Debug)]
pub struct ContactTerminalComponents {
    branch: Arc<ContactDispatchBranch>,
    split: Arc<VerifiedComponentSplit>,
    cover: Arc<PhysicalSupportCover>,
}
impl ContactTerminalComponents {
    pub fn prepare(
        branch: Arc<ContactDispatchBranch>,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        if !branch.contact().contact().free_axes().is_empty() {
            return Err(Error::ResourceIncomplete(
                "contact coefficient needs further dimension recursion",
            ));
        }
        let split = match produce_component_split(
            RegularAlgebra::ambient(branch.contact().contact().clone()),
            Arc::new(branch.contact().differential_coefficient().ideal().clone()),
            namespace,
            b,
        )? {
            ComponentProduction::Complete(s) => Arc::new(*s),
            ComponentProduction::Incomplete { reason, .. } => {
                return Err(Error::ResourceIncomplete(reason));
            }
        };
        let cover = branch.embedding().physical_cover(
            branch.owner().origin().auxiliary().source().clone(),
            Arc::new(split.cover().clone()),
            b,
        )?;
        Ok(Arc::new(Self {
            branch,
            split,
            cover,
        }))
    }
    pub fn branch(&self) -> &Arc<ContactDispatchBranch> {
        &self.branch
    }
    pub fn split(&self) -> &Arc<VerifiedComponentSplit> {
        &self.split
    }
    pub fn cover(&self) -> &Arc<PhysicalSupportCover> {
        &self.cover
    }
    pub fn localize(
        &self,
        index: usize,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<CoverLocalizationProduction> {
        self.cover.localize(
            self.branch.owner().origin().auxiliary().history().clone(),
            index,
            namespace,
            b,
        )
    }
}
