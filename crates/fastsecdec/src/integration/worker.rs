use std::{fmt::Display, time::Instant};

use fastsecdec_qmc::{Korobov2, Korobov3, QmcPartial, QmcPlan, QmcWorkPackage};
use serde::{Deserialize, Serialize};

use super::{
    IntegrationError, Periodization, Result,
    weights::{WeightLayout, WeightPartial},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QmcTask {
    pub(crate) content_id: String,
    pub(crate) epoch: u64,
    pub(crate) sector_id: u64,
    pub(crate) periodization: Periodization,
    pub(crate) work: QmcWorkPackage,
}

impl QmcTask {
    pub fn sector_id(&self) -> u64 {
        self.sector_id
    }
    pub fn point_count(&self) -> u64 {
        self.work.point_count()
    }
    pub fn work(&self) -> QmcWorkPackage {
        self.work
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QmcReturn {
    pub(crate) task: QmcTask,
    pub(crate) partial: QmcPartial,
    pub(crate) worker_seconds: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) weights: Option<WeightPartial>,
}

impl QmcReturn {
    pub fn from_partial(task: QmcTask, partial: QmcPartial, worker_seconds: f64) -> Result<Self> {
        if !worker_seconds.is_finite() || worker_seconds < 0.0 || task.work != partial.work() {
            return Err(IntegrationError::InvalidReturn(
                "invalid timing or mismatched partial interval".into(),
            ));
        }
        Ok(Self {
            task,
            partial: partial.finish()?,
            worker_seconds,
            weights: None,
        })
    }
    pub fn task(&self) -> &QmcTask {
        &self.task
    }
    pub fn worker_seconds(&self) -> f64 {
        self.worker_seconds
    }
}

/// A reusable worker context. Point generation and full-vector evaluation occur
/// here, on whichever thread/process the caller selected. Clone once per worker
/// and sector; no coordinate lattice or worker pool is owned by the session.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QmcWorker {
    pub(crate) content_id: String,
    pub(crate) epoch: u64,
    pub(crate) sector_id: u64,
    pub(crate) plan: QmcPlan,
    pub(crate) output_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) weight_layout: Option<WeightLayout>,
    pub(crate) periodization: Periodization,
    #[serde(skip)]
    pub(crate) point: Vec<f64>,
    #[serde(skip)]
    pub(crate) values: Vec<f64>,
}

impl QmcWorker {
    pub fn plan(&self) -> &QmcPlan {
        &self.plan
    }

    pub fn evaluate<E: Display>(
        &mut self,
        task: QmcTask,
        mut evaluate: impl FnMut(&[f64], &mut [f64]) -> std::result::Result<(), E>,
    ) -> Result<QmcReturn> {
        self.evaluate_with_weight(task, |point, _, output| evaluate(point, output))
    }

    /// Expose the known periodization weight for evaluation error budgeting.
    /// The callback writes unweighted coefficients; this worker applies the
    /// supplied weight exactly once after the callback returns.
    pub fn evaluate_with_weight<E: Display>(
        &mut self,
        task: QmcTask,
        evaluate: impl FnMut(&[f64], f64, &mut [f64]) -> std::result::Result<(), E>,
    ) -> Result<QmcReturn> {
        self.evaluate_inner(task, false, evaluate)
    }

    /// Accumulate final weighted coefficients supplied by the callback.
    /// The callback receives the periodization weight and must apply it before
    /// writing the vector. This method never multiplies that vector again.
    pub fn evaluate_weighted<E: Display>(
        &mut self,
        task: QmcTask,
        evaluate: impl FnMut(&[f64], f64, &mut [f64]) -> std::result::Result<(), E>,
    ) -> Result<QmcReturn> {
        self.evaluate_inner(task, true, evaluate)
    }

