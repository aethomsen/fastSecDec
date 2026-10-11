//! Complete target covers for one retained physical chart, with caller-owned steps.
use super::super::*;
use super::{helpers::Result, restrict_relative_chart};
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct StrictSupportRefinement {
    chart: Arc<RelativeRecursiveChart>,
    preparation: Arc<StrictSupportPreparation>,
    cover: Arc<VerifiedOpenCover>,
}
impl StrictSupportRefinement {
    pub fn prepare(
        chart: Arc<RelativeRecursiveChart>,
        preparation: Arc<StrictSupportPreparation>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        if !Arc::ptr_eq(chart.geometry(), preparation.geometry())
            || !Arc::ptr_eq(chart.geometry().frame(), chart.history().ledger().frame())
        {
            return Err(Error::Invalid("strict support refinement actual chart"));
        }
        let cover = preparation.physical_cover(b)?;
        Ok(Arc::new(Self {
            chart,
            preparation,
            cover,
        }))
    }
    pub fn chart(&self) -> &Arc<RelativeRecursiveChart> {
        &self.chart
    }
    pub fn preparation(&self) -> &Arc<StrictSupportPreparation> {
        &self.preparation
    }
    pub fn cover(&self) -> &Arc<VerifiedOpenCover> {
        &self.cover
    }
}
#[derive(Clone, Debug)]
pub enum ContinuedRefinementState {
    Pending {
        reason: Option<&'static str>,
    },
    EmptyPrincipal {
        receipt: Box<CoverLocalizationProduction>,
    },
    Carried {
        localization: Arc<LocalizedCoverHistory>,
        chart: Arc<ContinuedSupportedChart>,
    },
}
#[derive(Clone, Debug)]
pub struct ContinuedRefinementFrontier {
    center: Arc<ContinuedSupportedCenter>,
    owner: Arc<StrictSupportRefinement>,
    namespace: String,
    states: Vec<ContinuedRefinementState>,
}
impl ContinuedRefinementFrontier {
    pub fn new(
        center: Arc<ContinuedSupportedCenter>,
        owner: Arc<StrictSupportRefinement>,
        namespace: String,
        b: &mut Budget,
    ) -> Result<Self> {
        if !matches!(owner.chart.geometry().center().origin(),RecursiveCenterOrigin::SupportedProblem(c)if Arc::ptr_eq(c,&center))
        {
            return Err(Error::Invalid("continued refinement original center owner"));
        }
        b.reserve_slots(owner.cover.opens().len())?;
        Ok(Self {
            states: vec![
                ContinuedRefinementState::Pending { reason: None };
                owner.cover.opens().len()
            ],
            center,
            owner,
            namespace,
        })
    }
    pub fn owner(&self) -> &Arc<StrictSupportRefinement> {
        &self.owner
    }
    pub fn center(&self) -> &Arc<ContinuedSupportedCenter> {
        &self.center
    }
    pub fn states(&self) -> &[ContinuedRefinementState] {
        &self.states
    }
    /// Full original chart inventory only; not global resolution or a real atlas.
    pub fn complete_inventory(&self) -> bool {
        self.states.len() == self.owner.cover.opens().len()
            && self
                .states
                .iter()
                .all(|s| !matches!(s, ContinuedRefinementState::Pending { .. }))
    }
    /// Accepted opens are immutable. Budget retry is in-memory, not durable.
    pub fn advance(&mut self, index: usize, b: &mut Budget) -> Result<ContinuedChartAdvance> {
        let state = self
            .states
            .get(index)
            .ok_or(Error::Invalid("continued refinement open index"))?;
        if !matches!(state, ContinuedRefinementState::Pending { .. }) {
            return Ok(ContinuedChartAdvance::AlreadyComplete);
        }
        match self.one(index, b) {
            Ok(state) => {
                self.states[index] = state;
                Ok(ContinuedChartAdvance::Complete)
            }
            Err(Error::ResourceIncomplete(reason)) => {
                self.states[index] = ContinuedRefinementState::Pending {
                    reason: Some(reason),
                };
                Ok(ContinuedChartAdvance::Incomplete { reason })
            }
            Err(e) => Err(e),
        }
    }
    fn one(&self, index: usize, b: &mut Budget) -> Result<ContinuedRefinementState> {
        let open = match localize_verified_cover_open(
            self.owner.chart.history().clone(),
            self.owner.cover.clone(),
            index,
            &format!("{}_open{index}", self.namespace),
            b,
        )? {
            CoverLocalizationProduction::Complete(o) => Arc::new(*o),
            receipt @ CoverLocalizationProduction::Empty { .. } => {
                return Ok(ContinuedRefinementState::EmptyPrincipal {
                    receipt: Box::new(receipt),
                });
            }
            CoverLocalizationProduction::Incomplete { reason, .. } => {
                return Err(Error::ResourceIncomplete(reason));
            }
        };
        let chart = restrict_relative_chart(self.owner.chart.clone(), open.clone(), b)?;
        // Every non-guard open makes only THIS strict support empty. Keep all
        // other ancestors and the ambient contribution for actual continuation.
        if index > 0 {
            let local = chart.geometry().frame().local();
            let support = open
                .open()
                .extension()
                .ideal(self.owner.preparation.saturation().result(), b)?;
            if !local.ideal().sum(&support, b)?.contains(
                &local.ring().one(),
                local.unit_relations(),
                b,
            )? {
                return Err(Error::Invalid(
                    "support complement did not empty its original support",
                ));
            }
        }
        let carried = ContinuedSupportedChart::prepare(
            self.center.clone(),
            chart,
            &format!("{}_carry{index}", self.namespace),
            b,
        )?;
        Ok(ContinuedRefinementState::Carried {
            localization: open,
            chart: carried,
        })
    }
}
