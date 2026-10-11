//! Caller-stepped one-stage physical inventory, terminal checks before C/J work.
//! Progress is in memory. Native resource refusals do not discard accepted charts.
use super::super::super::*;
use super::super::helpers::Result;
use super::power_inventory::AuxiliaryFrontier;
use super::*;
use std::sync::Arc;
#[derive(Clone, Debug)]
enum FrontierOrigin {
    Initial(Arc<OriginalPhysicalProblem>),
    Fresh(Arc<PhysicalCycleStart>),
    Embedded {
        state: Arc<OriginalPhysicalState>,
        source: Arc<EmbeddedPresentation>,
    },
}
#[derive(Clone, Debug)]
pub enum PhysicalChartWork {
    Pending,
    Factoring {
        state: Arc<OriginalPhysicalState>,
        frontier: ComponentFactorFrontier,
    },
    Complete(Arc<PhysicalTerminalInventory>),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PhysicalChartAdvance {
    Progress,
    Complete,
    AlreadyComplete,
    Incomplete { reason: &'static str },
}
#[derive(Clone, Debug)]
pub struct PhysicalChartFrontier {
    origin: FrontierOrigin,
    blowup: Arc<RelativeRecursiveBlowup>,
    namespace: String,
    work: Vec<PhysicalChartWork>,
    auxiliary: Option<AuxiliaryFrontier>,
}
impl PhysicalChartFrontier {
    pub fn from_problem(
        problem: Arc<OriginalPhysicalProblem>,
        blowup: Arc<RelativeRecursiveBlowup>,
        namespace: String,
        b: &mut Budget,
    ) -> Result<Self> {
        if !matches!(blowup.cover().center().origin(),RecursiveCenterOrigin::Companion(c)if Arc::ptr_eq(c,problem.center()))
        {
            return Err(Error::Invalid("physical frontier initial center owner"));
        }
        Self::new(FrontierOrigin::Initial(problem), blowup, namespace, b)
    }
    pub fn from_start(
        start: Arc<PhysicalCycleStart>,
        blowup: Arc<RelativeRecursiveBlowup>,
        namespace: String,
        b: &mut Budget,
    ) -> Result<Self> {
        if !matches!(blowup.cover().center().origin(),RecursiveCenterOrigin::Companion(c)if Arc::ptr_eq(c,start.owner().center()))
        {
            return Err(Error::Invalid("physical frontier fresh center owner"));
        }
        Self::new(FrontierOrigin::Fresh(start), blowup, namespace, b)
    }
    pub fn from_embedded(
        state: Arc<OriginalPhysicalState>,
        source: Arc<EmbeddedPresentation>,
        blowup: Arc<RelativeRecursiveBlowup>,
        namespace: String,
        b: &mut Budget,
    ) -> Result<Self> {
        if !state.owns_presentation(&source)
            || !Arc::ptr_eq(source.chart(), state.chart())
            || !Arc::ptr_eq(
                blowup.cover().center().source(),
                state.chart().parent().target(),
            )
            || !Arc::ptr_eq(blowup.cover().center().history(), state.chart().history())
        {
            return Err(Error::Invalid("physical frontier active source/history"));
        }
        // The existing EmbeddedTransition checker admits each exact center and
        // all its C/J restrictions before a map may become accepted work.
        Self::new(
            FrontierOrigin::Embedded { state, source },
            blowup,
            namespace,
            b,
        )
    }
    fn new(
        origin: FrontierOrigin,
        blowup: Arc<RelativeRecursiveBlowup>,
        namespace: String,
        b: &mut Budget,
    ) -> Result<Self> {
        if blowup
            .charts()
            .iter()
            .any(|c| !Arc::ptr_eq(c.geometry().center(), blowup.cover().center()))
        {
            return Err(Error::Invalid("physical frontier full chart inventory"));
        }
        b.reserve_slots(blowup.charts().len())?;
        Ok(Self {
            origin,
            auxiliary: None,
            work: vec![PhysicalChartWork::Pending; blowup.charts().len()],
            blowup,
            namespace,
        })
    }
    pub fn blowup(&self) -> &Arc<RelativeRecursiveBlowup> {
        &self.blowup
    }
    pub fn work(&self) -> &[PhysicalChartWork] {
        &self.work
    }
    pub fn complete_inventory(&self) -> bool {
        self.work
            .iter()
            .all(|w| matches!(w, PhysicalChartWork::Complete(_)))
    }
    /// This remains local algebraic factor geometry, NOT real integration cover.
    pub fn all_local_factors_snc(&self) -> bool {
        !self.work.is_empty()
            && self
                .work
                .iter()
                .all(|w| matches!(w,PhysicalChartWork::Complete(t)if t.all_local_factors_snc()))
    }
    pub fn advance(
        &mut self,
        index: usize,
        limits: &ComponentFactorLimits,
        b: &mut Budget,
    ) -> Result<PhysicalChartAdvance> {
        let old = self
            .work
            .get(index)
            .ok_or(Error::Invalid("physical frontier chart index"))?;
        if matches!(old, PhysicalChartWork::Complete(_)) {
            return Ok(PhysicalChartAdvance::AlreadyComplete);
        }
        let proposed = self.step(index, old.clone(), limits, b);
        match proposed {
            Ok((work, status)) => {
                self.work[index] = work;
                Ok(status)
            }
            Err(Error::ResourceIncomplete(reason)) => {
                Ok(PhysicalChartAdvance::Incomplete { reason })
            }
            Err(e) => Err(e),
        }
    }
    fn step(
        &self,
        index: usize,
        old: PhysicalChartWork,
        limits: &ComponentFactorLimits,
        b: &mut Budget,
    ) -> Result<(PhysicalChartWork, PhysicalChartAdvance)> {
        let chart = self.blowup.charts()[index].clone();
        let name = format!("{}_chart{index}", self.namespace);
        match old {
            PhysicalChartWork::Pending => {
                let state = match &self.origin {
                    FrontierOrigin::Initial(problem) => problem.advance(chart.clone(), b)?,
                    FrontierOrigin::Fresh(start) => start.advance(chart.clone(), &name, b)?,
                    FrontierOrigin::Embedded { state, source } => {
                        let transition = EmbeddedTransition::prepare(
                            source.clone(),
                            chart.clone(),
                            &format!("{name}_active"),
                            b,
                        )?;
                        state.continue_embedded(transition, &name, b)?
                    }
                };
                let frontier = ComponentFactorFrontier::new(
                    chart.history().clone(),
                    chart.parent().target().clone(),
                    format!("{name}_factors"),
                    b,
                )?;
                Ok((
                    PhysicalChartWork::Factoring { state, frontier },
                    PhysicalChartAdvance::Progress,
                ))
            }
            PhysicalChartWork::Factoring {
                state,
                mut frontier,
            } => {
                let next = frontier.pending().next().cloned();
                if let Some(path) = next {
                    let outcome = frontier.advance(&path, limits, b)?;
                    let status = match outcome {
                        FactorAdvance::Progress { .. } => PhysicalChartAdvance::Progress,
                        FactorAdvance::ResourceIncomplete(reason) => {
                            PhysicalChartAdvance::Incomplete { reason }
                        }
                    };
                    return Ok((PhysicalChartWork::Factoring { state, frontier }, status));
                }
                match frontier.try_complete()? {
                    ComponentFactorCompletion::Complete(factors) => {
                        let terminal = PhysicalTerminalInventory::prove(
                            state,
                            Arc::new(factors),
                            &format!("{name}_terminal"),
                            b,
                        )?;
                        Ok((
                            PhysicalChartWork::Complete(terminal),
                            PhysicalChartAdvance::Complete,
                        ))
                    }
                    ComponentFactorCompletion::Incomplete(frontier) => Ok((
                        PhysicalChartWork::Factoring { state, frontier },
                        PhysicalChartAdvance::Incomplete {
                            reason: "physical original factor inventory incomplete",
                        },
                    )),
                }
            }
            PhysicalChartWork::Complete(_) => {
                Err(Error::Invalid("complete physical chart work reissued"))
            }
        }
    }
}

impl PhysicalChartFrontier {
    /// Enable auxiliary work at the actual original problem. Default physical
    /// construction/advance remains unchanged and does not execute this work.
    pub fn enable_initial_auxiliary(&mut self, b: &mut Budget) -> Result<()> {
        if self.auxiliary.is_some() {
            return Err(Error::Invalid("auxiliary work already enabled"));
        }
        let FrontierOrigin::Initial(problem) = &self.origin else {
            return Err(Error::Invalid(
                "initial auxiliary needs actual initial problem",
            ));
        };
        self.auxiliary = Some(AuxiliaryFrontier::initial(
            problem.clone(),
            self.blowup.clone(),
            format!("{}_auxiliary", self.namespace),
            b,
        )?);
        Ok(())
    }
    /// Reuse a COMPLETE inventory from the exact current physical source. This
    /// does not accept a vector of caller-authored power states.
    pub fn enable_carried_auxiliary(
        &mut self,
        prior: Arc<PhysicalPowerInventory>,
        b: &mut Budget,
    ) -> Result<()> {
        if self.auxiliary.is_some() {
            return Err(Error::Invalid("auxiliary work already enabled"));
        }
        let FrontierOrigin::Embedded { state, .. } = &self.origin else {
            return Err(Error::Invalid(
                "carried auxiliary needs actual embedded source",
            ));
        };
        if !Arc::ptr_eq(state, prior.state()) {
            return Err(Error::Invalid("carried auxiliary exact prior state"));
        }
        self.auxiliary = Some(AuxiliaryFrontier::continued(
            prior,
            self.blowup.clone(),
            format!("{}_auxiliary", self.namespace),
            b,
        )?);
        Ok(())
    }
    /// Retire only by this frontier's checked source-bound cycle start. The old
    /// inventory stays at its prior frame; new children use the new real tree.
    pub fn enable_restarted_auxiliary(
        &mut self,
        prior: Arc<PhysicalPowerInventory>,
        b: &mut Budget,
    ) -> Result<()> {
        if self.auxiliary.is_some() {
            return Err(Error::Invalid("auxiliary work already enabled"));
        }
        let FrontierOrigin::Fresh(start) = &self.origin else {
            return Err(Error::Invalid(
                "restarted auxiliary needs actual cycle start",
            ));
        };
        self.auxiliary = Some(AuxiliaryFrontier::fresh(
            start.clone(),
            prior,
            self.blowup.clone(),
            format!("{}_auxiliary", self.namespace),
            b,
        )?);
        Ok(())
    }
    pub fn auxiliary_work(&self) -> Option<&[PhysicalAuxiliaryWork]> {
        self.auxiliary.as_ref().map(|a| a.work())
    }
    /// Complete accounting may include physical terminal/retired ancestry. It
    /// does not mean every auxiliary J has empty cosupport. Disabled is false.
    pub fn auxiliary_accounting_complete(&self) -> bool {
        self.auxiliary.as_ref().is_some_and(|a| a.complete())
    }
    pub fn ready_auxiliary_inventory(&self, index: usize) -> Option<&Arc<PhysicalPowerInventory>> {
        match self.auxiliary_work()?.get(index)? {
            PhysicalAuxiliaryWork::Ready(p) => Some(p),
            _ => None,
        }
    }
    /// Separate from physical `advance`: inspected nonterminal physical charts
    /// remain eligible even when physical advance returns AlreadyComplete.
    pub fn advance_auxiliary(
        &mut self,
        index: usize,
        b: &mut Budget,
    ) -> Result<PhysicalAuxiliaryAdvance> {
        let physical = self
            .work
            .get(index)
            .ok_or(Error::Invalid("auxiliary physical chart index"))?;
        self.auxiliary
            .as_mut()
            .ok_or(Error::Invalid("auxiliary work is disabled"))?
            .advance(index, physical, b)
    }
}
