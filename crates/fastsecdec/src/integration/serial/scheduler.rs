use super::{ReplicaMoments, state::*, streams::Streams};
use crate::integration::{IntegrationError, IntegrationProblem, Result};
use numerica::numerical_integration::ContinuousGrid;
use std::collections::BTreeMap;

impl SerialSession {
    pub fn new(problem: IntegrationProblem, settings: SerialSettings) -> Result<Self> {
        problem.validate()?;
        settings.method.validate()?;
        settings.target.validate_layout(&problem.orders)?;
        settings.tolerance.validate()?;
        if settings.max_rounds == Some(0)
            || settings.max_in_flight == 0
            || settings.pilot_iterations == 0
            || !settings.learning_rate.is_finite()
            || !(0.0..=1.0).contains(&settings.learning_rate)
        {
            return Err(IntegrationError::Invalid(
                "serial budgets, pilot count, learning rate and worker capacity are invalid".into(),
            ));
        }
        let sectors = problem
            .sectors
            .iter()
            .map(|s| {
                let grid = settings.method.grid(s.dimension)?;
                Ok(SectorState {
                    id: s.id,
                    dimension: s.dimension,
                    pilot: settings.method.pilot(),
                    pilot_iteration: 0,
                    epoch: 0,
                    round: 0,
                    points: settings.method.points(),
                    target: settings.method.replicas(),
                    next_replica: 0,
                    moments: ReplicaMoments::new(problem.orders.len())?,
                    weights: Some(crate::integration::weights::WeightStatistics::new(
                        crate::integration::weights::WeightLayout::new(&problem).len(),
                    )),
                    previous: None,
                    training: grid.as_ref().map(ContinuousGrid::clone_without_samples),
                    grid,
                    first_visit: false,
                    worker_seconds: 0.,
                    accepted_points: 0,
                })
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            streams: Streams::new(settings.method.seed()),
            problem,
            settings,
            sectors,
            pending: BTreeMap::new(),
            run: 0,
            next_lease: 0,
        })
    }
    pub fn problem(&self) -> &IntegrationProblem {
        &self.problem
    }
    pub fn settings(&self) -> &SerialSettings {
        &self.settings
    }
    pub fn set_max_in_flight(&mut self, limit: usize) -> Result<()> {
        if limit == 0 {
            return Err(IntegrationError::Invalid(
                "serial reservation capacity must be positive".into(),
            ));
        }
        self.settings.max_in_flight = limit;
        Ok(())
    }
    pub(super) fn index(&self, id: u64) -> Result<usize> {
        self.sectors
            .iter()
            .position(|s| s.id == id)
            .ok_or_else(|| IntegrationError::Invalid("unknown serial sector".into()))
    }
    pub(super) fn pending_count(&self, id: u64) -> usize {
        self.pending
            .values()
            .filter(|p| p.task.sector_id() == id)
            .count()
    }
    pub(super) fn exhausted_sector(&self, s: &SectorState) -> bool {
        !s.pilot
            && s.moments.count() >= s.target
            && self.settings.max_rounds.is_some_and(|n| s.round + 1 >= n)
            && self.pending_count(s.id) == 0
    }
    pub(super) fn available(&self, s: &SectorState) -> bool {
        if self
            .pending
            .values()
            .any(|p| p.task.sector_id() == s.id && !p.issued)
        {
            return true;
        }
        // After reducing worker capacity, restored unissued reservations can
        // fill the ledger. Dispatch those before selecting fresh high-priority
        // work that reserve() cannot admit yet.
        if self.pending.len() >= self.settings.max_in_flight {
            return false;
        }
        !self.exhausted_sector(s)
            && (s.moments.count() + (self.pending_count(s.id) as u64) < s.target
                || self.pending_count(s.id) == 0)
    }
    pub(super) fn advance(&self, s: &mut SectorState) -> Result<()> {
        if s.moments.count() < s.target {
            return Ok(());
        }
        if s.pilot {
            if let Some(training) = &s.training {
                let mut grid = training.clone();
                grid.update(self.settings.learning_rate);
                s.grid = Some(grid.clone_without_samples());
            }
            s.pilot_iteration = s.pilot_iteration.checked_add(1).ok_or_else(|| {
                IntegrationError::Invalid("serial pilot iteration overflow".into())
            })?;
            s.pilot = s.pilot_iteration < self.settings.pilot_iterations;
            s.training = if s.pilot {
                s.grid.as_ref().map(ContinuousGrid::clone_without_samples)
            } else {
                None
            };
            s.epoch = s
                .epoch
                .checked_add(1)
                .ok_or_else(|| IntegrationError::Invalid("serial epoch overflow".into()))?;
            s.moments = ReplicaMoments::new(self.problem.orders.len())?;
            s.weights = Some(crate::integration::weights::WeightStatistics::new(
                crate::integration::weights::WeightLayout::new(&self.problem).len(),
            ));
            s.next_replica = 0;
            return Ok(());
        }
        s.previous = Some(AcceptedAllocation {
            weights: s.weights.clone(),
            estimate: s.moments.estimate(&self.problem, true)?,
            moments: s.moments.clone(),
            replicas: s.moments.count(),
            points_per_replica: s.points,
            epoch: s.epoch,
            round: s.round,
        });
        s.round = s
            .round
            .checked_add(1)
            .ok_or_else(|| IntegrationError::Invalid("serial round overflow".into()))?;
        let points = self
            .settings
            .method
            .next_points(s.points, self.settings.double_points)?;
        if points == s.points {
            s.target = s.target.checked_mul(2).ok_or_else(|| {
                IntegrationError::Invalid("serial replica target overflow".into())
            })?;
        } else {
            s.points = points;
            s.target = self.settings.method.replicas();
            s.next_replica = 0;
            s.epoch = s
                .epoch
                .checked_add(1)
                .ok_or_else(|| IntegrationError::Invalid("serial epoch overflow".into()))?;
            s.moments = ReplicaMoments::new(self.problem.orders.len())?;
            s.weights = Some(crate::integration::weights::WeightStatistics::new(
                crate::integration::weights::WeightLayout::new(&self.problem).len(),
            ));
            s.pilot = self.settings.method.pilot();
            s.pilot_iteration = 0;
            s.training = if s.pilot {
                s.grid.as_ref().map(ContinuousGrid::clone_without_samples)
            } else {
                None
            };
        }
        Ok(())
    }
    /// Call only at a whole-replica boundary. The caller measures sampling
    /// residence (excluding load/JIT and pauses) and owns the process lifecycle.
    pub fn choose_sector(
        &mut self,
        resident: Option<u64>,
        residence_seconds: f64,
        minimum_seconds: f64,
        occupied: &[u64],
    ) -> Result<Option<u64>> {
        if !residence_seconds.is_finite()
            || residence_seconds < 0.
            || !minimum_seconds.is_finite()
            || minimum_seconds <= 0.
        {
            return Err(IntegrationError::Invalid(
                "serial residence durations must be finite and positive".into(),
            ));
        }
        if self.meets_target()? {
            return Ok(None);
        }
        if let Some(id) = resident {
            let i = self.index(id)?;
            let valid = self.sectors[i].previous.is_some()
                || (!self.sectors[i].pilot && self.sectors[i].moments.count() >= 2);
            if valid
                && (residence_seconds >= minimum_seconds || self.exhausted_sector(&self.sectors[i]))
            {
                self.sectors[i].first_visit = true;
            }
            // An allocation may be fully reserved by other resident workers.
            // Wait for its final replica instead of evicting a loaded sector
            // before its minimum sampling residence has elapsed.
            if !self.exhausted_sector(&self.sectors[i])
                && (residence_seconds < minimum_seconds || !valid)
            {
                return Ok(Some(id));
            }
        }
        if !self.first_coverage_complete() {
            if let Some(s) = self.sectors.iter().find(|s| {
                !s.first_visit
                    && !occupied.contains(&s.id)
                    && !self
                        .pending
                        .values()
                        .any(|pending| pending.issued && pending.task.sector_id() == s.id)
                    && Some(s.id) != resident
                    && self.available(s)
            }) {
                return Ok(Some(s.id));
            }
            return Ok(resident.filter(|&id| {
                self.sectors
                    .iter()
                    .find(|s| s.id == id)
                    .is_some_and(|s| self.available(s))
            }));
        }
        let total = match self.estimate() {
            Ok(v) => Some(v),
            Err(IntegrationError::Unavailable(_)) => None,
            Err(e) => return Err(e),
        };
        let mut best: Option<(u64, f64)> = None;
        for s in &self.sectors {
            if !self.available(s) {
                continue;
            }
            let p = self.priority(s, total.as_ref())?;
            if best.is_none_or(|(id, q)| {
                p > q || (p == q && (Some(s.id) == resident || (Some(id) != resident && s.id < id)))
            }) {
                best = Some((s.id, p));
            }
        }
        Ok(best.map(|(id, _)| id))
    }
}
