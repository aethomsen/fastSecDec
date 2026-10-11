//! Caller-stepped one-stage physical inventory, terminal checks before C/J work.
//! Progress is in memory. Native resource refusals do not discard accepted charts.
use super::super::super::*;
use super::super::helpers::Result;
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
