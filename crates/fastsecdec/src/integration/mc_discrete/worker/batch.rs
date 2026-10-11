//! Group a bounded native sampling chunk, retaining its exact global order.
use super::*;
use crate::integration::{IntegrationError, mc::training_envelope, mc_live::CenteredAccumulator};
use numerica::numerical_integration::{MonteCarloRng, Sample};
use std::time::Instant;

#[derive(Default)]
struct SectorBatch {
    rows: Vec<usize>,
    points: Vec<f64>,
    weights: Vec<f64>,
    values: Vec<f64>,
}

impl HavanaDiscreteWorker {
    pub(super) fn evaluate_batch_inner<E: Display>(
        &mut self,
        task: HavanaDiscreteTask,
        batch_size: usize,
        weighted: bool,
        mut evaluate: impl FnMut(u64, &[f64], &[f64], &mut [f64]) -> std::result::Result<(), E>,
        mut observe: impl FnMut(McLiveView<'_>),
    ) -> Result<HavanaDiscreteReturn> {
        if batch_size == 0 {
            return Err(IntegrationError::InvalidReturn(
                "evaluation batch size must be positive".into(),
            ));
        }
        if task.grid_id != self.grid_id || task.points != self.points || task.rng_state == [0; 32] {
            return Err(IntegrationError::InvalidReturn(
                "discrete Havana task has a different grid or batch design".into(),
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
        let mut local = Vec::new();
        let started = Instant::now();
        let mut grid = super::super::grid::clone_without_samples(&self.grid)?;
        let mut rng = MonteCarloRng::import(task.rng_state);
        let mut total = CenteredAccumulator::new(self.outputs);
        let mut sectors = (0..self.sector_ids.len())
            .map(|_| CenteredAccumulator::new(self.outputs))
            .collect::<Vec<_>>();
        let mut groups = (0..self.sector_ids.len())
            .map(|_| SectorBatch::default())
            .collect::<Vec<_>>();
        let mut counts = vec![0u64; self.sector_ids.len()];
        let mut seconds = vec![0.0; self.sector_ids.len()];
        let mut samples = Vec::new();
        let mut sample_seconds = Vec::new();
        let mut evaluation_seconds = Vec::new();
        let mut values = Vec::new();
        for start in (0..task.points).step_by(batch_size) {
            let rows = batch_size.min(task.points - start);
            samples.resize_with(rows, Sample::new);
            sample_seconds.clear();
            evaluation_seconds.resize(rows, 0.0);
            evaluation_seconds.fill(0.0);
            for group in &mut groups {
                group.rows.clear();
                group.points.clear();
                group.weights.clear();
            }
            for (row, sample) in samples.iter_mut().enumerate() {
                let sample_started = Instant::now();
                grid.sample(&mut rng, sample);
                let Sample::Discrete(weight, index, Some(child)) = &*sample else {
                    unreachable!("native nested discrete grid sample")
                };
                let Sample::Continuous(_, point) = child.as_ref() else {
                    unreachable!("native continuous child sample")
                };
                if !weight.is_finite() || *weight <= 0.0 {
                    return Err(IntegrationError::Evaluation(
                        "native discrete importance weight must be finite and positive".into(),
                    ));
                }
                let group = &mut groups[*index];
                group.rows.push(row);
                group.points.extend_from_slice(point);
                group.weights.push(*weight);
                sample_seconds.push(sample_started.elapsed().as_secs_f64());
            }
            let output_len = rows.checked_mul(self.outputs).ok_or_else(|| {
                IntegrationError::InvalidReturn("evaluation batch dimensions overflow".into())
            })?;
            values.resize(output_len, f64::NAN);
            for (index, group) in groups.iter_mut().enumerate() {
                if group.rows.is_empty() {
                    continue;
                }
                group
                    .values
                    .resize(group.rows.len() * self.outputs, f64::NAN);
                group.values.fill(f64::NAN);
                let evaluation_started = Instant::now();
                evaluate(
                    self.sector_ids[index],
                    &group.points,
                    &group.weights,
                    &mut group.values,
                )
                .map_err(|e| IntegrationError::Evaluation(e.to_string()))?;
                // A matrix call has one measured sector span, not per-row
                // stopwatches. Its equal shares sum to that measured span.
                let share = evaluation_started.elapsed().as_secs_f64() / group.rows.len() as f64;
                for (source, target) in group.rows.iter().enumerate() {
                    values[target * self.outputs..(target + 1) * self.outputs].copy_from_slice(
                        &group.values[source * self.outputs..(source + 1) * self.outputs],
                    );
                    evaluation_seconds[*target] = share;
                }
            }
            // Restore original global RNG order for all native training and
            // full-vector/sector accumulators, including cross-covariance.
            for (row, sample) in samples.iter().enumerate() {
                let reduction_started = Instant::now();
                let Sample::Discrete(weight, index, _) = sample else {
                    unreachable!()
                };
                let values = &mut values[row * self.outputs..(row + 1) * self.outputs];
                if values
                    .iter()
                    .any(|v| !v.is_finite() || (!weighted && !(v * weight).is_finite()))
                {
                    return Err(IntegrationError::Evaluation(
                        "nonfinite discrete importance-weighted coefficient".into(),
                    ));
                }
                if self.training {
                    let envelope = values.iter().map(|v| v.abs()).fold(0.0, f64::max);
                    grid.add_training_sample(
                        sample,
                        if weighted {
                            training_envelope(envelope, *weight)?
                        } else {
                            envelope
                        },
                    )
                    .map_err(IntegrationError::Evaluation)?;
                }
                if !weighted {
                    for value in values.iter_mut() {
                        *value *= weight;
                    }
                }
                total.add(values);
                if let (Some(layout), Some(stats)) = (&self.weight_layout, &mut statistics) {
                    layout.magnitudes(values, 1.0, &mut absolute);
                    // Native grid owns the immutable proposal for this entire batch.
                    // Undo only inverse discrete selection, as native Havana does.
                    layout.magnitudes(values, grid.bins[*index].pdf, &mut local);
                    stats.push(&absolute, &local)?;
                }
                sectors[*index].add(values);
                counts[*index] += 1;
                let point_seconds = sample_seconds[row]
                    + evaluation_seconds[row]
                    + reduction_started.elapsed().as_secs_f64();
                seconds[*index] += point_seconds;
                observe(McLiveView {
                    batch: task.batch,
                    sector_id: None,
                    points: (start + row + 1) as u64,
                    total: Some(&total),
                    last_point_timing: Some((self.sector_ids[*index], point_seconds)),
                    sectors: &sectors,
                    sector_ids: &self.sector_ids,
                });
            }
        }
        let mean = total.mean(task.points);
        let sector_means = sectors
            .into_iter()
            .map(|acc| acc.mean(task.points))
            .collect::<Vec<_>>();
        if mean
            .iter()
            .chain(sector_means.iter().flatten())
            .any(|v| !v.is_finite())
        {
            return Err(IntegrationError::Evaluation(
                "nonfinite discrete batch mean".into(),
            ));
        }
        Ok(HavanaDiscreteReturn {
            task,
            mean,
            weights: statistics,
            sector_means,
            counts,
            sector_seconds: seconds,
            worker_seconds: started.elapsed().as_secs_f64(),
            training: self.training.then_some(grid),
        })
    }
}
