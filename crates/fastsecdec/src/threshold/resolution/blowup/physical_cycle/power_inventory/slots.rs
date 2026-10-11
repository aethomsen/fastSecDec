use super::*;
#[derive(Clone, Debug)]
pub enum PhysicalPowerSlotKind {
    Active(Arc<CarriedPowerState>),
    SourceCompleted(Arc<CarriedPowerState>),
    /// Original no-power construction; retained ancestry, not target geometry.
    NoPower(Arc<OriginalLevelOpen>),
    /// Actual first strict support is empty, with the original level retained.
    EmptyFirstSupport(Arc<OriginalLevelPullback>),
    /// Actual continued strict support is empty, with its chart receipt retained.
    EmptyContinuedSupport(Arc<CarriedPowerTransition>),
}
#[derive(Clone, Debug)]
enum SlotOrigin {
    First {
        tree: Arc<OriginalTreePullback>,
        level: usize,
        open: Option<usize>,
    },
    Inherited(Arc<PhysicalPowerSlot>),
    Transformed {
        prior: Arc<PhysicalPowerSlot>,
        transition: Arc<CarriedPowerTransition>,
        open: Option<usize>,
    },
    Completed(Arc<PhysicalPowerSlot>),
}
#[derive(Clone, Debug)]
pub struct PhysicalPowerSlot {
    origin: SlotOrigin,
    kind: PhysicalPowerSlotKind,
}
impl PhysicalPowerSlot {
    pub fn kind(&self) -> &PhysicalPowerSlotKind {
        &self.kind
    }
    pub fn first_slot(&self) -> Option<(usize, Option<usize>)> {
        match &self.origin {
            SlotOrigin::First { level, open, .. } => Some((*level, *open)),
            _ => None,
        }
    }
    pub fn prior(&self) -> Option<&Arc<Self>> {
        match &self.origin {
            SlotOrigin::Inherited(p) | SlotOrigin::Completed(p) => Some(p),
            SlotOrigin::Transformed { prior, .. } => Some(prior),
            _ => None,
        }
    }
    pub(super) fn first(
        tree: Arc<OriginalTreePullback>,
        level: usize,
        open: Option<usize>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let l = tree
            .levels()
            .get(level)
            .ok_or(Error::Invalid("power first level slot"))?;
        let kind = if let Some(i) = open {
            let o = l
                .opens()
                .get(i)
                .ok_or(Error::Invalid("power first open slot"))?;
            if o.power().is_some() {
                let state = CarriedPowerState::from_first(tree.clone(), level, i, b)?;
                if state.completed() {
                    PhysicalPowerSlotKind::SourceCompleted(state)
                } else {
                    PhysicalPowerSlotKind::Active(state)
                }
            } else {
                PhysicalPowerSlotKind::NoPower(o.clone())
            }
        } else {
            if !l.opens().is_empty() || !l.support().saturation().empty() {
                return Err(Error::Invalid("power first empty support authority"));
            }
            PhysicalPowerSlotKind::EmptyFirstSupport(l.clone())
        };
        Ok(Arc::new(Self {
            origin: SlotOrigin::First { tree, level, open },
            kind,
        }))
    }
    pub(super) fn advance(
        self: &Arc<Self>,
        chart: Arc<RelativeRecursiveChart>,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<Vec<Arc<Self>>> {
        b.reserve_slots(1)?;
        let PhysicalPowerSlotKind::Active(source) = &self.kind else {
            return Ok(vec![Arc::new(Self {
                origin: SlotOrigin::Inherited(self.clone()),
                kind: self.kind.clone(),
            })]);
        };
        match source.advance(chart, namespace, b)? {
            CarriedPowerAdvance::Incomplete(p) => Err(Error::ResourceIncomplete(p.reason())),
            CarriedPowerAdvance::SourceCompleted(s) => Ok(vec![Arc::new(Self {
                origin: SlotOrigin::Completed(self.clone()),
                kind: PhysicalPowerSlotKind::SourceCompleted(s),
            })]),
            CarriedPowerAdvance::Transformed(t) => {
                if !t.complete_inventory() {
                    return Err(Error::Invalid("power slot incomplete support inventory"));
                }
                if t.states().is_empty() {
                    if !t.support().saturation().empty() {
                        return Err(Error::Invalid("power slot empty support missing proof"));
                    }
                    Ok(vec![Arc::new(Self {
                        origin: SlotOrigin::Transformed {
                            prior: self.clone(),
                            transition: t.clone(),
                            open: None,
                        },
                        kind: PhysicalPowerSlotKind::EmptyContinuedSupport(t),
                    })])
                } else {
                    b.reserve_slots(t.states().len())?;
                    Ok(t.states()
                        .iter()
                        .enumerate()
                        .map(|(i, s)| {
                            Arc::new(Self {
                                origin: SlotOrigin::Transformed {
                                    prior: self.clone(),
                                    transition: t.clone(),
                                    open: Some(i),
                                },
                                kind: if s.completed() {
                                    PhysicalPowerSlotKind::SourceCompleted(s.clone())
                                } else {
                                    PhysicalPowerSlotKind::Active(s.clone())
                                },
                            })
                        })
                        .collect())
                }
            }
        }
    }
    pub(super) fn matches_first(
        &self,
        owner: &Arc<OriginalTreePullback>,
        expected_level: usize,
        expected_open: Option<usize>,
    ) -> bool {
        matches!(&self.origin,SlotOrigin::First{tree,level,open}if Arc::ptr_eq(tree,owner)&&*level==expected_level&&*open==expected_open)
    }
    pub(super) fn check_group(
        slots: &[Arc<Self>],
        expected: &Arc<Self>,
        state: &Arc<OriginalPhysicalState>,
    ) -> Result<()> {
        let first = slots
            .first()
            .ok_or(Error::Invalid("power empty accounting group"))?;
        match &first.origin {
            SlotOrigin::Inherited(p) | SlotOrigin::Completed(p) => {
                if slots.len() != 1 || !Arc::ptr_eq(p, expected) {
                    return Err(Error::Invalid("power inherited group association"));
                }
            }
            SlotOrigin::Transformed {
                prior, transition, ..
            } => {
                if !Arc::ptr_eq(prior, expected)
                    || !Arc::ptr_eq(transition.chart(), state.chart())
                    || slots.len() != transition.states().len().max(1)
                {
                    return Err(Error::Invalid("power transformed group coverage"));
                }
                for (i, slot) in slots.iter().enumerate() {
                    let expected_open = (!transition.states().is_empty()).then_some(i);
                    if !matches!(&slot.origin,SlotOrigin::Transformed{prior,transition:t,open}if Arc::ptr_eq(prior,expected)&&Arc::ptr_eq(t,transition)&&*open==expected_open)
                    {
                        return Err(Error::Invalid("power transformed open inventory"));
                    }
                }
            }
            _ => return Err(Error::Invalid("power continued group origin")),
        }
        Ok(())
    }
    pub(super) fn check(&self, state: &Arc<OriginalPhysicalState>) -> Result<()> {
        let good = match (&self.origin, &self.kind) {
            (SlotOrigin::First { tree, level, open }, kind) => {
                if !Arc::ptr_eq(tree.chart(), state.chart()) {
                    return Err(Error::Invalid("power slot first physical chart"));
                }
                let l = tree
                    .levels()
                    .get(*level)
                    .ok_or(Error::Invalid("power slot level"))?;
                match (open, kind) {
                    (
                        Some(i),
                        PhysicalPowerSlotKind::Active(s)
                        | PhysicalPowerSlotKind::SourceCompleted(s),
                    ) => {
                        matches!(s.first_slot(),Some((t,j,k))if Arc::ptr_eq(t,tree)&&j==*level&&k==*i)
                    }
                    (Some(i), PhysicalPowerSlotKind::NoPower(o)) => l
                        .opens()
                        .get(*i)
                        .is_some_and(|x| Arc::ptr_eq(x, o) && x.power().is_none()),
                    (None, PhysicalPowerSlotKind::EmptyFirstSupport(x)) => {
                        Arc::ptr_eq(l, x)
                            && l.opens().is_empty()
                            && l.support().saturation().empty()
                    }
                    _ => false,
                }
            }
            (SlotOrigin::Inherited(p), kind) => match (&p.kind, kind) {
                (
                    PhysicalPowerSlotKind::SourceCompleted(a),
                    PhysicalPowerSlotKind::SourceCompleted(b),
                ) => Arc::ptr_eq(a, b) && a.completed(),
                (PhysicalPowerSlotKind::NoPower(a), PhysicalPowerSlotKind::NoPower(b)) => {
                    Arc::ptr_eq(a, b)
                }
                (
                    PhysicalPowerSlotKind::EmptyFirstSupport(a),
                    PhysicalPowerSlotKind::EmptyFirstSupport(b),
                ) => Arc::ptr_eq(a, b),
                (
                    PhysicalPowerSlotKind::EmptyContinuedSupport(a),
                    PhysicalPowerSlotKind::EmptyContinuedSupport(b),
                ) => Arc::ptr_eq(a, b),
                _ => false,
            },
            (SlotOrigin::Completed(p), PhysicalPowerSlotKind::SourceCompleted(s)) => {
                matches!(&p.kind,PhysicalPowerSlotKind::Active(a)if Arc::ptr_eq(a,s)&&s.completed())
            }
            (
                SlotOrigin::Transformed {
                    prior,
                    transition: t,
                    open,
                },
                kind,
            ) => {
                if !Arc::ptr_eq(t.chart(), state.chart())
                    || !matches!(&prior.kind,PhysicalPowerSlotKind::Active(p)if Arc::ptr_eq(p,t.source()))
                {
                    return Err(Error::Invalid("power slot actual transition source/chart"));
                }
                match (open, kind) {
                    (
                        Some(i),
                        PhysicalPowerSlotKind::Active(s)
                        | PhysicalPowerSlotKind::SourceCompleted(s),
                    ) => t.states().get(*i).is_some_and(|x| Arc::ptr_eq(x, s)),
                    (None, PhysicalPowerSlotKind::EmptyContinuedSupport(x)) => {
                        Arc::ptr_eq(t, x)
                            && t.states().is_empty()
                            && t.support().saturation().empty()
                    }
                    _ => false,
                }
            }
            _ => false,
        };
        if !good {
            return Err(Error::Invalid("power slot owner/kind association"));
        }
        if let PhysicalPowerSlotKind::Active(s) = &self.kind
            && s.completed()
        {
            return Err(Error::Invalid("completed power slot relabelled active"));
        }
        if let PhysicalPowerSlotKind::SourceCompleted(s) = &self.kind
            && !s.completed()
        {
            return Err(Error::Invalid("active power slot relabelled complete"));
        }
        Ok(())
    }
}
