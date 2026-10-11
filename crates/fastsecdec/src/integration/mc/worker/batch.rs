//! Native Havana sampling with bounded caller-owned evaluator matrices.
use super::*;
use crate::integration::mc_live::CenteredAccumulator;
use numerica::numerical_integration::MonteCarloRng;
use std::time::Instant;

impl HavanaWorker {
    pub(super) fn evaluate_batch_inner<E: Display>(
        &mut self,
        task: HavanaTask,
        batch_size: usize,
        already_weighted: bool,
        mut evaluate: impl FnMut(&[f64], &[f64], &mut [f64]) -> std::result::Result<(), E>,
        mut observe: impl FnMut(McLiveView<'_>),
    ) -> Result<HavanaReturn> {
        if batch_size == 0 {
            return Err(IntegrationError::InvalidReturn(
                "evaluation batch size must be positive".into(),
            ));
        }
        if task.sector_id != self.sector_id
            || task.grid_id != self.grid_id
            || task.points != self.points
            || task.rng_state == [0; 32]
        {
            return Err(IntegrationError::InvalidReturn(
                "Havana task belongs to another integral, grid or production design".into(),
            ));
        }
        if let Some(layout) = &self.weight_layout {
            layout.validate(self.outputs)?;
        }
        let mut statistics = self
            .weight_layout
            .as_ref()
            .map(|l| WeightStatistics::new(l.len()));
        let mut absolute = Vec::new();
        let started = Instant::now();
        let mut grid = self.grid.clone_without_samples();
        let mut rng = MonteCarloRng::import(task.rng_state);
        let mut accumulator = CenteredAccumulator::new(self.outputs);
        let mut samples = vec![std::mem::take(&mut self.sample)];
        let mut points = Vec::new();
        let mut weights = Vec::new();
        let mut weighted_values = vec![0.0; self.outputs];
        for start in (0..task.points).step_by(batch_size) {
            let rows = batch_size.min(task.points - start);
            samples.resize_with(rows, Sample::new);
            points.clear();
            weights.clear();
            for sample in &mut samples {
                grid.sample(&mut rng, sample);
                let Sample::Continuous(weight, point) = &*sample else {
                    unreachable!("ContinuousGrid produces continuous samples")
                };
                if !weight.is_finite() || *weight <= 0.0 {
                    return Err(IntegrationError::Evaluation(
                        "native Havana sampled weight must be finite and strictly positive".into(),
                    ));
                }
                weights.push(*weight);
                points.extend_from_slice(point);
            }
            let output_len = rows.checked_mul(self.outputs).ok_or_else(|| {
                IntegrationError::InvalidReturn("evaluation batch dimensions overflow".into())
            })?;
            self.values.resize(output_len, f64::NAN);
            self.values.fill(f64::NAN);
            evaluate(&points, &weights, &mut self.values)
                .map_err(|e| IntegrationError::Evaluation(e.to_string()))?;
            // Native training changes accumulated data, not this package's
            // proposal. Keep every reduction and training insertion in RNG order.
            for (row, (sample, weight)) in samples.iter().zip(&weights).enumerate() {
                let values = &self.values[row * self.outputs..(row + 1) * self.outputs];
                if values
                    .iter()
                    .any(|v| !v.is_finite() || (!already_weighted && !(v * weight).is_finite()))
                {
                    return Err(IntegrationError::Evaluation(
                        "nonfinite importance-weighted coefficient".into(),
                    ));
                }
                for (out, value) in weighted_values.iter_mut().zip(values) {
                    *out = if already_weighted {
                        *value
                    } else {
                        value * weight
                    };
                }
                accumulator.add(&weighted_values);
                if let (Some(layout), Some(stats)) = (&self.weight_layout, &mut statistics) {
                    layout.magnitudes(&weighted_values, 1.0, &mut absolute);
                    stats.push(&absolute, &absolute)?;
                }
                if self.training {
                    let mut envelope = values.iter().map(|v| v.abs()).fold(0.0, f64::max);
                    if already_weighted {
                        envelope = training_envelope(envelope, *weight)?;
                    }
                    grid.add_training_sample(sample, envelope)
                        .map_err(IntegrationError::Evaluation)?;
                }
                observe(McLiveView {
                    batch: task.batch,
                    sector_id: Some(self.sector_id),
                    points: (start + row + 1) as u64,
                    total: None,
                    last_point_timing: None,
                    sectors: std::slice::from_ref(&accumulator),
                    sector_ids: &[self.sector_id],
                });
            }
        }
        self.sample = samples.pop().unwrap_or_default();
        let mean = accumulator.conditional_mean();
        if mean.iter().any(|v| !v.is_finite()) {
            return Err(IntegrationError::Evaluation(
                "nonfinite Monte Carlo batch mean".into(),
            ));
        }
        Ok(HavanaReturn {
            task,
            mean,
            weights: statistics,
            worker_seconds: started.elapsed().as_secs_f64(),
            training: self.training.then_some(grid),
        })
    }
}
