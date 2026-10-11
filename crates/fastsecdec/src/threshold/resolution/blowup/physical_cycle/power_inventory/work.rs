use super::*;
#[derive(Clone, Debug)]
pub enum PhysicalAuxiliaryWork {
    Pending,
    Preparing(Arc<PhysicalPowerPreparation>),
    PhysicalTerminal(Arc<PhysicalPowerTerminal>),
    Ready(Arc<PhysicalPowerInventory>),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PhysicalAuxiliaryAdvance {
    PhysicalPending,
    Progress,
    Accounted,
    AlreadyAccounted,
    Incomplete { reason: &'static str },
}
#[derive(Clone, Debug)]
enum Request {
    First { level: usize, open: Option<usize> },
    Continued(usize),
}
/// Accepted per-slot preparation; native inner refusals retain this predecessor.
#[derive(Clone, Debug)]
pub struct PhysicalPowerPreparation {
    state: Arc<OriginalPhysicalState>,
    origin: InventoryOrigin,
    requests: Arc<Vec<Request>>,
    slots: Vec<Arc<PhysicalPowerSlot>>,
    groups: Vec<SlotGroup>,
    retired: Option<Arc<RetiredPowerInventory>>,
}
impl PhysicalPowerPreparation {
    pub fn state(&self) -> &Arc<OriginalPhysicalState> {
        &self.state
    }
    pub fn accepted_source_slots(&self) -> usize {
        self.groups.len()
    }
    pub fn source_slots(&self) -> usize {
        self.requests.len()
    }
    pub fn target_slots(&self) -> &[Arc<PhysicalPowerSlot>] {
        &self.slots
    }
    fn prepare(
        source: &AuxiliaryOrigin,
        state: Arc<OriginalPhysicalState>,
        name: &str,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        source.check_state(&state)?;
        let (origin, requests) = if let Some(original) = source.first_tree() {
            let tree = original.pull_first(state.chart().clone(), name, b)?;
            let _ = tree.histories(b)?;
            let n = tree.levels().iter().try_fold(0usize, |n, l| {
                n.checked_add(l.opens().len().max(1))
                    .ok_or(Error::ResourceIncomplete("power source slot count"))
            })?;
            b.reserve_slots(n)?;
            let mut requests = Vec::with_capacity(n);
            for (level, l) in tree.levels().iter().enumerate() {
                if l.opens().is_empty() {
                    if !l.support().saturation().empty() {
                        return Err(Error::Invalid("empty power level lacks support proof"));
                    }
                    requests.push(Request::First { level, open: None });
                } else {
                    for i in 0..l.opens().len() {
                        requests.push(Request::First {
                            level,
                            open: Some(i),
                        });
                    }
                }
            }
            (InventoryOrigin::First(tree), requests)
        } else {
            let AuxiliaryOrigin::Continued(prior) = source else {
                return Err(Error::Invalid("power source preparation origin"));
            };
            prior.check_shape()?;
            b.reserve_slots(prior.slots.len())?;
            (
                InventoryOrigin::Continued(prior.clone()),
                (0..prior.slots.len()).map(Request::Continued).collect(),
            )
        };
        Ok(Arc::new(Self {
            state,
            origin,
            requests: Arc::new(requests),
            slots: vec![],
            groups: vec![],
            retired: source.retirement().cloned(),
        }))
    }
    fn step(&self, name: &str, b: &mut Budget) -> Result<Self> {
        let index = self.groups.len();
        let request = self
            .requests
            .get(index)
            .ok_or(Error::Invalid("completed power preparation reissued"))?;
        let values = match (&self.origin, request) {
            (InventoryOrigin::First(tree), Request::First { level, open }) => {
                b.reserve_slots(1)?;
                vec![PhysicalPowerSlot::first(tree.clone(), *level, *open, b)?]
            }
            (InventoryOrigin::Continued(prior), Request::Continued(i)) => prior
                .slots
                .get(*i)
                .ok_or(Error::Invalid("power predecessor slot"))?
                .advance(
                    self.state.chart().clone(),
                    &format!("{name}_slot{index}"),
                    b,
                )?,
            _ => return Err(Error::Invalid("power preparation request origin")),
        };
        let end = self
            .slots
            .len()
            .checked_add(values.len())
            .ok_or(Error::ResourceIncomplete("power target slot count"))?;
        b.reserve_slots(end)?;
        b.reserve_slots(
            self.groups
                .len()
                .checked_add(1)
                .ok_or(Error::ResourceIncomplete("power group count"))?,
        )?;
        let mut next = self.clone();
        next.groups.push(SlotGroup {
            begin: self.slots.len(),
            end,
        });
        next.slots.extend(values);
        Ok(next)
    }
    fn finish(self) -> Result<Arc<PhysicalPowerInventory>> {
        if self.groups.len() != self.requests.len() {
            return Err(Error::Invalid("incomplete power preparation admission"));
        }
        let out = Arc::new(PhysicalPowerInventory {
            state: self.state,
            origin: self.origin,
            slots: self.slots,
            groups: self.groups,
            retired: self.retired,
        });
        out.check_shape()?;
        Ok(out)
    }
}
#[derive(Clone, Debug)]
pub(in super::super) struct AuxiliaryFrontier {
    origin: AuxiliaryOrigin,
    blowup: Arc<RelativeRecursiveBlowup>,
    namespace: String,
    work: Vec<PhysicalAuxiliaryWork>,
}
impl AuxiliaryFrontier {
    pub(in super::super) fn initial(
        problem: Arc<OriginalPhysicalProblem>,
        blowup: Arc<RelativeRecursiveBlowup>,
        namespace: String,
        b: &mut Budget,
    ) -> Result<Self> {
        Self::new(AuxiliaryOrigin::First(problem), blowup, namespace, b)
    }
    pub(in super::super) fn continued(
        prior: Arc<PhysicalPowerInventory>,
        blowup: Arc<RelativeRecursiveBlowup>,
        namespace: String,
        b: &mut Budget,
    ) -> Result<Self> {
        prior.check_shape()?;
        Self::new(AuxiliaryOrigin::Continued(prior), blowup, namespace, b)
    }
    pub(in super::super) fn fresh(
        start: Arc<PhysicalCycleStart>,
        prior: Arc<PhysicalPowerInventory>,
        blowup: Arc<RelativeRecursiveBlowup>,
        namespace: String,
        b: &mut Budget,
    ) -> Result<Self> {
        prior.check_shape()?;
        if !Arc::ptr_eq(start.source(), prior.state()) {
            return Err(Error::Invalid("power retirement exact source state"));
        }
        let Some(a) = start.source().active_original_cycle() else {
            return Err(Error::ResourceIncomplete(
                "power retirement bootstrap cycle",
            ));
        };
        let Some(z) = prior.state().active_original_cycle() else {
            return Err(Error::Invalid("power retirement absent prior cycle"));
        };
        if !Arc::ptr_eq(a, z) {
            return Err(Error::Invalid("power retirement exact active cycle"));
        }
        b.reserve_slots(1)?;
        let retired = Arc::new(RetiredPowerInventory {
            source: prior,
            start: start.clone(),
        });
        Self::new(
            AuxiliaryOrigin::Fresh { start, retired },
            blowup,
            namespace,
            b,
        )
    }
    fn new(
        origin: AuxiliaryOrigin,
        blowup: Arc<RelativeRecursiveBlowup>,
        namespace: String,
        b: &mut Budget,
    ) -> Result<Self> {
        b.reserve_slots(blowup.charts().len())?;
        Ok(Self {
            origin,
            work: vec![PhysicalAuxiliaryWork::Pending; blowup.charts().len()],
            blowup,
            namespace,
        })
    }
    pub(in super::super) fn work(&self) -> &[PhysicalAuxiliaryWork] {
        &self.work
    }
    pub(in super::super) fn complete(&self) -> bool {
        self.work.iter().all(|w| {
            matches!(
                w,
                PhysicalAuxiliaryWork::PhysicalTerminal(_) | PhysicalAuxiliaryWork::Ready(_)
            )
        })
    }
    pub(in super::super) fn advance(
        &mut self,
        index: usize,
        physical: &PhysicalChartWork,
        b: &mut Budget,
    ) -> Result<PhysicalAuxiliaryAdvance> {
        let old = self
            .work
            .get(index)
            .ok_or(Error::Invalid("auxiliary chart index"))?;
        if matches!(
            old,
            PhysicalAuxiliaryWork::PhysicalTerminal(_) | PhysicalAuxiliaryWork::Ready(_)
        ) {
            return Ok(PhysicalAuxiliaryAdvance::AlreadyAccounted);
        }
        let PhysicalChartWork::Complete(terminal) = physical else {
            return Ok(PhysicalAuxiliaryAdvance::PhysicalPending);
        };
        if !Arc::ptr_eq(terminal.state().chart(), &self.blowup.charts()[index]) {
            return Err(Error::Invalid("auxiliary physical chart association"));
        }
        self.origin.check_state(terminal.state())?;
        match self.step(index, old, terminal, b) {
            Ok((next, status)) => {
                self.work[index] = next;
                Ok(status)
            }
            Err(Error::ResourceIncomplete(reason)) => {
                Ok(PhysicalAuxiliaryAdvance::Incomplete { reason })
            }
            Err(e) => Err(e),
        }
    }
    fn step(
        &self,
        index: usize,
        old: &PhysicalAuxiliaryWork,
        terminal: &Arc<PhysicalTerminalInventory>,
        b: &mut Budget,
    ) -> Result<(PhysicalAuxiliaryWork, PhysicalAuxiliaryAdvance)> {
        b.count(1)?;
        let name = format!("{}_chart{index}", self.namespace);
        if terminal.all_local_factors_snc() {
            return Ok((
                PhysicalAuxiliaryWork::PhysicalTerminal(Arc::new(PhysicalPowerTerminal {
                    terminal: terminal.clone(),
                    source: self.origin.clone(),
                })),
                PhysicalAuxiliaryAdvance::Accounted,
            ));
        }
        match old {
            PhysicalAuxiliaryWork::Pending => {
                let prepared = PhysicalPowerPreparation::prepare(
                    &self.origin,
                    terminal.state().clone(),
                    &name,
                    b,
                )?;
                // Native original trees contain at least their terminal level;
                // each level/group retains >=1 slot, including empty support.
                // Thus issued preparations always have a next source request.
                Ok((
                    PhysicalAuxiliaryWork::Preparing(prepared),
                    PhysicalAuxiliaryAdvance::Progress,
                ))
            }
            PhysicalAuxiliaryWork::Preparing(p) => {
                if !Arc::ptr_eq(p.state(), terminal.state()) {
                    return Err(Error::Invalid("auxiliary preparation physical owner"));
                }
                let next = p.step(&name, b)?;
                if next.groups.len() == next.requests.len() {
                    Ok((
                        PhysicalAuxiliaryWork::Ready(next.finish()?),
                        PhysicalAuxiliaryAdvance::Accounted,
                    ))
                } else {
                    Ok((
                        PhysicalAuxiliaryWork::Preparing(Arc::new(next)),
                        PhysicalAuxiliaryAdvance::Progress,
                    ))
                }
            }
            _ => Err(Error::Invalid("complete auxiliary work reissued")),
        }
    }
}
