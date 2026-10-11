use crate::integration::{
    IntegrationError, Periodization, QmcSettings, QmcTask, QmcWorker, Result,
    mc::{HavanaTask, HavanaWorker},
};
use fastsecdec_qmc::{QmcAccumulator, QmcPlan};
use numerica::numerical_integration::{ContinuousGrid, MonteCarloRng, Sample};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicaIdentity {
    pub content_id: String,
    pub sector_id: u64,
    pub epoch: u64,
    pub pilot: bool,
    pub replica: u64,
    pub stream: u64,
    pub run: u64,
    pub lease: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) enum Sampler {
    Qmc {
        plan: QmcPlan,
        periodization: Periodization,
    },
    Mc {
        grid: Box<ContinuousGrid<f64>>,
        training: bool,
    },
}

/// A synchronous complete-lattice/batch job. Serialize through caller-owned IPC
/// or a staging file. This job owns no evaluator and never creates workers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerialTask {
    pub(super) identity: ReplicaIdentity,
    pub(super) sampler: Sampler,
    pub(super) rng_state: [u8; 32],
    pub(super) points: u64,
    pub(super) dimension: usize,
    pub(super) outputs: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) weight_layout: Option<crate::integration::weights::WeightLayout>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerialReturn {
    pub(super) identity: ReplicaIdentity,
    pub(super) task_digest: [u8; 32],
    pub(super) mean: Vec<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) weights: Option<crate::integration::weights::WeightStatistics>,
    pub(super) training: Option<ContinuousGrid<f64>>,
    pub(super) worker_seconds: f64,
}
impl SerialReturn {
    pub fn identity(&self) -> &ReplicaIdentity {
        &self.identity
    }
    pub fn mean(&self) -> &[f64] {
        &self.mean
    }
    pub fn worker_seconds(&self) -> f64 {
        self.worker_seconds
    }
}
impl SerialTask {
    pub fn identity(&self) -> &ReplicaIdentity {
        &self.identity
    }
    pub fn sector_id(&self) -> u64 {
        self.identity.sector_id
    }
    pub fn point_count(&self) -> u64 {
        self.points
    }
    pub fn dimension(&self) -> usize {
        self.dimension
    }
    pub fn output_count(&self) -> usize {
        self.outputs
    }
    pub fn digest(&self) -> Result<[u8; 32]> {
        Ok(*blake3::hash(&serde_json::to_vec(self)?).as_bytes())
    }
    pub(super) fn qmc_sampler(
        settings: &QmcSettings,
        dimension: usize,
        points: u64,
        state: [u8; 32],
    ) -> Result<Sampler> {
        let rule = settings.allocation_rule(dimension, points, 2)?;
        let mut rng = MonteCarloRng::import(state);
        let shift = (0..dimension)
            .map(|_| (rng.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64))
            .collect();
        Ok(Sampler::Qmc {
            plan: QmcPlan::with_shifts(rule, vec![shift])?,
            periodization: settings.periodization,
        })
    }
    pub(super) fn validate(&self) -> Result<()> {
        if self.identity.content_id.is_empty()
            || self.rng_state == [0; 32]
            || self.dimension == 0
            || self.outputs == 0
            || self.points < 2
        {
            return Err(IntegrationError::InvalidReturn(
                "invalid serial job identity or dimensions".into(),
            ));
        }
        if let Some(layout) = &self.weight_layout {
            layout.validate(self.outputs)?;
        }
        match &self.sampler {
            Sampler::Qmc { plan, .. } => {
                if plan.dimension() != self.dimension
                    || plan.shift_count() != 1
                    || plan.total_points() != self.points
                {
                    return Err(IntegrationError::InvalidReturn(
                        "serial QMC job is not one complete shifted lattice".into(),
                    ));
                }
            }
            Sampler::Mc { grid, training } => {
                super::streams::mc_draws(self.points, self.dimension)?;
                if grid.continuous_dimensions.len() != self.dimension
                    || *training != self.identity.pilot
                {
                    return Err(IntegrationError::InvalidReturn(
                        "serial MC grid or phase differs".into(),
                    ));
                }
                for dim in &grid.continuous_dimensions {
                    if dim.partitioning.first() != Some(&0.)
                        || dim.partitioning.last() != Some(&1.)
                        || dim
                            .partitioning
                            .windows(2)
                            .any(|p| !p[0].is_finite() || p[0] >= p[1])
                    {
                        return Err(IntegrationError::InvalidReturn(
                            "invalid serial MC partition".into(),
                        ));
                    }
                }
            }
        }
        Ok(())
    }
    /// Callback writes the complete importance-weighted Laurent vector; this
    /// method reuses existing native workers and never applies weights twice.
    pub fn evaluate_weighted_batch<E: Display>(
        &self,
        batch_size: usize,
        evaluate: impl FnMut(&[f64], &[f64], &mut [f64]) -> std::result::Result<(), E>,
    ) -> Result<SerialReturn> {
        self.validate()?;
        let digest = self.digest()?;
        let (mean, training, worker_seconds, weights) = match &self.sampler {
            Sampler::Qmc {
                plan,
                periodization,
            } => {
                let mut worker = QmcWorker {
                    content_id: self.identity.content_id.clone(),
                    epoch: self.identity.epoch,
                    sector_id: self.identity.sector_id,
                    plan: plan.clone(),
                    output_count: self.outputs,
                    weight_layout: self.weight_layout.clone(),
                    periodization: *periodization,
                    point: Vec::new(),
                    values: Vec::new(),
                };
                let task = QmcTask {
                    content_id: self.identity.content_id.clone(),
                    epoch: self.identity.epoch,
                    sector_id: self.identity.sector_id,
                    periodization: *periodization,
                    work: plan.work(0, self.points)?,
                };
                let returned = worker.evaluate_weighted_batch(task, batch_size, evaluate)?;
                let seconds = returned.worker_seconds;
                let mut accumulator = QmcAccumulator::new(plan.clone(), self.outputs)?;
                accumulator.merge(returned.partial)?;
                let mean = accumulator.shift_estimates()?.remove(0).mean;
                (
                    mean,
                    None,
                    seconds,
                    returned.weights.map(|mut p| p.groups.remove(0)),
                )
            }
            Sampler::Mc { grid, training } => {
                let points = usize::try_from(self.points).map_err(|_| {
                    IntegrationError::Invalid("MC points exceed addressable range".into())
                })?;
                let mut worker = HavanaWorker {
                    sector_id: self.identity.sector_id,
                    grid_id: digest,
                    grid: grid.clone_without_samples(),
                    points,
                    outputs: self.outputs,
                    weight_layout: self.weight_layout.clone(),
                    training: *training,
                    sample: Sample::new(),
                    values: Vec::new(),
                };
                let task = HavanaTask {
                    sector_id: self.identity.sector_id,
                    batch: 0,
                    points,
                    grid_id: digest,
                    rng_state: self.rng_state,
                };
                let returned =
                    worker.evaluate_weighted_batch_observed(task, batch_size, evaluate, |_| {})?;
                (
                    returned.mean,
                    returned.training,
                    returned.worker_seconds,
                    returned.weights,
                )
            }
        };
        Ok(SerialReturn {
            identity: self.identity.clone(),
            task_digest: digest,
            mean,
            training,
            worker_seconds,
            weights,
        })
    }
}
