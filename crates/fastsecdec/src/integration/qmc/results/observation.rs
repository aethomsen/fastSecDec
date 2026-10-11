use super::*;
use crate::integration::{
    ContributionReport, IntegrationObservation, ReplicaRelation, SectorContribution, observation,
};

impl QmcSession {
    /// Observe accepted work even when a native mean/covariance cannot be
    /// represented. Valid totals and marginal estimates are retained separately.
    pub fn diagnostic_observation(&self) -> Result<IntegrationObservation> {
        let progress = self.progress_sectors()?;
        let snapshot = observation::snapshot(
            &self.problem,
            self.method,
            self.stage,
            progress,
            self.estimate(),
        )?;
        let shared = self.method == IntegrationMethod::DemocraticQmc;
        let ids: Vec<_> = self
            .runs
            .iter()
            .map(|r| r.accumulator.complete_shift_ids())
            .collect();
        let common: std::collections::BTreeSet<_> = ids
            .first()
            .into_iter()
            .flatten()
            .copied()
            .filter(|id| ids.iter().all(|row| row.binary_search(id).is_ok()))
            .collect();
        let sectors = self
            .runs
            .iter()
            .zip(&snapshot.sectors)
            .enumerate()
            .map(|(i, (run, progress))| {
                let means = if self.stage == IntegrationStage::Pilot {
                    Ok(Vec::new())
                } else {
                    run.accumulator
                        .shift_estimates()
                        .map(|rows| {
                            rows.into_iter()
                                .filter(|row| !shared || common.contains(&row.shift))
                                .map(|row| row.mean)
                                .collect()
                        })
                        .map_err(IntegrationError::from)
                };
                SectorContribution::diagnostic(
                    &self.problem,
                    self.stage,
                    progress.clone(),
                    means,
                    if shared { common.len() } else { ids[i].len() },
                    run.accumulator.plan().rule().points(),
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let contributions = ContributionReport::from_snapshot(
            &self.problem,
            snapshot.clone(),
            if shared {
                ReplicaRelation::SharedAcrossSectors
            } else {
                ReplicaRelation::IndependentAcrossSectors
            },
            sectors,
        );
        Ok(IntegrationObservation {
            snapshot,
            contributions,
            weight_diagnostics: Some(self.weight_diagnostics()?),
        })
    }
}
