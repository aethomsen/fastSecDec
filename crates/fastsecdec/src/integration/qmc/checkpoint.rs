use super::*;

#[derive(Serialize, Deserialize)]
struct Checkpoint {
    version: u8,
    configuration_id: [u8; 32],
    problem: IntegrationProblem,
    settings: QmcSettings,
    method: IntegrationMethod,
    stage: IntegrationStage,
    epoch: u64,
    allocations: Vec<ProductionAllocation>,
    runs: Vec<SectorRun>,
}

impl QmcSession {
    fn configuration_id(&self) -> Result<[u8; 32]> {
        Ok(*blake3::hash(&serde_json::to_vec(&(
            &self.problem,
            &self.settings,
            self.method,
            self.stage,
            self.epoch,
            &self.allocations,
        ))?)
        .as_bytes())
    }
    pub fn checkpoint(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(&Checkpoint {
            version: 1,
            configuration_id: self.configuration_id()?,
            problem: self.problem.clone(),
            settings: self.settings.clone(),
            method: self.method,
            stage: self.stage,
            epoch: self.epoch,
            allocations: self.allocations.clone(),
            runs: self.runs.clone(),
        })?)
    }

    /// Pending work is intentionally not restored: every unfinished canonical
    /// package becomes available for reissue. Completed packages remain merged.
    pub fn restore(bytes: &[u8], expected_problem: &IntegrationProblem) -> Result<Self> {
        let mut state: Checkpoint = serde_json::from_slice(bytes)?;
        expected_problem.validate()?;
        if state.version != 1
            || &state.problem != expected_problem
            || matches!(
                state.method,
                IntegrationMethod::HavanaMc | IntegrationMethod::HavanaDiscreteMc
            )
            || (state.method == IntegrationMethod::DemocraticQmc
                && state.stage != IntegrationStage::Production)
        {
            return Err(IntegrationError::Invalid(
                "checkpoint version, method or complete integral identity differs".into(),
            ));
        }
        state.settings.validate()?;
        let mut session = Self {
            problem: state.problem,
            settings: state.settings,
            method: state.method,
            stage: state.stage,
            epoch: state.epoch,
            allocations: state.allocations,
            runs: Vec::new(),
            cursor: 0,
        };
        if session.configuration_id()? != state.configuration_id {
            return Err(IntegrationError::Invalid(
                "checkpoint immutable integration configuration differs".into(),
            ));
        }
        let expected = session.make_runs()?;
        if expected.len() != state.runs.len() {
            return Err(IntegrationError::Invalid(
                "checkpoint does not cover every sector".into(),
            ));
        }
        for (reference, run) in expected.iter().zip(&mut state.runs) {
            if run.package_boundaries.is_empty() {
                run.package_boundaries = vec![0, run.accumulator.plan().total_points()];
            }
            if reference.accumulator.plan() != run.accumulator.plan()
                || reference.accumulator.output_count() != run.accumulator.output_count()
                || run.costs.values().any(|v| !v.is_finite() || *v < 0.0)
                || run.package_boundaries.first() != Some(&0)
                || run.package_boundaries.last() != Some(&run.accumulator.plan().total_points())
                || run.package_boundaries.windows(2).any(|v| v[0] >= v[1])
                || run
                    .package_boundaries
                    .iter()
                    .any(|end| !end.is_multiple_of(run.accumulator.plan().rule().points()))
            {
                return Err(IntegrationError::Invalid(
                    "checkpoint plans, output shape or timings differ".into(),
                ));
            }
            run.seconds()?;
            if let Some(rows) = &run.weights {
                if !rows.keys().eq(run.costs.keys()) {
                    return Err(IntegrationError::Invalid(
                        "weight packages differ from accepted packages".into(),
                    ));
                }
                for (&start, stats) in rows {
                    let work = run.canonical_work(start, session.settings.package_points)?;
                    stats.validate(
                        start,
                        work.point_count(),
                        run.accumulator.plan().rule().points(),
                        crate::integration::weights::WeightLayout::new(&session.problem).len(),
                    )?;
                }
            }
            let mut starts = BTreeSet::new();
            for work in run.accumulator.completed_work_packages() {
                if run.canonical_work(work.start(), session.settings.package_points)? != work {
                    return Err(IntegrationError::Invalid(
                        "checkpoint contains a noncanonical completed package".into(),
                    ));
                }
                starts.insert(work.start());
            }
            if !starts.iter().eq(run.costs.keys()) {
                return Err(IntegrationError::Invalid(
                    "checkpoint timings do not correspond one-to-one to completed packages".into(),
                ));
            }
        }
        session.runs = state.runs;
        Ok(session)
    }
}
