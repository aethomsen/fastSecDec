use super::SerialSession;
use crate::integration::{IntegrationError, IntegrationProblem, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Envelope {
    version: u32,
    digest: [u8; 32],
    session: SerialSession,
}
impl SerialSession {
    /// Checkpoint only after the caller's replay and diagnostic transaction is
    /// prepared; store all three together atomically in the caller's envelope.
    pub fn checkpoint(&self) -> Result<Vec<u8>> {
        let digest = *blake3::hash(&serde_json::to_vec(self)?).as_bytes();
        Ok(serde_json::to_vec(&Envelope {
            version: 1,
            digest,
            session: self.clone(),
        })?)
    }
    /// Call only after all previous workers are dead. Accepted work survives;
    /// each abandoned reservation is reissued with identical coordinates and a
    /// different run/lease identity so delayed messages cannot be admitted.
    pub fn restore(bytes: &[u8], problem: &IntegrationProblem) -> Result<Self> {
        let envelope: Envelope = serde_json::from_slice(bytes)?;
        if envelope.version != 1
            || *blake3::hash(&serde_json::to_vec(&envelope.session)?).as_bytes() != envelope.digest
        {
            return Err(IntegrationError::Invalid(
                "serial checkpoint version or digest differs".into(),
            ));
        }
        let mut session = envelope.session;
        problem.validate()?;
        session.settings.method.validate()?;
        session.settings.tolerance.validate()?;
        session.settings.target.validate_layout(&problem.orders)?;
        if &session.problem != problem
            || session.sectors.len() != problem.sectors.len()
            || session.streams.state == [0; 32]
            || session.settings.max_in_flight == 0
            || session.settings.max_rounds == Some(0)
            || session.settings.pilot_iterations == 0
            || !session.settings.learning_rate.is_finite()
            || !(0.0..=1.0).contains(&session.settings.learning_rate)
        {
            return Err(IntegrationError::Invalid(
                "serial checkpoint integral identity or settings differ".into(),
            ));
        }
        for (s, spec) in session.sectors.iter().zip(&problem.sectors) {
            s.moments.validate(problem.orders.len())?;
            if let Some(stats) = &s.weights {
                stats.validate(
                    crate::integration::weights::WeightLayout::new(problem).len(),
                    s.moments
                        .count()
                        .checked_mul(s.points)
                        .ok_or(IntegrationError::NumericRange)?,
                )?;
            }
            if s.id != spec.id
                || s.dimension != spec.dimension
                || s.points < 2
                || s.target < 2
                || s.moments.count() > s.target
                || s.moments.count() > s.next_replica
                || !s.worker_seconds.is_finite()
                || s.worker_seconds < 0.
                || (s.pilot && !session.settings.method.pilot())
                || s.next_replica > s.target
                || s.moments.count().checked_add(
                    session
                        .pending
                        .values()
                        .filter(|p| p.task.sector_id() == s.id)
                        .count() as u64,
                ) != Some(s.next_replica)
                || s.pilot_iteration > session.settings.pilot_iterations
                || (s.pilot && s.pilot_iteration >= session.settings.pilot_iterations)
            {
                return Err(IntegrationError::Invalid(
                    "invalid serial checkpoint sector state".into(),
                ));
            }
            if let Some(previous) = &s.previous {
                previous.estimate.validate()?;
                previous.moments.validate(problem.orders.len())?;
                if let Some(stats) = &previous.weights {
                    stats.validate(
                        crate::integration::weights::WeightLayout::new(problem).len(),
                        previous
                            .replicas
                            .checked_mul(previous.points_per_replica)
                            .ok_or(IntegrationError::NumericRange)?,
                    )?;
                }
                if previous.estimate.orders != problem.orders
                    || previous.estimate.components != problem.components
                    || !previous.estimate.production_complete
                    || previous.replicas < 2
                    || previous.points_per_replica < 2
                    || previous.replicas != previous.moments.count()
                    || previous.estimate != previous.moments.estimate(problem, true)?
                {
                    return Err(IntegrationError::Invalid(
                        "invalid previous serial production estimate".into(),
                    ));
                }
            }
        }
        let mut identities = std::collections::BTreeSet::new();
        for (&stream, p) in &session.pending {
            p.task.validate()?;
            if p.task
                .weight_layout
                .as_ref()
                .is_some_and(|l| *l != crate::integration::weights::WeightLayout::new(problem))
            {
                return Err(IntegrationError::Invalid(
                    "restored weight layout differs".into(),
                ));
            }
            let task = &p.task;
            let id = &task.identity;
            let s = session
                .sectors
                .iter()
                .find(|s| s.id == id.sector_id)
                .ok_or_else(|| {
                    IntegrationError::Invalid("pending serial sector is absent".into())
                })?;
            if stream != id.stream
                || stream >= session.streams.next
                || id.content_id != problem.content_id
                || id.epoch != s.epoch
                || id.pilot != s.pilot
                || id.replica >= s.next_replica
                || id.lease >= session.next_lease
                || id.run != session.run
                || task.dimension() != s.dimension
                || task.point_count() != s.points
                || task.output_count() != problem.orders.len()
                || !identities.insert((id.sector_id, id.epoch, id.replica))
            {
                return Err(IntegrationError::Invalid(
                    "invalid pending serial reservation".into(),
                ));
            }
        }
        session.run = session.run.checked_add(1).ok_or_else(|| {
            IntegrationError::Invalid("serial recovery run counter exhausted".into())
        })?;
        for pending in session.pending.values_mut() {
            pending.issued = false;
            pending.task.identity.run = session.run;
        }
        Ok(session)
    }
}
