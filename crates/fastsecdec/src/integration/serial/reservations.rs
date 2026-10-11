use super::{ReplicaIdentity, SerialReturn, SerialTask, job::Sampler, state::*, streams::mc_draws};
use crate::integration::{IntegrationError, Result, estimate::precise_sum};

impl SerialSession {
    /// Reserve one whole statistical replica. None means the selected sector is
    /// exhausted, its allocation is fully reserved, or the caller capacity is full.
    pub fn reserve(&mut self, id: u64) -> Result<Option<SerialTask>> {
        let i = self.index(id)?;
        if self.pending.values().filter(|p| p.issued).count() >= self.settings.max_in_flight {
            return Ok(None);
        }
        let lease = self
            .next_lease
            .checked_add(1)
            .ok_or_else(|| IntegrationError::Invalid("serial lease counter exhausted".into()))?;
        if let Some(p) = self
            .pending
            .values_mut()
            .find(|p| p.task.sector_id() == id && !p.issued)
        {
            p.issued = true;
            p.task.identity.lease = self.next_lease;
            p.task.identity.run = self.run;
            self.next_lease = lease;
            return Ok(Some(p.task.clone()));
        }
        if self.pending.len() >= self.settings.max_in_flight || !self.available(&self.sectors[i]) {
            return Ok(None);
        }
        let mut sector = self.sectors[i].clone();
        self.advance(&mut sector)?;
        if sector.moments.count() + self.pending_count(id) as u64 >= sector.target {
            return Ok(None);
        }
        let draws = match self.settings.method {
            SerialMethod::Qmc(_) | SerialMethod::AdaptiveQmc(_) => u64::try_from(sector.dimension)
                .map_err(|_| {
                    IntegrationError::Invalid("QMC dimension exceeds RNG draw budget".into())
                })?,
            _ => mc_draws(sector.points, sector.dimension)?,
        };
        let mut streams = self.streams.clone();
        let (stream, rng_state) = streams.reserve(draws)?;
        let replica = sector.next_replica;
        sector.next_replica = replica
            .checked_add(1)
            .ok_or_else(|| IntegrationError::Invalid("serial replica identity exhausted".into()))?;
        let sampler = match &self.settings.method {
            SerialMethod::Qmc(settings) | SerialMethod::AdaptiveQmc(settings) => {
                SerialTask::qmc_sampler(settings, sector.dimension, sector.points, rng_state)?
            }
            _ => Sampler::Mc {
                grid: Box::new(sector.grid.as_ref().unwrap().clone_without_samples()),
                training: sector.pilot,
            },
        };
        let task = SerialTask {
            identity: ReplicaIdentity {
                content_id: self.problem.content_id.clone(),
                sector_id: id,
                epoch: sector.epoch,
                pilot: sector.pilot,
                replica,
                stream,
                run: self.run,
                lease: self.next_lease,
            },
            sampler,
            rng_state,
            points: sector.points,
            dimension: sector.dimension,
            outputs: self.problem.orders.len(),
            weight_layout: Some(crate::integration::weights::WeightLayout::new(
                &self.problem,
            )),
        };
        task.validate()?;
        self.sectors[i] = sector;
        self.streams = streams;
        self.next_lease = lease;
        self.pending.insert(
            stream,
            Pending {
                task: task.clone(),
                issued: true,
            },
        );
        Ok(Some(task))
    }
    /// Only call after the previous worker is confirmed dead. Retry retains its
    /// random stream and replica identity but receives a fresh fenced lease.
    pub fn release(&mut self, task: &SerialTask) -> Result<()> {
        let p = self
            .pending
            .get_mut(&task.identity.stream)
            .ok_or_else(|| IntegrationError::InvalidReturn("unknown serial reservation".into()))?;
        if !p.issued || p.task.identity != task.identity {
            return Err(IntegrationError::InvalidReturn(
                "stale serial reservation lease".into(),
            ));
        }
        p.issued = false;
        Ok(())
    }
    /// Validate the complete return without changing state. Callers validate
    /// replay/diagnostics first, submit, then commit their infallible prepared
    /// metadata updates and checkpoint them together with this session.
    pub fn validate_return(&self, value: &SerialReturn) -> Result<()> {
        let p = self.pending.get(&value.identity.stream).ok_or_else(|| {
            IntegrationError::InvalidReturn("duplicate or unissued serial return".into())
        })?;
        if !p.issued
            || p.task.identity != value.identity
            || p.task.digest()? != value.task_digest
            || value.mean.len() != self.problem.orders.len()
            || value.mean.iter().any(|v| !v.is_finite())
            || !value.worker_seconds.is_finite()
            || value.worker_seconds < 0.
        {
            return Err(IntegrationError::InvalidReturn(
                "serial return identity, lease, shape or timing differs".into(),
            ));
        }
        if let Some(stats) = &value.weights {
            if p.task.weight_layout.is_none() {
                return Err(IntegrationError::InvalidReturn(
                    "unrequested weight payload".into(),
                ));
            }
            stats.validate(
                crate::integration::weights::WeightLayout::new(&self.problem).len(),
                p.task.points,
            )?;
        }
        let expects = matches!(p.task.sampler, Sampler::Mc { training: true, .. });
        if expects != value.training.is_some() {
            return Err(IntegrationError::InvalidReturn(
                "serial training payload differs from reserved phase".into(),
            ));
        }
        Ok(())
    }
    pub fn submit(&mut self, value: SerialReturn) -> Result<()> {
        self.validate_return(&value)?;
        let i = self.index(value.identity.sector_id)?;
        let mut sector = self.sectors[i].clone();
        if sector.epoch != value.identity.epoch || sector.pilot != value.identity.pilot {
            return Err(IntegrationError::InvalidReturn(
                "serial return statistical epoch differs".into(),
            ));
        }
        sector.moments.add(&value.mean)?;
        match (&mut sector.weights, &value.weights) {
            (Some(all), Some(row)) => all.merge(row)?,
            (_, None) => sector.weights = None,
            (None, Some(_)) => {}
        }
        if let Some(training) = value.training {
            sector
                .training
                .as_mut()
                .unwrap()
                .merge(&training)
                .map_err(IntegrationError::InvalidReturn)?;
        }
        sector.worker_seconds = precise_sum([sector.worker_seconds, value.worker_seconds])?;
        sector.accepted_points = sector
            .accepted_points
            .checked_add(sector.points)
            .ok_or_else(|| {
                IntegrationError::Invalid("serial accepted point count overflow".into())
            })?;
        self.sectors[i] = sector;
        self.pending.remove(&value.identity.stream);
        if self.exhausted_sector(&self.sectors[i]) && self.sectors[i].moments.count() >= 2 {
            self.sectors[i].first_visit = true;
        }
        Ok(())
    }
}
