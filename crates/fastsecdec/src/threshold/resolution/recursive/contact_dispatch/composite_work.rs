//! One caller-stepped physical cover for an immutable lower construction node.
use super::composite_tree::{LowerCoverTree, LowerTreeEntry};
use super::*;

#[derive(Clone, Debug)]
pub enum CompositeLiftWork {
    Pending,
    EmptyPrincipal(Box<CoverLocalizationProduction>),
    EmptySupport(PhysicalSupportRestriction),
    DeferredBelowOriginal(Arc<LocalizedCoverHistory>),
    /// The exact lower index remains in `lower_owner`; no invariant/density
    /// completion is inferred from this accounting entry.
    DeferredLower {
        localization: Arc<LocalizedCoverHistory>,
        index: usize,
    },
    Next(Box<CompositeLiftNext>),
    Center(Arc<CompositeContactCenter>),
}
#[derive(Clone, Debug)]
pub struct CompositeLiftNext {
    localization: Arc<LocalizedCoverHistory>,
    matched: Arc<MatchedPhysicalSupport>,
    children: Vec<Option<Box<CompositeCoverLift>>>,
    lower_children: Arc<[Arc<LowerCoverTree>]>,
}
impl CompositeLiftNext {
    pub fn localization(&self) -> &Arc<LocalizedCoverHistory> {
        &self.localization
    }
    pub fn matched(&self) -> &Arc<MatchedPhysicalSupport> {
        &self.matched
    }
    pub fn children(&self) -> &[Option<Box<CompositeCoverLift>>] {
        &self.children
    }
}
#[derive(Clone, Debug)]
pub struct CompositeCoverLift {
    context: Arc<CompositeContext>,
    lower: Arc<LowerCoverTree>,
    history: Arc<ResolutionHistory>,
    pulled: Arc<PulledSupportCover>,
    cover: Arc<PhysicalSupportCover>,
    work: Vec<CompositeLiftWork>,
    namespace: String,
}
impl CompositeCoverLift {
    pub(super) fn reserve_snapshot(&self, b: &mut Budget) -> Result<()> {
        b.reserve_slots(self.work.len())?;
        for work in &self.work {
            b.charge(1)?;
            if let CompositeLiftWork::Next(next) = work {
                b.reserve_slots(next.children.len())?;
                for child in next.children.iter().flatten() {
                    child.reserve_snapshot(b)?;
                }
            }
        }
        Ok(())
    }
    pub(super) fn first(
        branch: Arc<ContactDispatchBranch>,
        child_source: &Arc<MarkedIdeal>,
        source_open: Arc<LocalizedCoverHistory>,
        embedding: Arc<SupportEmbedding>,
        selected: (&FiniteContactDispatcher, usize),
        namespace: String,
        b: &mut Budget,
    ) -> Result<Self> {
        let (owner, index) = selected;
        let (parent, actual_open, _) = embedding
            .source_open()
            .ok_or(Error::Invalid("composite original source restriction"))?;
        if !Arc::ptr_eq(parent, branch.embedding())
            || !Arc::ptr_eq(actual_open, &source_open)
            || !Arc::ptr_eq(owner.cover().origin().auxiliary().source(), child_source)
            || !Arc::ptr_eq(
                owner.cover().origin().auxiliary().frame(),
                branch.contact().contact(),
            )
        {
            return Err(Error::Invalid("composite genuine child source/history"));
        }
        let lower = LowerCoverTree::from_branch(owner, index, b)?;
        let original = branch.owner().origin().auxiliary().source();
        let source = Arc::new(MarkedIdeal::new(
            source_open.open().extension().ideal(original.ideal(), b)?,
            original.mark(),
            b,
        )?);
        let context = Arc::new(CompositeContext {
            branch,
            source_open: source_open.clone(),
            lower_owner: owner.cover().clone(),
        });
        Self::prepare(
            context,
            lower,
            embedding,
            (source, source_open.history().clone()),
            false,
            namespace,
            b,
        )
    }
    fn prepare(
        context: Arc<CompositeContext>,
        lower: Arc<LowerCoverTree>,
        embedding: Arc<SupportEmbedding>,
        original: (Arc<MarkedIdeal>, Arc<ResolutionHistory>),
        matched: bool,
        namespace: String,
        b: &mut Budget,
    ) -> Result<Self> {
        let (source, history) = original;
        if !Arc::ptr_eq(history.ledger().frame(), embedding.ambient()) {
            return Err(Error::Invalid("composite next ambient history/frame"));
        }
        let pulled = if matched {
            PulledSupportCover::prepare_matched(embedding.clone(), lower.cover.clone(), b)?
        } else {
            PulledSupportCover::prepare(embedding.clone(), lower.cover.clone(), b)?
        };
        let cover = embedding.physical_cover(source, pulled.pulled().clone(), b)?;
        b.reserve_slots(cover.opens().len())?;
        Ok(Self {
            context,
            lower,
            history,
            pulled,
            work: vec![CompositeLiftWork::Pending; cover.opens().len()],
            cover,
            namespace,
        })
    }
    pub fn lower_owner(&self) -> &Arc<ContactDispatchCover> {
        &self.lower.owner
    }
    pub fn lower_history(&self) -> &Arc<ResolutionHistory> {
        &self.lower.history
    }
    pub fn history(&self) -> &Arc<ResolutionHistory> {
        &self.history
    }
    pub fn cover(&self) -> &Arc<PhysicalSupportCover> {
        &self.cover
    }
    pub fn pulled_cover(&self) -> &Arc<PulledSupportCover> {
        &self.pulled
    }
    pub fn work(&self) -> &[CompositeLiftWork] {
        &self.work
    }
    pub fn complete_inventory(&self) -> bool {
        self.work.iter().all(|w| match w {
            CompositeLiftWork::Pending => false,
            CompositeLiftWork::Next(n) => n
                .children
                .iter()
                .all(|c| c.as_ref().is_some_and(|c| c.complete_inventory())),
            _ => true,
        })
    }
    pub(super) fn advance(
        &mut self,
        index: usize,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        if !matches!(
            self.work
                .get(index)
                .ok_or(Error::Invalid("composite physical open index"))?,
            CompositeLiftWork::Pending
        ) {
            return Ok(ContactDispatchAdvance::Accepted);
        }
        match self.one(index, b) {
            Ok(w) => {
                self.work[index] = w;
                Ok(ContactDispatchAdvance::Progress)
            }
            Err(Error::ResourceIncomplete(reason)) => {
                Ok(ContactDispatchAdvance::Incomplete { reason })
            }
            Err(e) => Err(e),
        }
    }
    fn one(&self, index: usize, b: &mut Budget) -> Result<CompositeLiftWork> {
        let opening = match self.cover.localize(
            self.history.clone(),
            index,
            &format!("{}_open{index}", self.namespace),
            b,
        )? {
            CoverLocalizationProduction::Complete(o) => Arc::new(*o),
            r @ CoverLocalizationProduction::Empty { .. } => {
                return Ok(CompositeLiftWork::EmptyPrincipal(Box::new(r)));
            }
            CoverLocalizationProduction::Incomplete { reason, .. } => {
                return Err(Error::ResourceIncomplete(reason));
            }
        };
        let i = match self
            .cover
            .opens()
            .get(index)
            .ok_or(Error::Invalid("composite full physical index"))?
        {
            PhysicalSupportOpen::BelowMark { .. } => {
                return Ok(CompositeLiftWork::DeferredBelowOriginal(opening));
            }
            PhysicalSupportOpen::Lower { index, .. } => *index,
        };
        let entry = self
            .lower
            .entries
            .get(i)
            .ok_or(Error::Invalid("composite actual lower index"))?;
        let lower_open = match entry {
            LowerTreeEntry::Deferred => {
                return Ok(CompositeLiftWork::DeferredLower {
                    localization: opening,
                    index: i,
                });
            }
            LowerTreeEntry::Next { opening, .. } => opening,
            LowerTreeEntry::Center(c) => c.localization(),
        };
        if !Arc::ptr_eq(lower_open.open().source(), &self.lower.history) {
            return Err(Error::Invalid("composite lower exact history"));
        }
        let matched = match self.cover.restrict_pulled_lower(
            opening.clone(),
            lower_open.clone(),
            self.pulled.clone(),
            &format!("{}_match{index}", self.namespace),
            b,
        )? {
            MatchedPhysicalRestriction::Support(s) => s,
            MatchedPhysicalRestriction::EmptySupport(r) => {
                return Ok(CompositeLiftWork::EmptySupport(r));
            }
        };
        Ok(match entry {
            LowerTreeEntry::Center(c) => {
                CompositeLiftWork::Center(CompositeContactCenter::prepare(
                    self.context.clone(),
                    c.clone(),
                    self.cover.clone(),
                    opening,
                    matched,
                    b,
                )?)
            }
            LowerTreeEntry::Next { children, .. } => {
                b.reserve_slots(children.len())?;
                CompositeLiftWork::Next(Box::new(CompositeLiftNext {
                    localization: opening,
                    matched,
                    children: (0..children.len()).map(|_| None).collect(),
                    lower_children: children.clone(),
                }))
            }
            LowerTreeEntry::Deferred => unreachable!(),
        })
    }
    pub(super) fn prepare_child(
        &mut self,
        index: usize,
        child: usize,
        b: &mut Budget,
    ) -> Result<ContactDispatchAdvance> {
        let Some(CompositeLiftWork::Next(next)) = self.work.get(index) else {
            return Err(Error::Invalid("composite prepared next-cover opening"));
        };
        let slot = next
            .children
            .get(child)
            .ok_or(Error::Invalid("composite next cover index"))?;
        if slot.is_some() {
            return Ok(ContactDispatchAdvance::Accepted);
        }
        let result = (|| {
            if !Arc::ptr_eq(
                &next.lower_children[child].history,
                next.matched.lower().history(),
            ) {
                return Err(Error::Invalid("composite next original history owner"));
            }
            let embedding = SupportEmbedding::refine_matched(next.matched.clone(), b)?;
            let source = self.cover.source();
            let current = Arc::new(MarkedIdeal::new(
                next.localization
                    .open()
                    .extension()
                    .ideal(source.ideal(), b)?,
                source.mark(),
                b,
            )?);
            Self::prepare(
                self.context.clone(),
                next.lower_children[child].clone(),
                embedding,
                (current, next.localization.history().clone()),
                true,
                format!("{}_open{index}_child{child}", self.namespace),
                b,
            )
        })();
        match result {
            Ok(c) => {
                let CompositeLiftWork::Next(next) = &mut self.work[index] else {
                    unreachable!()
                };
                next.children[child] = Some(Box::new(c));
                Ok(ContactDispatchAdvance::Progress)
            }
            Err(Error::ResourceIncomplete(reason)) => {
                Ok(ContactDispatchAdvance::Incomplete { reason })
            }
            Err(e) => Err(e),
        }
    }
    pub(super) fn node_mut(&mut self, path: &[(usize, usize)]) -> Result<&mut Self> {
        let Some((&(opening, child), rest)) = path.split_first() else {
            return Ok(self);
        };
        let Some(CompositeLiftWork::Next(next)) = self.work.get_mut(opening) else {
            return Err(Error::Invalid("composite path opening"));
        };
        next.children
            .get_mut(child)
            .and_then(Option::as_deref_mut)
            .ok_or(Error::Invalid("composite path pending child"))?
            .node_mut(rest)
    }
}
