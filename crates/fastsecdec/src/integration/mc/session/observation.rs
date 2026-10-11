use super::*;
use crate::{
    integration::{
        ContributionReport, IntegrationObservation, ReplicaRelation, SectorContribution,
        observation,
    },
    status::IntegrationMethod,
};

impl HavanaSession {
    /// Observe accepted batches independently of statistical range failures.
    pub fn diagnostic_observation(&self) -> Result<IntegrationObservation> {
        let snapshot = observation::snapshot(
            &self.problem,
            IntegrationMethod::HavanaMc,
            self.stage,
            self.progress_sectors()?,
            self.estimate(),
        )?;
        let sectors = self
            .runs
            .iter()
            .zip(&snapshot.sectors)
            .map(|(run, progress)| {
                let means = if self.stage == IntegrationStage::Pilot {
                    Vec::new()
                } else {
                    run.records
                        .values()
                        .map(|record| record.mean.clone())
                        .collect()
                };
                SectorContribution::diagnostic(
                    &self.problem,
                    self.stage,
                    progress.clone(),
                    Ok(means),
                    run.records.len(),
                    self.settings.points_per_batch as u64,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let contributions = ContributionReport::from_snapshot(
            &self.problem,
            snapshot.clone(),
            ReplicaRelation::IndependentAcrossSectors,
            sectors,
        );
        Ok(IntegrationObservation {
            snapshot,
            contributions,
            weight_diagnostics: Some(self.weight_diagnostics()?),
        })
    }
}
