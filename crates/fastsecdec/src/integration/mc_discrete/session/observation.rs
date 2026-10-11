use super::*;
use crate::{
    integration::{
        ContributionReport, IntegrationObservation, ReplicaRelation, SectorContribution,
        VectorEstimate, estimate::precise_sum, observation::observed_estimate,
    },
    status::{DiscreteSectorAllocation, IntegrationMethod, IntegrationSnapshot, SectorSnapshot},
};
use fastsecdec_qmc::QmcEstimate;
impl HavanaDiscreteSession {
    pub fn estimate(&self) -> Result<VectorEstimate> {
        if self.stage == IntegrationStage::Pilot {
            return Err(IntegrationError::Unavailable(
                "pilot batches only train the native grid".into(),
            ));
        }
        if self.grid.is_none() {
            return Ok(VectorEstimate {
                orders: self.problem.orders.clone(),
                components: self.problem.components.clone(),
                mean: self.problem.exact_coefficients.clone(),
                standard_error: vec![0.0; self.problem.orders.len()],
                covariance_of_mean: vec![0.0; self.problem.orders.len().pow(2)],
                production_complete: true,
            });
        }
        let mut estimate = VectorEstimate::from_qmc(
            &self.problem.orders,
            &self.problem.components,
            QmcEstimate::from_shift_means(
                &self
                    .records
                    .values()
                    .map(|r| r.mean.clone())
                    .collect::<Vec<_>>(),
            )?,
            self.is_complete(),
        );
        for (value, exact) in estimate
            .mean
            .iter_mut()
            .zip(&self.problem.exact_coefficients)
        {
            *value = precise_sum([*value, *exact])?;
        }
        Ok(estimate)
    }
    fn progress(&self) -> Result<Vec<SectorSnapshot>> {
        self.problem
            .sectors
            .iter()
            .zip(self.sector_probabilities())
            .enumerate()
            .map(|(i, (spec, (_, probability)))| {
                Ok(SectorSnapshot {
                    id: spec.id,
                    dimension: spec.dimension,
                    completed_points: self.records.values().try_fold(0u64, |s, r| {
                        s.checked_add(r.counts[i]).ok_or_else(|| {
                            IntegrationError::Invalid("discrete point count overflow".into())
                        })
                    })?,
                    planned_points: None,
                    discrete_allocation: Some(DiscreteSectorAllocation {
                        probability,
                        points_per_batch: self.settings.batch.points_per_batch as u64,
                    }),
                    complete_replicas: self.records.len(),
                    planned_replicas: self.settings.batch.batches as usize,
                    worker_seconds: precise_sum(
                        self.records.values().map(|r| r.sector_seconds[i]),
                    )?,
                })
            })
            .collect()
    }
    pub fn snapshot(&self) -> Result<IntegrationSnapshot> {
        Ok(self.diagnostic_observation()?.snapshot)
    }
    pub fn contributions(&self) -> Result<ContributionReport> {
        Ok(self.diagnostic_observation()?.contributions)
    }
    /// Full batch covariance is authoritative; stochastic sector marginals have
    /// shared random selections and their covariance matrices must not be summed.
    pub fn diagnostic_observation(&self) -> Result<IntegrationObservation> {
        let (estimate, uncertainty) =
            observed_estimate(self.estimate(), self.stage, self.grid.is_none())?;
        let sectors = self.progress()?;
        let snapshot = IntegrationSnapshot {
            method: IntegrationMethod::HavanaDiscreteMc,
            stage: self.stage,
            completed_points: self.records.len() as u64
                * self.settings.batch.points_per_batch as u64,
            planned_points: if self.grid.is_none() {
                0
            } else {
                self.settings.batch.planned_points()?
            },
            complete_sectors: if self.is_complete() { sectors.len() } else { 0 },
            worker_seconds: precise_sum(self.records.values().map(|r| r.worker_seconds))?,
            sectors,
            uncertainty,
            estimate,
            stop_reason: None,
            evaluation_diagnostics: None,
        };
        let rows = snapshot
            .sectors
            .iter()
            .enumerate()
            .map(|(i, progress)| {
                let used = if self.stage == IntegrationStage::Production {
                    self.records.len()
                } else {
                    0
                };
                let means = self
                    .records
                    .values()
                    .map(|r| r.sector_means[i].clone())
                    .collect::<Vec<_>>();
                let result = if self.stage == IntegrationStage::Pilot {
                    Err(IntegrationError::Unavailable(
                        "pilot contributions are not production".into(),
                    ))
                } else {
                    QmcEstimate::from_shift_means(&means)
                        .map(|v| {
                            VectorEstimate::from_qmc(
                                &self.problem.orders,
                                &self.problem.components,
                                v,
                                self.is_complete(),
                            )
                        })
                        .map_err(Into::into)
                };
                let (estimate, uncertainty) = observed_estimate(result, self.stage, false)?;
                Ok(SectorContribution {
                    progress: progress.clone(),
                    used_replicas: used,
                    used_points: if used == 0 {
                        0
                    } else {
                        progress.completed_points
                    },
                    uncertainty,
                    estimate,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let contributions = ContributionReport::from_snapshot(
            &self.problem,
            snapshot.clone(),
            ReplicaRelation::SharedAcrossSectors,
            rows,
        );
        Ok(IntegrationObservation {
            snapshot,
            contributions,
            weight_diagnostics: Some(self.weight_diagnostics()?),
        })
    }
}
