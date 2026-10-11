//! Depth-independent original-polynomial transport through issued physical maps.
//! This state follows one algebraic lineage, not a glued real integration atlas.
use super::super::super::*;
use super::super::{
    general_transform::controlled_marked, helpers::Result, presentation::PresentationOrigin,
};
use super::*;
use std::sync::Arc;

/// The actual pre-first-map source, shared by every descendant and sibling.
#[derive(Clone, Debug)]
pub struct OriginalPhysicalRoot {
    owner: Arc<EmbeddedChildCycle>,
    source: Arc<MarkedIdeal>,
    history: Arc<ResolutionHistory>,
}
impl OriginalPhysicalRoot {
    pub fn owner(&self) -> &Arc<EmbeddedChildCycle> {
        &self.owner
    }
    pub fn source(&self) -> &Arc<MarkedIdeal> {
        &self.source
    }
    pub fn history(&self) -> &Arc<ResolutionHistory> {
        &self.history
    }
}
#[derive(Clone, Debug)]
pub enum PhysicalLineageEdge {
    Bootstrap(Arc<AmbientCycleCarry>),
    Initial {
        problem: Arc<OriginalPhysicalProblem>,
        chart: Arc<RelativeRecursiveChart>,
    },
    FreshCycle {
        start: Arc<PhysicalCycleStart>,
        chart: Arc<RelativeRecursiveChart>,
    },
    Continued {
        source: Arc<OriginalPhysicalState>,
        transition: Arc<EmbeddedTransition>,
    },
}
#[derive(Clone, Debug)]
enum ActivePhysicalCycle {
    Bootstrap(Arc<AmbientCycleCarry>),
    First {
        origin: Arc<EmbeddedChildCycle>,
        chart: Arc<RelativeRecursiveChart>,
    },
    Continued {
        origin: Arc<EmbeddedChildCycle>,
        transition: Arc<EmbeddedTransition>,
    },
}
/// Immutable evidence at the LAST admissible frame. Retired G is never pulled
/// through subsequent centers or used to claim current empty cosupport.
#[derive(Clone, Debug)]
pub struct RetiredPhysicalCycle {
    source: Arc<OriginalPhysicalState>,
    completion: PhysicalCycleCompletion,
    previous: Option<Arc<RetiredPhysicalCycle>>,
    count: usize,
}
impl RetiredPhysicalCycle {
    pub fn previous(&self) -> Option<&Arc<Self>> {
        self.previous.as_ref()
    }
    pub fn count(&self) -> usize {
        self.count
    }
    pub fn source(&self) -> &Arc<OriginalPhysicalState> {
        &self.source
    }
    pub fn completion(&self) -> &PhysicalCycleCompletion {
        &self.completion
    }
}
#[derive(Clone, Debug)]
pub struct OriginalPhysicalState {
    root: Arc<OriginalPhysicalRoot>,
    edge: PhysicalLineageEdge,
    chart: Arc<RelativeRecursiveChart>,
    generators: Vec<OriginalGeneratorPullback>,
    exceptional: AccumulatedExceptional,
    jacobian: Poly,
    active: ActivePhysicalCycle,
    retired: Option<Arc<RetiredPhysicalCycle>>,
    depth: usize,
}
impl OriginalPhysicalState {
    pub fn from_ambient(carry: Arc<AmbientCycleCarry>, b: &mut Budget) -> Result<Arc<Self>> {
        b.reserve_slots(carry.generators().len())?;
        let root = Arc::new(OriginalPhysicalRoot {
            owner: carry.original().clone(),
            source: carry.original_source().clone(),
            history: carry.previous().geometry().center().history().clone(),
        });
        if !Arc::ptr_eq(root.source(), root.owner().center().parent_source()) {
            return Err(Error::Invalid("physical root original source"));
        }
        Ok(Arc::new(Self {
            root,
            edge: PhysicalLineageEdge::Bootstrap(carry.clone()),
            chart: carry.chart().clone(),
            generators: carry.generators().to_vec(),
            exceptional: carry.exceptional().clone(),
            jacobian: carry.original_coordinate_jacobian().clone(),
            active: ActivePhysicalCycle::Bootstrap(carry),
            retired: None,
            depth: 2,
        }))
    }
    /// Accept an existing full C/J/history transition from this exact active
    /// lineage. A newly supplied equal-polynomial presentation has no authority.
    pub fn continue_embedded(
        self: &Arc<Self>,
        transition: Arc<EmbeddedTransition>,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let origin = match &self.active {
            ActivePhysicalCycle::First { origin, .. }
            | ActivePhysicalCycle::Continued { origin, .. } => origin,
            ActivePhysicalCycle::Bootstrap(_) => {
                return Err(Error::ResourceIncomplete(
                    "bootstrap cycle needs its checked residual restart",
                ));
            }
        };
        if !Arc::ptr_eq(transition.source().original_cycle(), origin)
            || !self.owns_presentation(transition.source())
            || !Arc::ptr_eq(transition.source().chart(), &self.chart)
            || !Arc::ptr_eq(
                transition.chart().parent().source(),
                self.chart.parent().target(),
            )
            || !Arc::ptr_eq(
                transition.chart().geometry().center().history(),
                self.chart.history(),
            )
        {
            return Err(Error::Invalid(
                "physical continuation active source/history",
            ));
        }
        let chart = transition.chart().clone();
        let exceptional = self.exceptional.advance(&chart, b)?;
        let (generators, jacobian) = self.transport(&chart, None, &exceptional, namespace, b)?;
        let depth = self
            .depth
            .checked_add(1)
            .ok_or(Error::ResourceIncomplete("physical lineage depth"))?;
        Ok(Arc::new(Self {
            root: self.root.clone(),
            edge: PhysicalLineageEdge::Continued {
                source: self.clone(),
                transition: transition.clone(),
            },
            chart,
            generators,
            exceptional,
            jacobian,
            active: ActivePhysicalCycle::Continued {
                origin: origin.clone(),
                transition,
            },
            retired: self.retired.clone(),
            depth,
        }))
    }
    pub(super) fn owns_presentation(&self, p: &Arc<EmbeddedPresentation>) -> bool {
        match &p.origin {
            PresentationOrigin::Incidence(drop) => self.owns_presentation(drop.source()),
            PresentationOrigin::Initial(induced) => {
                matches!(&self.active,ActivePhysicalCycle::First{origin,chart} if Arc::ptr_eq(induced.origin(),origin)&&Arc::ptr_eq(induced.ambient().chart(),chart)&&Arc::ptr_eq(induced.ambient().center(),origin.center()))
            }
            PresentationOrigin::Transition { owner, .. } => {
                matches!(&self.active,ActivePhysicalCycle::Continued{transition,..} if Arc::ptr_eq(owner,transition))
            }
        }
    }
    pub fn root(&self) -> &Arc<OriginalPhysicalRoot> {
        &self.root
    }
    pub fn edge(&self) -> &PhysicalLineageEdge {
        &self.edge
    }
    pub fn chart(&self) -> &Arc<RelativeRecursiveChart> {
        &self.chart
    }
    pub fn generators(&self) -> &[OriginalGeneratorPullback] {
        &self.generators
    }
    pub fn exceptional(&self) -> &AccumulatedExceptional {
        &self.exceptional
    }
    pub fn original_coordinate_jacobian(&self) -> &Poly {
        &self.jacobian
    }
    pub fn retired(&self) -> Option<&Arc<RetiredPhysicalCycle>> {
        self.retired.as_ref()
    }
    pub fn retired_count(&self) -> usize {
        self.retired.as_ref().map_or(0, |r| r.count)
    }
    pub fn depth(&self) -> usize {
        self.depth
    }
    pub fn active_original_cycle(&self) -> Option<&Arc<EmbeddedChildCycle>> {
        match &self.active {
            ActivePhysicalCycle::Bootstrap(_) => None,
            ActivePhysicalCycle::First { origin, .. }
            | ActivePhysicalCycle::Continued { origin, .. } => Some(origin),
        }
    }
    /// Prepare only when companion evidence is needed. Original-factor terminal
    /// certification need not construct the lower C/J support of terminal maps.
    pub fn prepare_active_companion(
        &self,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<Arc<RelativeMarkedTransform>> {
        match &self.active {
            ActivePhysicalCycle::Bootstrap(_) => Err(Error::ResourceIncomplete(
                "bootstrap companion collection adapter",
            )),
            ActivePhysicalCycle::First { origin, chart } => {
                let q = origin.center().coefficient().contact();
                let source = Arc::new(MarkedIdeal::new(
                    (**q.source().source()).clone(),
                    q.source().algebraic_maximum_order(),
                    b,
                )?);
                controlled_marked(chart.geometry().clone(), source, namespace, b)
            }
            ActivePhysicalCycle::Continued { transition, .. } => Ok(transition.companion.clone()),
        }
    }
    pub fn active_presentations(
        &self,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<Vec<Arc<EmbeddedPresentation>>> {
        match &self.active {
            ActivePhysicalCycle::Bootstrap(_) => Err(Error::ResourceIncomplete(
                "bootstrap active presentation adapter",
            )),
            ActivePhysicalCycle::First { origin, chart } => {
                let carried = carry_companion_chart(chart.clone(), namespace, b)?;
                b.reserve_slots(carried.opens().len())?;
                (0..carried.opens().len())
                    .map(|i| {
                        EmbeddedPresentation::from_induced(
                            induce_child_chart(
                                origin.clone(),
                                carried.clone(),
                                i,
                                &format!("{namespace}_{i}"),
                                b,
                            )?,
                            b,
                        )
                    })
                    .collect()
            }
            ActivePhysicalCycle::Continued { transition, .. } => transition.presentations(b),
        }
    }
}
#[derive(Clone, Debug)]
pub struct PhysicalTerminalInventory {
    state: Arc<OriginalPhysicalState>,
    factors: Arc<CompletedComponentFactors>,
    records: Vec<OriginalTerminalRecord>,
}
impl PhysicalTerminalInventory {
    pub fn prove(
        state: Arc<OriginalPhysicalState>,
        factors: Arc<CompletedComponentFactors>,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let records = super::terminal::prove_original_terminal(
            super::terminal::OriginalTerminalInput {
                source: state.chart().parent().target(),
                history: state.chart().history(),
                exceptional: state.exceptional(),
                generators: state.generators(),
                jacobian: state.original_coordinate_jacobian(),
            },
            &factors,
            namespace,
            b,
        )?;
        Ok(Arc::new(Self {
            state,
            factors,
            records,
        }))
    }
    pub fn state(&self) -> &Arc<OriginalPhysicalState> {
        &self.state
    }
    pub fn factor_inventory(&self) -> &Arc<CompletedComponentFactors> {
        &self.factors
    }
    pub fn records(&self) -> &[OriginalTerminalRecord] {
        &self.records
    }
    pub fn all_local_factors_snc(&self) -> bool {
        !self.records.is_empty()
            && self
                .records
                .iter()
                .all(|r| matches!(r, OriginalTerminalRecord::Terminal { .. }))
    }
}

mod start;
mod transport;
pub use start::{PhysicalCycleCompletion, PhysicalCycleStart, PhysicalRestartOwner};

mod initial;
pub use initial::OriginalPhysicalProblem;
