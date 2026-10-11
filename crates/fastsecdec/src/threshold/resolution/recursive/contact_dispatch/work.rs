use super::super::component_localization::pull_ideal;
use super::*;
#[derive(Clone, Debug)]
pub enum ContactComponentWork {
    Pending,
    EmptyPrincipal(Box<CoverLocalizationProduction>),
    /// The ambient open and its original density are retained.
    EmptySupport(PhysicalSupportRestriction),
    UnitChild {
        localization: Arc<LocalizedCoverHistory>,
        support: Arc<SupportEmbedding>,
        child: Ideal,
    },
    Center(Arc<DispatchedContactCenter>),
}
/// Caller-stepped localization and original-ancestor ascent for ONE issued
/// terminal contact branch. Complete inventory does not select a global center.
#[derive(Clone, Debug)]
pub struct ContactComponentFrontier {
    owner: Arc<ContactTerminalComponents>,
    work: Vec<ContactComponentWork>,
    namespace: String,
}
#[derive(Clone, Debug)]
pub enum ContactDispatchAdvance {
    Progress,
    Accepted,
    Incomplete { reason: &'static str },
}
impl ContactComponentFrontier {
    pub fn new(
        owner: Arc<ContactTerminalComponents>,
        namespace: String,
        b: &mut Budget,
    ) -> Result<Self> {
        b.reserve_slots(owner.cover().opens().len())?;
        Ok(Self {
            work: vec![ContactComponentWork::Pending; owner.cover().opens().len()],
            owner,
            namespace,
        })
    }
    pub fn owner(&self) -> &Arc<ContactTerminalComponents> {
        &self.owner
    }
    pub fn work(&self) -> &[ContactComponentWork] {
        &self.work
    }
    pub fn complete_inventory(&self) -> bool {
        self.work
            .iter()
            .all(|w| !matches!(w, ContactComponentWork::Pending))
    }
    pub fn advance(&mut self, index: usize, b: &mut Budget) -> Result<ContactDispatchAdvance> {
        let current = self
            .work
            .get(index)
            .ok_or(Error::Invalid("dispatch component frontier index"))?;
        if !matches!(current, ContactComponentWork::Pending) {
            return Ok(ContactDispatchAdvance::Accepted);
        }
        match self.one(index, b) {
            Ok(work) => {
                self.work[index] = work;
                Ok(ContactDispatchAdvance::Progress)
            }
            Err(Error::ResourceIncomplete(reason)) => {
                Ok(ContactDispatchAdvance::Incomplete { reason })
            }
            Err(e) => Err(e),
        }
    }
    fn one(&self, index: usize, b: &mut Budget) -> Result<ContactComponentWork> {
        let open =
            match self
                .owner
                .localize(index, &format!("{}_open{index}", self.namespace), b)?
            {
                CoverLocalizationProduction::Complete(o) => Arc::new(*o),
                receipt @ CoverLocalizationProduction::Empty { .. } => {
                    return Ok(ContactComponentWork::EmptyPrincipal(Box::new(receipt)));
                }
                CoverLocalizationProduction::Incomplete { reason, .. } => {
                    return Err(Error::ResourceIncomplete(reason));
                }
            };
        let support = match self.owner.cover().restrict_support(
            open.clone(),
            &format!("{}_support{index}", self.namespace),
            b,
        )? {
            PhysicalSupportRestriction::Support(s) => s,
            receipt @ PhysicalSupportRestriction::EmptySupport { .. } => {
                return Ok(ContactComponentWork::EmptySupport(receipt));
            }
        };
        let (_, _, map) = support
            .physical_localization()
            .ok_or(Error::Invalid("dispatch joined support map"))?;
        let original = self
            .owner
            .branch()
            .contact()
            .differential_coefficient()
            .ideal();
        let child = pull_ideal(map, original, b)?;
        let local = support.frame().local();
        let mut zero = true;
        for f in child.generators() {
            zero &= local.zero(f, b)?;
        }
        if zero {
            return Ok(ContactComponentWork::Center(
                DispatchedContactCenter::prepare(self.owner.clone(), open, support, b)?,
            ));
        }
        if local
            .ideal()
            .sum(&child, b)?
            .contains(&local.ring().one(), local.unit_relations(), b)?
        {
            return Ok(ContactComponentWork::UnitChild {
                localization: open,
                support,
                child,
            });
        }
        Err(Error::ResourceIncomplete(
            "dispatch terminal component is not zero or unit",
        ))
    }
}