    /// Evaluate bounded point-major matrices after the native periodization
    /// transform. The callback writes final weighted complete Laurent vectors;
    /// the native partial receives rows in their original lattice order.
    pub fn evaluate_weighted_batch<E: Display>(
        &mut self,
        task: QmcTask,
        batch_size: usize,
        evaluate: impl FnMut(&[f64], &[f64], &mut [f64]) -> std::result::Result<(), E>,
    ) -> Result<QmcReturn> {
        self.evaluate_batch_inner(task, batch_size, true, evaluate)
    }

    fn evaluate_inner<E: Display>(
        &mut self,
        task: QmcTask,
        already_weighted: bool,
        mut evaluate: impl FnMut(&[f64], f64, &mut [f64]) -> std::result::Result<(), E>,
    ) -> Result<QmcReturn> {
        self.evaluate_batch_inner(task, 1, already_weighted, |point, weights, out| {
            evaluate(point, weights[0], out)
        })
    }

    fn evaluate_batch_inner<E: Display>(
        &mut self,
        task: QmcTask,
        batch_size: usize,
        already_weighted: bool,
        mut evaluate: impl FnMut(&[f64], &[f64], &mut [f64]) -> std::result::Result<(), E>,
    ) -> Result<QmcReturn> {
        if batch_size == 0 {
            return Err(IntegrationError::InvalidReturn(
                "evaluation batch size must be positive".into(),
            ));
        }
        if task.content_id != self.content_id
            || task.epoch != self.epoch
            || task.sector_id != self.sector_id
            || task.periodization != self.periodization
        {
            return Err(IntegrationError::InvalidReturn(
                "task belongs to another integral, sector or allocation".into(),
            ));
        }
        let mut partial = QmcPartial::new(&self.plan, task.work, self.output_count)?;
        if let Some(layout) = &self.weight_layout {
            layout.validate(self.output_count)?;
        }
        let mut statistics = self
            .weight_layout
            .as_ref()
            .map(|layout| {
                WeightPartial::new(
                    task.work.start(),
                    task.point_count(),
                    self.plan.rule().points(),
                    layout.len(),
                )
            })
            .transpose()?;
        let mut magnitudes = Vec::new();
        self.point.resize(self.plan.dimension(), 0.0);
        let mut points = Vec::new();
        let mut weights = Vec::new();
        let started = Instant::now();
        let end = task.work.start() + task.work.point_count();
        for start in (task.work.start()..end).step_by(batch_size) {
            let rows = (batch_size as u64).min(end - start) as usize;
            points.clear();
            weights.clear();
            for index in start..start + rows as u64 {
                self.plan.point(index, &mut self.point)?;
                let weight = match self.periodization {
                    Periodization::None => 1.0,
                    Periodization::Korobov3 => Korobov3::transform_in_place(&mut self.point)?,
                    Periodization::Korobov2 => Korobov2::transform_in_place(&mut self.point)?,
                };
                points.extend_from_slice(&self.point);
                weights.push(weight);
            }
            let output_len = rows.checked_mul(self.output_count).ok_or_else(|| {
                IntegrationError::InvalidReturn("evaluation batch dimensions overflow".into())
            })?;
            self.values.resize(output_len, f64::NAN);
            self.values.fill(f64::NAN);
            evaluate(&points, &weights, &mut self.values)
                .map_err(|error| IntegrationError::Evaluation(error.to_string()))?;
            for (row, weight) in weights.iter().enumerate() {
                let values =
                    &mut self.values[row * self.output_count..(row + 1) * self.output_count];
                if !already_weighted {
                    for value in values.iter_mut() {
                        *value *= weight;
                    }
                }
                partial.push(values)?;
                if let (Some(layout), Some(stats)) = (&self.weight_layout, &mut statistics) {
                    layout.magnitudes(values, 1.0, &mut magnitudes);
                    let shift =
                        (start + row as u64) / self.plan.rule().points() - stats.first_shift;
                    stats.groups[shift as usize].push(&magnitudes, &magnitudes)?;
                }
            }
        }
        let mut returned = QmcReturn::from_partial(task, partial, started.elapsed().as_secs_f64())?;
        returned.weights = statistics;
        Ok(returned)
    }
}
