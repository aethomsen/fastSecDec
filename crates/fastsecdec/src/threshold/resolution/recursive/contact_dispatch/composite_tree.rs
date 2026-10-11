//! Private views of complete, genuinely issued lower construction trees.
//! Every deferred index remains associated with the immutable lower dispatcher.
use super::*;

#[derive(Clone, Debug)]
pub(super) enum LowerCenter {
    Recursive(Arc<RecursiveContactCenter>),
    Composite(Arc<CompositeContactCenter>),
}
impl LowerCenter {
    pub(super) fn localization(&self) -> &Arc<LocalizedCoverHistory> {
        match self {
            Self::Recursive(c) => c.localization(),
            Self::Composite(c) => c.localization(),
        }
    }
    pub(super) fn ideal(&self) -> &Ideal {
        match self {
            Self::Recursive(c) => c.ideal(),
            Self::Composite(c) => c.ideal(),
        }
    }
    pub(super) fn normals(&self) -> &[Poly] {
        match self {
            Self::Recursive(c) => c.normals(),
            Self::Composite(c) => c.normals(),
        }
    }
}

#[derive(Clone, Debug)]
pub(super) enum LowerTreeEntry {
    /// Accounted by the retained original dispatcher. This makes no statement
    /// about physical density or a strict invariant prefix.
    Deferred,
    Next {
        opening: Arc<LocalizedCoverHistory>,
        children: Arc<[Arc<LowerCoverTree>]>,
    },
    Center(LowerCenter),
}
#[derive(Clone, Debug)]
pub(super) struct LowerCoverTree {
    pub(super) owner: Arc<ContactDispatchCover>,
    pub(super) history: Arc<ResolutionHistory>,
    pub(super) cover: Arc<VerifiedOpenCover>,
    pub(super) entries: Arc<[LowerTreeEntry]>,
}
impl LowerCoverTree {
    pub(super) fn from_branch(
        owner: &FiniteContactDispatcher,
        index: usize,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let child = owner
            .recursive_branch(index)
            .ok_or(Error::Invalid("composite actual recursive lower branch"))?;
        if !child.construction_inventory_complete() {
            return Err(Error::ResourceIncomplete(
                "composite lower construction incomplete",
            ));
        }
        let Some(RecursiveBranchAscent::Finite(f)) = child.ascent() else {
            return Err(Error::Invalid("composite finite lower ascent owner"));
        };
        let cover = owner.cover().physical_cover().clone();
        let history = owner.cover().origin().auxiliary().history().clone();
        let selected = owner
            .cover()
            .contact_open_index(child.branch().contact().candidate_index())?;
        if !Arc::ptr_eq(f.branch(), child.branch())
            || !Arc::ptr_eq(f.source_open().open().source(), &history)
            || !Arc::ptr_eq(f.source_open().open().cover(), &cover)
            || f.source_open().open().index() != selected
        {
            return Err(Error::Invalid("composite original lower source opening"));
        }
        b.reserve_slots(cover.opens().len())?;
        b.reserve_slots(f.branches().len())?;
        let mut children = Vec::new();
        for branch in f.branches() {
            match branch {
                RecursiveAscentBranch::Components(c) => {
                    children.push(Self::from_components(owner.cover().clone(), c, b)?)
                }
                RecursiveAscentBranch::Composite(c) => {
                    children.push(Self::from_composite(owner.cover().clone(), c, b)?)
                }
                RecursiveAscentBranch::DeferredEmptyContact(_)
                | RecursiveAscentBranch::DeferredRecursiveChild(_) => {}
                RecursiveAscentBranch::Pending => {
                    return Err(Error::Invalid("composite accepted pending branch"));
                }
            }
        }
        let mut entries = vec![LowerTreeEntry::Deferred; cover.opens().len()];
        entries[selected] = LowerTreeEntry::Next {
            opening: f.source_open().clone(),
            children: Arc::from(children),
        };
        Ok(Arc::new(Self {
            owner: owner.cover().clone(),
            history,
            cover,
            entries: Arc::from(entries),
        }))
    }
    fn from_components(
        owner: Arc<ContactDispatchCover>,
        c: &RecursiveComponentLift,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        if !c.complete_inventory() {
            return Err(Error::Invalid("composite unfinished component lift"));
        }
        b.reserve_slots(c.work().len())?;
        let entries = c
            .work()
            .iter()
            .map(|w| match w {
                RecursiveLiftWork::Center(c) => {
                    LowerTreeEntry::Center(LowerCenter::Recursive(c.clone()))
                }
                _ => LowerTreeEntry::Deferred,
            })
            .collect::<Vec<_>>();
        Self::checked(
            owner,
            c.source_open().history().clone(),
            c.cover().full_cover().clone(),
            entries,
            b,
        )
    }
    fn from_composite(
        owner: Arc<ContactDispatchCover>,
        c: &CompositeCoverLift,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        if !c.complete_inventory() {
            return Err(Error::Invalid("composite unfinished lower cover tree"));
        }
        b.reserve_slots(c.work().len())?;
        let mut entries = Vec::new();
        for w in c.work() {
            entries.push(match w {
                CompositeLiftWork::Center(center) => {
                    LowerTreeEntry::Center(LowerCenter::Composite(center.clone()))
                }
                CompositeLiftWork::Next(next) => {
                    b.reserve_slots(next.children().len())?;
                    let mut children = Vec::new();
                    for child in next.children() {
                        children.push(Self::from_composite(
                            owner.clone(),
                            child
                                .as_deref()
                                .ok_or(Error::Invalid("composite missing lower tree"))?,
                            b,
                        )?);
                    }
                    LowerTreeEntry::Next {
                        opening: next.localization().clone(),
                        children: Arc::from(children),
                    }
                }
                _ => LowerTreeEntry::Deferred,
            });
        }
        Self::checked(
            owner,
            c.history().clone(),
            c.cover().full_cover().clone(),
            entries,
            b,
        )
    }
    fn checked(
        owner: Arc<ContactDispatchCover>,
        history: Arc<ResolutionHistory>,
        cover: Arc<VerifiedOpenCover>,
        entries: Vec<LowerTreeEntry>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        b.charge(1)?;
        if entries.len() != cover.opens().len()
            || !Arc::ptr_eq(cover.algebra(), history.ledger().frame().local())
        {
            return Err(Error::Invalid(
                "composite exact full cover/history inventory",
            ));
        }
        for (index, entry) in entries.iter().enumerate() {
            let opening = match entry {
                LowerTreeEntry::Next { opening, .. } => Some(opening),
                LowerTreeEntry::Center(c) => Some(c.localization()),
                LowerTreeEntry::Deferred => None,
            };
            if opening.is_some_and(|o| {
                !Arc::ptr_eq(o.open().source(), &history)
                    || !Arc::ptr_eq(o.open().cover(), &cover)
                    || o.open().index() != index
            }) {
                return Err(Error::Invalid("composite center exact opening/index"));
            }
        }
        Ok(Arc::new(Self {
            owner,
            history,
            cover,
            entries: Arc::from(entries),
        }))
    }
}
