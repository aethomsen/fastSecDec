use super::*;

impl OriginalPhysicalState {
    /// Issued BEFORE the next physical pivot, retaining the actual new C/J root.
    /// This bootstrap adapter consumes the original two-map drop; subsequent
    /// cycles use `begin_cycle` with the current source-bound completion.
    pub fn begin_ambient_cycle(
        self: &Arc<Self>,
        owner: Arc<AmbientContinuationCenter>,
        b: &mut Budget,
    ) -> Result<Arc<PhysicalCycleStart>> {
        let ActivePhysicalCycle::Bootstrap(carry) = &self.active else {
            return Err(Error::ResourceIncomplete(
                "generic positive residual restart requires its own completion owner",
            ));
        };
        if !Arc::ptr_eq(owner.residual_drop().inventory().carry(), carry)
            || !Arc::ptr_eq(owner.source_open().open().source(), self.chart.history())
        {
            return Err(Error::Invalid("physical restart actual completed cycle"));
        }
        let origin = EmbeddedChildCycle::new(owner.center().clone(), b)?;
        PhysicalCycleStart::new(
            self.clone(),
            PhysicalRestartOwner::Ambient(owner),
            origin,
            b,
        )
    }
    pub fn begin_cycle(
        self: &Arc<Self>,
        owner: Arc<PhysicalContinuationCenter>,
        b: &mut Budget,
    ) -> Result<Arc<PhysicalCycleStart>> {
        if !Arc::ptr_eq(owner.residual_drop().inventory().state(), self)
            || !Arc::ptr_eq(owner.source_open().open().source(), self.chart.history())
        {
            return Err(Error::Invalid("physical restart current completed cycle"));
        }
        let origin = EmbeddedChildCycle::new(owner.center().clone(), b)?;
        PhysicalCycleStart::new(
            self.clone(),
            PhysicalRestartOwner::Recursive(owner),
            origin,
            b,
        )
    }
}
#[derive(Clone, Debug)]
pub enum PhysicalCycleCompletion {
    Ambient(Arc<AmbientResidualDrop>),
    Recursive(Arc<PhysicalResidualDrop>),
}
#[derive(Clone, Debug)]
pub enum PhysicalRestartOwner {
    Ambient(Arc<AmbientContinuationCenter>),
    Recursive(Arc<PhysicalContinuationCenter>),
}
impl PhysicalRestartOwner {
    pub fn center(&self) -> &Arc<CompanionCenter> {
        match self {
            Self::Ambient(o) => o.center(),
            Self::Recursive(o) => o.center(),
        }
    }
    pub fn source_open(&self) -> &Arc<LocalizedCoverHistory> {
        match self {
            Self::Ambient(o) => o.source_open(),
            Self::Recursive(o) => o.source_open(),
        }
    }
    fn completion(&self) -> PhysicalCycleCompletion {
        match self {
            Self::Ambient(o) => PhysicalCycleCompletion::Ambient(o.residual_drop().clone()),
            Self::Recursive(o) => PhysicalCycleCompletion::Recursive(o.residual_drop().clone()),
        }
    }
}
#[derive(Clone, Debug)]
pub struct PhysicalCycleStart {
    source: Arc<OriginalPhysicalState>,
    owner: PhysicalRestartOwner,
    origin: Arc<EmbeddedChildCycle>,
    retirement: Arc<RetiredPhysicalCycle>,
}
impl PhysicalCycleStart {
    fn new(
        source: Arc<OriginalPhysicalState>,
        owner: PhysicalRestartOwner,
        origin: Arc<EmbeddedChildCycle>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        b.reserve_slots(1)?;
        let count = source
            .retired_count()
            .checked_add(1)
            .ok_or(Error::ResourceIncomplete("retired cycle count"))?;
        let retirement = Arc::new(RetiredPhysicalCycle {
            source: source.clone(),
            completion: owner.completion(),
            previous: source.retired.clone(),
            count,
        });
        Ok(Arc::new(Self {
            source,
            owner,
            origin,
            retirement,
        }))
    }
    pub fn source(&self) -> &Arc<OriginalPhysicalState> {
        &self.source
    }
    pub fn owner(&self) -> &PhysicalRestartOwner {
        &self.owner
    }
    pub fn origin(&self) -> &Arc<EmbeddedChildCycle> {
        &self.origin
    }
    pub fn advance(
        self: &Arc<Self>,
        chart: Arc<RelativeRecursiveChart>,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<Arc<OriginalPhysicalState>> {
        if !matches!(chart.geometry().center().origin(),RecursiveCenterOrigin::Companion(c)if Arc::ptr_eq(c,self.owner.center()))
            || !Arc::ptr_eq(chart.parent().source(), self.owner.center().parent_source())
            || !Arc::ptr_eq(
                chart.geometry().center().history(),
                self.owner.source_open().history(),
            )
        {
            return Err(Error::Invalid("physical fresh-cycle actual center"));
        }
        let exceptional = self
            .source
            .exceptional
            .restrict(self.owner.source_open(), b)?
            .advance(&chart, b)?;
        let (generators, jacobian) = self.source.transport(
            &chart,
            Some(self.owner.source_open()),
            &exceptional,
            namespace,
            b,
        )?;
        let depth = self
            .source
            .depth
            .checked_add(1)
            .ok_or(Error::ResourceIncomplete("physical lineage depth"))?;
        Ok(Arc::new(OriginalPhysicalState {
            root: self.source.root.clone(),
            edge: PhysicalLineageEdge::FreshCycle {
                start: self.clone(),
                chart: chart.clone(),
            },
            chart: chart.clone(),
            generators,
            exceptional,
            jacobian,
            active: ActivePhysicalCycle::First {
                origin: self.origin.clone(),
                chart: chart.clone(),
            },
            retired: Some(self.retirement.clone()),
            depth,
        }))
    }
}
