use super::*;

#[derive(Serialize, Deserialize)]
struct Checkpoint {
    version: u8,
    problem: IntegrationProblem,
    settings: HavanaSettings,
    epoch: u64,
    streams: Streams,
    sectors: Vec<SectorState>,
    integrity: [u8; 32],
}
impl Checkpoint {
    fn integrity(&self) -> Result<[u8; 32]> {
        Ok(*blake3::hash(&serde_json::to_vec(&(
            self.version,
            &self.problem,
            &self.settings,
            self.epoch,
            &self.streams,
            &self.sectors,
        ))?)
        .as_bytes())
    }
}

#[derive(Serialize, Deserialize)]
struct SectorState {
    partitions: Vec<Vec<f64>>,
    seeds: Vec<[u8; 32]>,
    records: Vec<BatchRecord>,
}

impl HavanaSession {
    /// Production checkpoints contain validated grid partitions and batch
    /// summaries, never Havana's private mutable adaptation internals. Complete
    /// or restart a pilot before taking a resumable production checkpoint.
    pub fn checkpoint(&self) -> Result<Vec<u8>> {
        if self.stage != IntegrationStage::Production {
            return Err(IntegrationError::Unavailable(
                "Havana checkpoints require a frozen production grid".into(),
            ));
        }
        let mut state = Checkpoint {
            version: 2,
            problem: self.problem.clone(),
            settings: self.settings.clone(),
            epoch: self.epoch,
            streams: self.streams.clone(),
            sectors: self
                .runs
                .iter()
                .map(|run| SectorState {
                    partitions: run
                        .grid
                        .continuous_dimensions
                        .iter()
                        .map(|d| d.partitioning.clone())
                        .collect(),
                    seeds: run.seeds.clone(),
                    records: run.records.values().cloned().collect(),
                })
                .collect(),
            integrity: [0; 32],
        };
        state.integrity = state.integrity()?;
        Ok(serde_json::to_vec(&state)?)
    }

    pub fn restore(bytes: &[u8], expected_problem: &IntegrationProblem) -> Result<Self> {
        let state: Checkpoint = serde_json::from_slice(bytes)?;
        expected_problem.validate()?;
        if state.version != 2
            || state.integrity != state.integrity()?
            || &state.problem != expected_problem
            || state.sectors.len() != state.problem.sectors.len()
            || state.streams.state == [0; 32]
        {
            return Err(IntegrationError::Invalid(
                "Havana checkpoint identity or shape differs".into(),
            ));
        }
        let mut session = Self::production(state.problem, state.settings)?;
        session.epoch = state.epoch;
        session.streams = state.streams;
        let mut all_seeds = BTreeSet::new();
        for (i, stored) in state.sectors.into_iter().enumerate() {
            if stored.partitions.len() != session.problem.sectors[i].dimension
                || stored.seeds.len() != session.settings.batches as usize
                || stored
                    .seeds
                    .iter()
                    .any(|s| *s == [0; 32] || !all_seeds.insert(*s))
            {
                return Err(IntegrationError::Invalid(
                    "Havana checkpoint grid dimension or random streams differ".into(),
                ));
            }
            for (dimension, partition) in session.runs[i]
                .grid
                .continuous_dimensions
                .iter_mut()
                .zip(stored.partitions)
            {
                if partition.len() != session.settings.bins + 1
                    || partition.first() != Some(&0.0)
                    || partition.last() != Some(&1.0)
                    || partition
                        .windows(2)
                        .any(|w| !w[0].is_finite() || w[0] >= w[1])
                {
                    return Err(IntegrationError::Invalid(
                        "invalid frozen grid partition".into(),
                    ));
                }
                dimension.partitioning = partition;
            }
            session.runs[i].grid_id = session.grid_id(i, &session.runs[i].grid)?;
            session.runs[i].seeds = stored.seeds;
            for record in stored.records {
                if session.validate_task(&record.task)? != i {
                    return Err(IntegrationError::Invalid(
                        "Monte Carlo batch is stored under the wrong sector".into(),
                    ));
                }
                if let Some(stats) = &record.weights {
                    stats.validate(
                        WeightLayout::new(&session.problem).len(),
                        record.task.points as u64,
                    )?;
                }
                if record.mean.len() != session.problem.orders.len()
                    || record.mean.iter().any(|v| !v.is_finite())
                    || !record.worker_seconds.is_finite()
                    || record.worker_seconds < 0.0
                    || session.runs[i]
                        .records
                        .insert(record.task.batch, record)
                        .is_some()
                {
                    return Err(IntegrationError::Invalid(
                        "invalid or duplicate Monte Carlo batch summary".into(),
                    ));
                }
            }
        }
        if session.streams.next < all_seeds.len() as u64
            || all_seeds.contains(&session.streams.state)
        {
            return Err(IntegrationError::Invalid(
                "Havana checkpoint random frontier overlaps reserved batches".into(),
            ));
        }
        Ok(session)
    }
}
