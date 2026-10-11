mod checkpoint;
mod design;
mod refinement;
mod results;
mod weights;

use std::collections::{BTreeMap, BTreeSet};

use fastsecdec_qmc::QmcAccumulator;
use serde::{Deserialize, Serialize};

use crate::status::{IntegrationMethod, IntegrationStage};

use super::{
    IntegrationError, IntegrationProblem, QmcReturn, QmcSettings, QmcTask, QmcWorker, Result,
    estimate::precise_sum,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductionAllocation {
    pub sector_id: u64,
    pub points: u64,
    pub shifts: u32,
}

/// Effective numerical design, requested explicitly for reporting or saving.
///
/// Settings identify the rule, seed and transform. Allocations are authoritative
/// for the current stage's points and shifts in each sector; adaptive production
/// may differ from the base settings. This is not an accumulation checkpoint.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QmcDesign {
    pub settings: QmcSettings,
    pub allocations: Vec<ProductionAllocation>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SectorRun {
    pub(crate) accumulator: QmcAccumulator,
    pub(crate) costs: BTreeMap<u64, f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    weights: Option<BTreeMap<u64, crate::integration::weights::WeightPartial>>,
    #[serde(skip)]
    pending: BTreeSet<u64>,
    /// Complete-allocation ends preserve the canonical tail package when more
    /// shifts are appended and package size does not divide the lattice size.
    #[serde(default)]
    package_boundaries: Vec<u64>,
}

impl SectorRun {
    pub(crate) fn seconds(&self) -> Result<f64> {
        precise_sum(self.costs.values().copied())
    }
}

/// A fixed production design or a separate adaptive pilot. Request work,
/// evaluate it with caller-owned workers, submit returns, then inspect results.
/// Statistical stages are never blended automatically.
#[derive(Clone, Debug)]
pub struct QmcSession {
    pub(crate) problem: IntegrationProblem,
    pub(crate) settings: QmcSettings,
    pub(crate) method: IntegrationMethod,
    pub(crate) stage: IntegrationStage,
    pub(crate) epoch: u64,
    pub(crate) allocations: Vec<ProductionAllocation>,
    pub(crate) runs: Vec<SectorRun>,
    pub(crate) cursor: usize,
}

impl QmcSession {
    /// Statistical method whose stream and coverage semantics this session uses.
    pub fn method(&self) -> IntegrationMethod {
        self.method
    }

    /// Copy the current design for a final report without embedding potentially
    /// large caller-supplied generating vectors in every progress snapshot.
    pub fn design(&self) -> QmcDesign {
        QmcDesign {
            settings: self.settings.clone(),
            allocations: self.allocations.clone(),
        }
    }

    pub fn democratic(problem: IntegrationProblem, settings: QmcSettings) -> Result<Self> {
        Self::new(
            problem,
            settings,
            IntegrationMethod::DemocraticQmc,
            IntegrationStage::Production,
        )
    }

    pub fn adaptive(problem: IntegrationProblem, settings: QmcSettings) -> Result<Self> {
        Self::new(
            problem,
            settings,
            IntegrationMethod::AdaptiveQmc,
            IntegrationStage::Pilot,
        )
    }

    fn new(
        problem: IntegrationProblem,
        settings: QmcSettings,
        method: IntegrationMethod,
        mut stage: IntegrationStage,
    ) -> Result<Self> {
        problem.validate()?;
        settings.validate()?;
        if problem.sectors.is_empty() {
            stage = IntegrationStage::Production;
        }
        let allocations = problem
            .sectors
            .iter()
            .map(|sector| ProductionAllocation {
                sector_id: sector.id,
                points: settings.points,
                shifts: settings.shifts,
            })
            .collect();
        let mut session = Self {
            problem,
            settings,
            method,
            stage,
            epoch: 0,
            allocations,
            runs: Vec::new(),
            cursor: 0,
        };
        session.runs = session.make_runs()?;
        Ok(session)
    }

    fn stream(&self, index: usize) -> u64 {
        match (self.method, self.stage) {
            (IntegrationMethod::DemocraticQmc, _) => 0xD000_0000_0000_0000,
            (_, IntegrationStage::Pilot) => 0xA000_0000_0000_0000 | index as u64,
            (_, IntegrationStage::Production) => 0xB000_0000_0000_0000 | index as u64,
        }
    }

    pub(crate) fn make_runs(&self) -> Result<Vec<SectorRun>> {
        if self.allocations.len() != self.problem.sectors.len() {
            return Err(IntegrationError::Invalid(
                "allocation must cover every sector exactly once".into(),
            ));
        }
        self.problem
            .sectors
            .iter()
            .zip(&self.allocations)
            .enumerate()
            .map(|(i, (sector, allocation))| {
                if sector.id != allocation.sector_id {
                    return Err(IntegrationError::Invalid(
                        "allocation sector identities or order differ".into(),
                    ));
                }
                let plan = self.settings.plan(
                    sector.dimension,
                    allocation.points,
                    allocation.shifts,
                    self.stream(i),
                )?;
                Ok(SectorRun {
                    package_boundaries: vec![0, plan.total_points()],
                    accumulator: QmcAccumulator::new(plan, self.problem.orders.len())?,
                    costs: BTreeMap::new(),
                    weights: Some(BTreeMap::new()),
                    pending: BTreeSet::new(),
                })
            })
            .collect()
    }

    pub fn problem(&self) -> &IntegrationProblem {
        &self.problem
    }
    pub fn stage(&self) -> IntegrationStage {
        self.stage
    }
    pub fn is_complete(&self) -> bool {
        self.runs.iter().all(|run| run.accumulator.is_complete())
    }

    /// Canonical packages are independent of worker count and arrival order.
    /// `None` means all remaining work is in flight or the allocation is done.
    pub fn next_work(&mut self) -> Result<Option<QmcTask>> {
        for _ in 0..self.runs.len() {
            let i = self.cursor % self.runs.len();
            self.cursor = (i + 1) % self.runs.len();
            let run = &mut self.runs[i];
            for missing in run.accumulator.missing_ranges() {
                let mut start = missing.start;
                while start < missing.end {
                    let work = run.canonical_work(start, self.settings.package_points)?;
                    let count = work.point_count();
                    if run.pending.insert(start) {
                        return Ok(Some(QmcTask {
                            content_id: self.problem.content_id.clone(),
                            epoch: self.epoch,
                            sector_id: self.problem.sectors[i].id,
                            periodization: self.settings.periodization,
                            work,
                        }));
                    }
                    start += count;
                }
            }
        }
        Ok(None)
    }

    fn sector_index(&self, id: u64) -> Result<usize> {
        self.problem
            .sectors
            .iter()
            .position(|s| s.id == id)
            .ok_or_else(|| IntegrationError::InvalidReturn("unknown sector identity".into()))
    }

    pub fn worker_context(&self, sector_id: u64) -> Result<QmcWorker> {
        let i = self.sector_index(sector_id)?;
        Ok(QmcWorker {
            content_id: self.problem.content_id.clone(),
            epoch: self.epoch,
            sector_id,
            plan: self.runs[i].accumulator.plan().clone(),
            output_count: self.problem.orders.len(),
            weight_layout: Some(crate::integration::weights::WeightLayout::new(
                &self.problem,
            )),
            periodization: self.settings.periodization,
            point: Vec::new(),
            values: Vec::new(),
        })
    }

    /// Release a failed or abandoned task for identical reissue.
    pub fn retry(&mut self, task: &QmcTask) -> Result<()> {
        let i = self.validate_task(task)?;
        if !self.runs[i].pending.remove(&task.work.start()) {
            return Err(IntegrationError::InvalidReturn(
                "task is not in flight".into(),
            ));
        }
        Ok(())
    }

    fn validate_task(&self, task: &QmcTask) -> Result<usize> {
        if task.content_id != self.problem.content_id
            || task.epoch != self.epoch
            || task.periodization != self.settings.periodization
        {
            return Err(IntegrationError::InvalidReturn(
                "task belongs to another integral or statistical stage".into(),
            ));
        }
        let i = self.sector_index(task.sector_id)?;
        if self.runs[i].canonical_work(task.work.start(), self.settings.package_points)?
            != task.work
        {
            return Err(IntegrationError::InvalidReturn(
                "task is not a canonical package of this allocation".into(),
            ));
        }
        Ok(i)
    }

    /// Invalid returns leave the session unchanged; accepted returns cannot be
    /// accepted again. Cross-integral and pilot/production mixing are rejected.
    pub fn submit(&mut self, value: QmcReturn) -> Result<()> {
        let i = self.validate_task(&value.task)?;
        let run = &mut self.runs[i];
        let start = value.task.work.start();
        if !run.pending.contains(&start)
            || value.partial.work() != value.task.work
            || !value.worker_seconds.is_finite()
            || value.worker_seconds < 0.0
        {
            return Err(IntegrationError::InvalidReturn(
                "unissued, duplicate, mistimed or mismatched return".into(),
            ));
        }
        precise_sum(run.costs.values().copied().chain([value.worker_seconds]))?;
        if let Some(stats) = &value.weights {
            stats.validate(
                start,
                value.task.point_count(),
                run.accumulator.plan().rule().points(),
                crate::integration::weights::WeightLayout::new(&self.problem).len(),
            )?;
        }
        run.accumulator.merge(value.partial)?;
        match (&mut run.weights, value.weights) {
            (Some(rows), Some(stats)) => {
                rows.insert(start, stats);
            }
            (_, None) => run.weights = None,
            (None, Some(_)) => {}
        }
        run.pending.remove(&start);
        run.costs.insert(start, value.worker_seconds);
        Ok(())
    }
}
