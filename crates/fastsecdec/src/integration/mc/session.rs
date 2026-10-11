mod checkpoint;
mod weights;
use crate::integration::weights::{WeightLayout, WeightStatistics};
mod contributions;
mod observation;
mod refinement;
mod results;

use std::collections::{BTreeMap, BTreeSet};

use numerica::numerical_integration::{ContinuousGrid, Sample};
use serde::{Deserialize, Serialize};

use crate::{
    integration::{
        IntegrationError, IntegrationProblem, Result,
        streams::{Streams, mc_draws},
    },
    status::IntegrationStage,
};

use super::{HavanaReturn, HavanaSettings, HavanaTask, HavanaWorker};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct BatchRecord {
    task: HavanaTask,
    mean: Vec<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    weights: Option<WeightStatistics>,
    worker_seconds: f64,
}

#[derive(Clone, Debug)]
struct SectorRun {
    grid: ContinuousGrid<f64>,
    grid_id: [u8; 32],
    seeds: Vec<[u8; 32]>,
    records: BTreeMap<u32, BatchRecord>,
    pending: BTreeSet<u32>,
    training: ContinuousGrid<f64>,
    waiting_training: BTreeMap<u32, ContinuousGrid<f64>>,
    next_training: u32,
}

#[derive(Clone, Debug)]
pub struct HavanaSession {
    problem: IntegrationProblem,
    settings: HavanaSettings,
    stage: IntegrationStage,
    epoch: u64,
    runs: Vec<SectorRun>,
    streams: Streams,
    cursor: usize,
}

impl HavanaSession {
    pub fn production(problem: IntegrationProblem, settings: HavanaSettings) -> Result<Self> {
        Self::new(problem, settings, IntegrationStage::Production)
    }
    pub fn pilot(problem: IntegrationProblem, settings: HavanaSettings) -> Result<Self> {
        Self::new(problem, settings, IntegrationStage::Pilot)
    }

    fn new(
        problem: IntegrationProblem,
        settings: HavanaSettings,
        mut stage: IntegrationStage,
    ) -> Result<Self> {
        problem.validate()?;
        settings.validate()?;
        if problem.sectors.is_empty() {
            stage = IntegrationStage::Production;
        }
        let grids = problem
            .sectors
            .iter()
            .map(|s| settings.grid(s.dimension))
            .collect::<Result<Vec<_>>>()?;
        let mut session = Self {
            streams: Streams::new(settings.seed),
            problem,
            settings,
            stage,
            epoch: 0,
            runs: Vec::new(),
            cursor: 0,
        };
        session.install_grids(grids)?;
        Ok(session)
    }

    fn grid_id(&self, sector: usize, grid: &ContinuousGrid<f64>) -> Result<[u8; 32]> {
        let partitions: Vec<_> = grid
            .continuous_dimensions
            .iter()
            .map(|d| &d.partitioning)
            .collect();
        Ok(*blake3::hash(&serde_json::to_vec(&(
            &self.problem,
            &self.settings,
            self.stage,
            self.epoch,
            sector,
            partitions,
        ))?)
        .as_bytes())
    }

    fn install_grids(&mut self, grids: Vec<ContinuousGrid<f64>>) -> Result<()> {
        let mut streams = self.streams.clone();
        let mut runs = Vec::new();
        for (i, grid) in grids.into_iter().enumerate() {
            let draws = mc_draws(
                self.settings.points_per_batch as u64,
                self.problem.sectors[i].dimension,
            )?;
            let seeds = (0..self.settings.batches)
                .map(|_| streams.reserve(draws).map(|(_, state)| state))
                .collect::<Result<_>>()?;
            runs.push(SectorRun {
                grid_id: self.grid_id(i, &grid)?,
                training: grid.clone_without_samples(),
                grid,
                seeds,
                records: BTreeMap::new(),
                pending: BTreeSet::new(),
                waiting_training: BTreeMap::new(),
                next_training: 0,
            });
        }
        self.runs = runs;
        self.streams = streams;
        self.cursor = 0;
        Ok(())
    }

    pub fn problem(&self) -> &IntegrationProblem {
        &self.problem
    }
    pub fn stage(&self) -> IntegrationStage {
        self.stage
    }
    pub fn settings(&self) -> &HavanaSettings {
        &self.settings
    }
    pub fn is_complete(&self) -> bool {
        self.runs
            .iter()
            .all(|r| r.records.len() == self.settings.batches as usize)
    }

    fn index(&self, id: u64) -> Result<usize> {
        self.problem
            .sectors
            .iter()
            .position(|s| s.id == id)
            .ok_or_else(|| IntegrationError::InvalidReturn("unknown Havana sector".into()))
    }

    fn task(&self, i: usize, batch: u32) -> HavanaTask {
        HavanaTask {
            sector_id: self.problem.sectors[i].id,
            batch,
            points: self.settings.points_per_batch,
            grid_id: self.runs[i].grid_id,
            rng_state: self.runs[i].seeds[batch as usize],
        }
    }

