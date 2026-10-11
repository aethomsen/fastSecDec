mod checkpoint;
mod weights;
use crate::integration::weights::{WeightLayout, WeightStatistics};
mod observation;
mod refinement;
use super::grid::clone_without_samples;
use super::{
    HavanaDiscreteReturn, HavanaDiscreteSettings, HavanaDiscreteTask, HavanaDiscreteWorker,
};
use crate::{
    integration::{
        IntegrationError, IntegrationProblem, Result,
        streams::{Streams, mc_draws},
    },
    status::IntegrationStage,
};
use numerica::numerical_integration::{DiscreteGrid, Grid};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Serialize, Deserialize)]
struct BatchRecord {
    task: HavanaDiscreteTask,
    mean: Vec<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    weights: Option<WeightStatistics>,
    sector_means: Vec<Vec<f64>>,
    counts: Vec<u64>,
    sector_seconds: Vec<f64>,
    worker_seconds: f64,
}
#[derive(Clone, Debug)]
pub struct HavanaDiscreteSession {
    problem: IntegrationProblem,
    settings: HavanaDiscreteSettings,
    stage: IntegrationStage,
    epoch: u64,
    grid: Option<DiscreteGrid<f64>>,
    training: Option<DiscreteGrid<f64>>,
    grid_id: [u8; 32],
    seeds: Vec<[u8; 32]>,
    streams: Streams,
    records: BTreeMap<u32, BatchRecord>,
    pending: BTreeSet<u32>,
    waiting_training: BTreeMap<u32, DiscreteGrid<f64>>,
    next_training: u32,
}
impl HavanaDiscreteSession {
    pub fn production(
        problem: IntegrationProblem,
        settings: HavanaDiscreteSettings,
    ) -> Result<Self> {
        Self::new(problem, settings, IntegrationStage::Production)
    }
    pub fn pilot(problem: IntegrationProblem, settings: HavanaDiscreteSettings) -> Result<Self> {
        Self::new(problem, settings, IntegrationStage::Pilot)
    }
    fn new(
        problem: IntegrationProblem,
        settings: HavanaDiscreteSettings,
        mut stage: IntegrationStage,
    ) -> Result<Self> {
        problem.validate()?;
        settings.validate()?;
        let grid = if problem.sectors.is_empty() {
            stage = IntegrationStage::Production;
            None
        } else {
            Some(settings.grid(&problem)?)
        };
        let mut session = Self {
            streams: Streams::new(settings.batch.seed),
            problem,
            settings,
            stage,
            epoch: 0,
            grid,
            training: None,
            grid_id: [0; 32],
            seeds: Vec::new(),
            records: BTreeMap::new(),
            pending: BTreeSet::new(),
            waiting_training: BTreeMap::new(),
            next_training: 0,
        };
        session.install()?;
        Ok(session)
    }
    fn proposal(&self) -> Vec<(f64, Vec<Vec<f64>>)> {
        self.grid
            .iter()
            .flat_map(|grid| &grid.bins)
            .map(|bin| {
                let Some(Grid::Continuous(grid)) = &bin.sub_grid else {
                    unreachable!("native continuous sector child")
                };
                (
                    bin.pdf,
                    grid.continuous_dimensions
                        .iter()
                        .map(|d| d.partitioning.clone())
                        .collect(),
                )
            })
            .collect()
    }
    fn identity(&self) -> Result<[u8; 32]> {
        Ok(*blake3::hash(&serde_json::to_vec(&(
            &self.problem,
            &self.settings,
            self.stage,
            self.epoch,
            self.proposal(),
        ))?)
        .as_bytes())
    }
    fn install(&mut self) -> Result<()> {
        self.grid_id = self.identity()?;
        self.training = self.grid.as_ref().map(clone_without_samples).transpose()?;
        let draws = self.batch_draws()?;
        let mut streams = self.streams.clone();
        self.seeds = (0..self.settings.batch.batches)
            .map(|_| streams.reserve(draws).map(|(_, state)| state))
            .collect::<Result<_>>()?;
        self.streams = streams;
        self.records.clear();
        self.pending.clear();
        self.waiting_training.clear();
        self.next_training = 0;
        Ok(())
    }
    pub fn problem(&self) -> &IntegrationProblem {
        &self.problem
    }
    pub fn settings(&self) -> &HavanaDiscreteSettings {
        &self.settings
    }
    pub fn stage(&self) -> IntegrationStage {
        self.stage
    }
    pub fn sector_probabilities(&self) -> Vec<(u64, f64)> {
        self.problem
            .sectors
            .iter()
            .zip(self.grid.iter().flat_map(|g| &g.bins))
            .map(|(s, b)| (s.id, b.pdf))
            .collect()
    }
    pub fn with_sector_probabilities(mut self, probabilities: &[f64]) -> Result<Self> {
        if !self.records.is_empty()
            || !self.pending.is_empty()
            || self.epoch != 0
            || probabilities.len() != self.problem.sectors.len()
            || probabilities.iter().any(|p| !p.is_finite() || *p <= 0.0)
        {
            return Err(IntegrationError::Invalid("initial probabilities require a fresh session and one finite positive probability per sector".into()));
        }
        let sum = crate::integration::estimate::precise_sum(probabilities.iter().copied())?;
        if !probabilities.is_empty() && (sum - 1.0).abs() > 16.0 * f64::EPSILON {
            return Err(IntegrationError::Invalid(
                "initial sector probabilities must sum to one".into(),
            ));
        }
        if let Some(grid) = &mut self.grid {
            for (bin, p) in grid.bins.iter_mut().zip(probabilities) {
                bin.pdf = *p / sum;
            }
        }
        self.grid_id = self.identity()?;
        self.training = self.grid.as_ref().map(clone_without_samples).transpose()?;
        Ok(self)
    }
    pub fn is_complete(&self) -> bool {
        self.grid.is_none() || self.records.len() == self.settings.batch.batches as usize
    }
    fn task(&self, batch: u32) -> HavanaDiscreteTask {
        HavanaDiscreteTask {
            batch,
            points: self.settings.batch.points_per_batch,
            grid_id: self.grid_id,
            rng_state: self.seeds[batch as usize],
        }
    }
    pub fn next_work(&mut self) -> Option<HavanaDiscreteTask> {
        self.grid.as_ref()?;
        for batch in 0..self.settings.batch.batches {
            if !self.records.contains_key(&batch) && self.pending.insert(batch) {
                return Some(self.task(batch));
            }
        }
        None
    }
    pub fn worker_context(&self) -> Result<HavanaDiscreteWorker> {
        Ok(HavanaDiscreteWorker {
            sector_ids: self.problem.sectors.iter().map(|s| s.id).collect(),
            grid_id: self.grid_id,
            grid: clone_without_samples(self.grid.as_ref().ok_or_else(|| {
                IntegrationError::Unavailable("exact-only problem has no sampling worker".into())
            })?)?,
            points: self.settings.batch.points_per_batch,
            outputs: self.problem.orders.len(),
            weight_layout: Some(WeightLayout::new(&self.problem)),
            training: self.stage == IntegrationStage::Pilot,
        })
    }
    fn validate_task(&self, task: &HavanaDiscreteTask) -> Result<()> {
        if task.batch >= self.settings.batch.batches || self.task(task.batch) != *task {
            return Err(IntegrationError::InvalidReturn(
                "discrete task belongs to another problem, grid or epoch".into(),
            ));
        }
        Ok(())
    }
    pub fn retry(&mut self, task: &HavanaDiscreteTask) -> Result<()> {
        self.validate_task(task)?;
        if !self.pending.remove(&task.batch) {
            return Err(IntegrationError::InvalidReturn(
                "discrete task is not in flight".into(),
            ));
        }
        Ok(())
    }
    fn validate_record(&self, record: &BatchRecord) -> Result<()> {
        self.validate_task(&record.task)?;
        if let Some(stats) = &record.weights {
            stats.validate(
                WeightLayout::new(&self.problem).len(),
                record.task.points as u64,
            )?;
        }
        let n = self.problem.orders.len();
        let sectors = self.problem.sectors.len();
        if record.mean.len() != n
            || record.sector_means.len() != sectors
            || record.counts.len() != sectors
            || record.sector_seconds.len() != sectors
            || record.sector_means.iter().any(|row| row.len() != n)
            || record
                .mean
                .iter()
                .chain(record.sector_means.iter().flatten())
                .any(|x| !x.is_finite())
            || record.worker_seconds < 0.0
            || !record.worker_seconds.is_finite()
            || record
                .sector_seconds
                .iter()
                .any(|x| !x.is_finite() || *x < 0.0)
            || record
                .counts
                .iter()
                .try_fold(0u64, |s, v| s.checked_add(*v))
                != Some(record.task.points as u64)
            || record
                .counts
                .iter()
                .zip(&record.sector_means)
                .any(|(count, row)| *count == 0 && row.iter().any(|x| *x != 0.0))
        {
            return Err(IntegrationError::InvalidReturn(
                "invalid discrete batch statistics or actual selection counts".into(),
            ));
        }
        Ok(())
    }
    pub fn submit(&mut self, value: HavanaDiscreteReturn) -> Result<()> {
        let record = BatchRecord {
            task: value.task,
            mean: value.mean,
            weights: value.weights,
            sector_means: value.sector_means,
            counts: value.counts,
            sector_seconds: value.sector_seconds,
            worker_seconds: value.worker_seconds,
        };
        self.validate_record(&record)?;
        let batch = record.task.batch;
        if !self.pending.contains(&batch)
            || value.training.is_some() != (self.stage == IntegrationStage::Pilot)
        {
            return Err(IntegrationError::InvalidReturn(
                "duplicate, unissued or wrong-phase discrete batch".into(),
            ));
        }
        if let Some(grid) = &value.training {
            self.training
                .as_ref()
                .unwrap()
                .is_mergeable(grid)
                .map_err(IntegrationError::InvalidReturn)?;
        }
        if let Some(grid) = value.training {
            self.waiting_training.insert(batch, grid);
        }
        while let Some(grid) = self.waiting_training.remove(&self.next_training) {
            self.training
                .as_mut()
                .unwrap()
                .merge(&grid)
                .map_err(IntegrationError::InvalidReturn)?;
            self.next_training += 1;
        }
        self.pending.remove(&batch);
        self.records.insert(batch, record);
        Ok(())
    }
    fn trained(&self, discrete: f64, continuous: f64) -> Result<DiscreteGrid<f64>> {
        if self.stage != IntegrationStage::Pilot
            || !self.is_complete()
            || !self.pending.is_empty()
            || [discrete, continuous]
                .iter()
                .any(|r| !r.is_finite() || *r < 0.0)
        {
            return Err(IntegrationError::Invalid(
                "adaptation requires a complete pilot and finite nonnegative rates".into(),
            ));
        }
        let mut grid = self.training.clone().ok_or_else(|| {
            IntegrationError::Unavailable("exact-only problem has no pilot".into())
        })?;
        grid.update(discrete, continuous);
        if grid.bins.iter().any(|b| !b.pdf.is_finite() || b.pdf <= 0.0) {
            return Err(IntegrationError::Invalid(
                "native adaptation produced an invalid sector probability".into(),
            ));
        }
        clone_without_samples(&grid)
    }
    pub fn adapt_pilot(&mut self, discrete: f64, continuous: f64) -> Result<()> {
        let grid = self.trained(discrete, continuous)?;
        let mut next = self.clone();
        next.grid = Some(grid);
        next.epoch = next
            .epoch
            .checked_add(1)
            .ok_or_else(|| IntegrationError::Invalid("pilot epoch overflow".into()))?;
        next.install()?;
        *self = next;
        Ok(())
    }
    pub fn freeze_production(
        &mut self,
        discrete: f64,
        continuous: f64,
        points_per_batch: usize,
        batches: u32,
    ) -> Result<()> {
        let grid = self.trained(discrete, continuous)?;
        let mut next = self.clone();
        next.settings.batch.points_per_batch = points_per_batch;
        next.settings.batch.batches = batches;
        next.settings.validate()?;
        next.grid = Some(grid);
        next.stage = IntegrationStage::Production;
        next.epoch = next
            .epoch
            .checked_add(1)
            .ok_or_else(|| IntegrationError::Invalid("production epoch overflow".into()))?;
        next.install()?;
        *self = next;
        Ok(())
    }
}
