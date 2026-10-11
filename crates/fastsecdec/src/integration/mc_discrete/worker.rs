use crate::integration::weights::{WeightLayout, WeightStatistics};
use crate::integration::{McLiveView, Result};
use numerica::numerical_integration::DiscreteGrid;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
mod batch;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HavanaDiscreteTask {
    pub(crate) batch: u32,
    pub(crate) points: usize,
    pub(crate) grid_id: [u8; 32],
    pub(crate) rng_state: [u8; 32],
}
impl HavanaDiscreteTask {
    pub fn batch(&self) -> u32 {
        self.batch
    }
    pub fn point_count(&self) -> usize {
        self.points
    }
}
#[derive(Clone, Debug)]
pub struct HavanaDiscreteReturn {
    pub(crate) task: HavanaDiscreteTask,
    pub(crate) mean: Vec<f64>,
    pub(crate) weights: Option<WeightStatistics>,
    pub(crate) sector_means: Vec<Vec<f64>>,
    pub(crate) counts: Vec<u64>,
    pub(crate) sector_seconds: Vec<f64>,
    pub(crate) worker_seconds: f64,
    pub(crate) training: Option<DiscreteGrid<f64>>,
}
impl HavanaDiscreteReturn {
    pub fn task(&self) -> &HavanaDiscreteTask {
        &self.task
    }
    pub fn mean(&self) -> &[f64] {
        &self.mean
    }
    /// Actual sector selections in the integration problem's sector order.
    pub fn sector_counts(&self) -> &[u64] {
        &self.counts
    }
    pub fn worker_seconds(&self) -> f64 {
        self.worker_seconds
    }
    pub fn sector_worker_seconds(&self) -> &[f64] {
        &self.sector_seconds
    }
}
#[derive(Clone, Debug)]
pub struct HavanaDiscreteWorker {
    pub(crate) sector_ids: Vec<u64>,
    pub(crate) grid_id: [u8; 32],
    pub(crate) grid: DiscreteGrid<f64>,
    pub(crate) points: usize,
    pub(crate) outputs: usize,
    pub(crate) weight_layout: Option<WeightLayout>,
    pub(crate) training: bool,
}
impl HavanaDiscreteWorker {
    pub fn sector_ids(&self) -> &[u64] {
        &self.sector_ids
    }
    pub fn evaluate<E: Display>(
        &mut self,
        task: HavanaDiscreteTask,
        mut evaluate: impl FnMut(u64, &[f64], &mut [f64]) -> std::result::Result<(), E>,
    ) -> Result<HavanaDiscreteReturn> {
        self.evaluate_inner(task, false, |id, x, _, out| evaluate(id, x, out), |_| {})
    }
    /// The callback receives the full inverse sector/coordinate probability.
    /// It writes unweighted coefficients; this worker applies the weight once.
    pub fn evaluate_with_weight<E: Display>(
        &mut self,
        task: HavanaDiscreteTask,
        evaluate: impl FnMut(u64, &[f64], f64, &mut [f64]) -> std::result::Result<(), E>,
    ) -> Result<HavanaDiscreteReturn> {
        self.evaluate_inner(task, false, evaluate, |_| {})
    }
    /// The callback writes final importance-weighted coefficients. The root
    /// native sampling weight is never applied to these values again.
    pub fn evaluate_weighted<E: Display>(
        &mut self,
        task: HavanaDiscreteTask,
        evaluate: impl FnMut(u64, &[f64], f64, &mut [f64]) -> std::result::Result<(), E>,
    ) -> Result<HavanaDiscreteReturn> {
        self.evaluate_inner(task, true, evaluate, |_| {})
    }
    /// Observe native prefix statistics without admitting an incomplete batch.
    pub fn evaluate_weighted_observed<E: Display>(
        &mut self,
        task: HavanaDiscreteTask,
        evaluate: impl FnMut(u64, &[f64], f64, &mut [f64]) -> std::result::Result<(), E>,
        observe: impl FnMut(McLiveView<'_>),
    ) -> Result<HavanaDiscreteReturn> {
        self.evaluate_inner(task, true, evaluate, observe)
    }
    /// Evaluate bounded native sample chunks, grouped by sector. Every sector
    /// callback receives point-major coordinates, full sampling weights and a
    /// point-major output matrix. Within each sector the original sample order
    /// is preserved, then results are reduced/trained in global sample order.
    pub fn evaluate_weighted_batch_observed<E: Display>(
        &mut self,
        task: HavanaDiscreteTask,
        batch_size: usize,
        evaluate: impl FnMut(u64, &[f64], &[f64], &mut [f64]) -> std::result::Result<(), E>,
        observe: impl FnMut(McLiveView<'_>),
    ) -> Result<HavanaDiscreteReturn> {
        self.evaluate_batch_inner(task, batch_size, true, evaluate, observe)
    }

    fn evaluate_inner<E: Display>(
        &mut self,
        task: HavanaDiscreteTask,
        weighted: bool,
        mut evaluate: impl FnMut(u64, &[f64], f64, &mut [f64]) -> std::result::Result<(), E>,
        observe: impl FnMut(McLiveView<'_>),
    ) -> Result<HavanaDiscreteReturn> {
        self.evaluate_batch_inner(
            task,
            1,
            weighted,
            |id, point, weights, out| evaluate(id, point, weights[0], out),
            observe,
        )
    }
}