    pub fn next_work(&mut self) -> Option<HavanaTask> {
        for _ in 0..self.runs.len() {
            let i = self.cursor;
            self.cursor = (i + 1) % self.runs.len();
            for batch in 0..self.settings.batches {
                if !self.runs[i].records.contains_key(&batch) && self.runs[i].pending.insert(batch)
                {
                    return Some(self.task(i, batch));
                }
            }
        }
        None
    }

    pub fn worker_context(&self, sector_id: u64) -> Result<HavanaWorker> {
        let i = self.index(sector_id)?;
        Ok(HavanaWorker {
            sector_id,
            grid_id: self.runs[i].grid_id,
            grid: self.runs[i].grid.clone_without_samples(),
            points: self.settings.points_per_batch,
            outputs: self.problem.orders.len(),
            weight_layout: Some(WeightLayout::new(&self.problem)),
            training: self.stage == IntegrationStage::Pilot,
            sample: Sample::new(),
            values: Vec::new(),
        })
    }

    fn validate_task(&self, task: &HavanaTask) -> Result<usize> {
        let i = self.index(task.sector_id)?;
        if task.batch >= self.settings.batches || self.task(i, task.batch) != *task {
            return Err(IntegrationError::InvalidReturn(
                "Havana task belongs to another integral, grid or epoch".into(),
            ));
        }
        Ok(i)
    }

    pub fn retry(&mut self, task: &HavanaTask) -> Result<()> {
        let i = self.validate_task(task)?;
        if !self.runs[i].pending.remove(&task.batch) {
            return Err(IntegrationError::InvalidReturn(
                "Havana task is not in flight".into(),
            ));
        }
        Ok(())
    }

    pub fn submit(&mut self, value: HavanaReturn) -> Result<()> {
        let i = self.validate_task(&value.task)?;
        let run = &mut self.runs[i];
        let batch = value.task.batch;
        if !run.pending.contains(&batch)
            || value.mean.len() != self.problem.orders.len()
            || value.mean.iter().any(|v| !v.is_finite())
            || !value.worker_seconds.is_finite()
            || value.worker_seconds < 0.0
            || value.training.is_some() != (self.stage == IntegrationStage::Pilot)
        {
            return Err(IntegrationError::InvalidReturn(
                "invalid, duplicate or incomplete Havana return".into(),
            ));
        }
        if let Some(stats) = &value.weights {
            stats.validate(
                WeightLayout::new(&self.problem).len(),
                value.task.points as u64,
            )?;
        }
        if let Some(grid) = &value.training {
            run.training
                .is_mergeable(grid)
                .map_err(IntegrationError::InvalidReturn)?;
        }
        if let Some(grid) = value.training {
            run.waiting_training.insert(batch, grid);
        }
        while let Some(grid) = run.waiting_training.remove(&run.next_training) {
            run.training
                .merge(&grid)
                .map_err(IntegrationError::InvalidReturn)?;
            run.next_training += 1;
        }
        run.records.insert(
            batch,
            BatchRecord {
                task: value.task,
                mean: value.mean,
                weights: value.weights,
                worker_seconds: value.worker_seconds,
            },
        );
        run.pending.remove(&batch);
        Ok(())
    }

    /// Adapt from complete pilot batches and run a fresh independent pilot.
    pub fn adapt_pilot(&mut self, learning_rate: f64) -> Result<()> {
        self.advance(
            learning_rate,
            self.settings.points_per_batch,
            self.settings.batches,
            IntegrationStage::Pilot,
        )
    }

    /// Freeze the trained grids before production. Pilot observations never
    /// contribute to the reported production mean or covariance.
    pub fn freeze_production(
        &mut self,
        learning_rate: f64,
        points_per_batch: usize,
        batches: u32,
    ) -> Result<()> {
        self.advance(
            learning_rate,
            points_per_batch,
            batches,
            IntegrationStage::Production,
        )
    }

    fn advance(
        &mut self,
        learning_rate: f64,
        points: usize,
        batches: u32,
        stage: IntegrationStage,
    ) -> Result<()> {
        if self.stage != IntegrationStage::Pilot
            || !self.is_complete()
            || !learning_rate.is_finite()
            || !(0.0..=1.0).contains(&learning_rate)
        {
            return Err(IntegrationError::Invalid(
                "grid adaptation requires a complete pilot and a learning rate in [0,1]".into(),
            ));
        }
        let mut next = Self {
            problem: self.problem.clone(),
            settings: self.settings.clone(),
            stage,
            epoch: self.epoch,
            runs: Vec::new(),
            streams: self.streams.clone(),
            cursor: 0,
        };
        next.settings.points_per_batch = points;
        next.settings.batches = batches;
        next.settings.validate()?;
        next.stage = stage;
        next.epoch = next
            .epoch
            .checked_add(1)
            .ok_or_else(|| IntegrationError::Invalid("Havana epoch overflow".into()))?;
        let grids = self
            .runs
            .iter()
            .map(|run| {
                let mut grid = run.training.clone();
                grid.update(learning_rate);
                grid.clone_without_samples()
            })
            .collect();
        next.install_grids(grids)?;
        *self = next;
        Ok(())
    }
}
