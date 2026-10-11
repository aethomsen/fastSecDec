mod contributions;
mod live;
mod observation;
#[cfg(test)]
mod tests;

use super::*;
use crate::{
    integration::VectorEstimate,
    status::{IntegrationSnapshot, SectorSnapshot, UncertaintyStatus},
};
use fastsecdec_qmc::{QmcEstimate, ShiftEstimate};

impl QmcSession {
    /// Complete same-shift totals across every stochastic sector, including
    /// exact contributions, in increasing shift-ID order. A partial sector
    /// never contributes a row. Only democratic production shares shift IDs;
    /// adaptive allocations must use their separate sector estimates instead.
    /// An all-exact problem has no stochastic replicas and returns an empty list.
    ///
    /// These absolute binary64 vectors are diagnostics. `estimate()` retains
    /// sector-centered differences to preserve covariance beside large offsets.
    pub fn complete_shift_estimates(&self) -> Result<Vec<ShiftEstimate>> {
        if self.stage != IntegrationStage::Production
            || self.method != IntegrationMethod::DemocraticQmc
        {
            return Err(IntegrationError::Unavailable(
                "common shift totals require democratic QMC production".into(),
            ));
        }
        self.common_shift_rows()?
            .iter()
            .map(|rows| {
                Ok(ShiftEstimate {
                    shift: rows[0].shift,
                    mean: (0..self.problem.orders.len())
                        .map(|j| {
                            precise_sum(
                                rows.iter()
                                    .map(|row| row.mean[j])
                                    .chain([self.problem.exact_coefficients[j]]),
                            )
                        })
                        .collect::<Result<_>>()?,
                })
            })
            .collect()
    }

    fn common_shift_rows(&self) -> Result<Vec<Vec<ShiftEstimate>>> {
        let means: Vec<_> = self
            .runs
            .iter()
            .map(|run| run.accumulator.shift_estimates())
            .collect::<std::result::Result<_, _>>()?;
        let Some(first_sector) = means.first() else {
            return Ok(Vec::new());
        };
        Ok(first_sector
            .iter()
            .filter_map(|first| {
                means
                    .iter()
                    // Native shift_estimates enumerates each complete shift
                    // once in increasing ID order, including when gaps remain.
                    .map(|rows| {
                        rows.binary_search_by_key(&first.shift, |row| row.shift)
                            .ok()
                            .map(|index| rows[index].clone())
                    })
                    .collect()
            })
            .collect())
    }

    pub fn estimate(&self) -> Result<VectorEstimate> {
        if self.stage == IntegrationStage::Pilot {
            return Err(IntegrationError::Unavailable(
                "pilot samples are not production evidence".into(),
            ));
        }
        let n = self.problem.orders.len();
        if self.runs.is_empty() {
            return Ok(VectorEstimate {
                orders: self.problem.orders.clone(),
                components: self.problem.components.clone(),
                mean: self.problem.exact_coefficients.clone(),
                standard_error: vec![0.0; n],
                covariance_of_mean: vec![0.0; n * n],
                production_complete: true,
            });
        }
        let value = if self.method == IntegrationMethod::DemocraticQmc {
            let complete_rows = self.common_shift_rows()?;
            let Some(anchor) = complete_rows.first() else {
                return Err(fastsecdec_qmc::QmcError::InsufficientShifts { complete: 0 }.into());
            };
            // Center each sector before aggregation. Exact constants enter the
            // absolute mean only, preserving both small covariance and large
            // cancellations involving whole zero-dimensional sectors.
            let aggregate = complete_rows
                .iter()
                .map(|rows| {
                    (0..n)
                        .map(|j| {
                            precise_sum(rows.iter().zip(anchor).map(|(v, a)| v.mean[j] - a.mean[j]))
                        })
                        .collect::<Result<Vec<_>>>()
                })
                .collect::<Result<Vec<_>>>()?;
            let mut value = QmcEstimate::from_shift_means(&aggregate)?;
            for (j, mean) in value.mean.iter_mut().enumerate() {
                *mean = precise_sum(
                    anchor
                        .iter()
                        .map(|a| a.mean[j])
                        .chain([self.problem.exact_coefficients[j], *mean]),
                )?;
            }
            value
        } else {
            let estimates: Vec<_> = self
                .runs
                .iter()
                .map(|r| r.accumulator.estimate())
                .collect::<std::result::Result<_, _>>()?;
            let mean = (0..n)
                .map(|j| {
                    precise_sum(
                        estimates
                            .iter()
                            .map(|v| v.mean[j])
                            .chain([self.problem.exact_coefficients[j]]),
                    )
                })
                .collect::<Result<Vec<_>>>()?;
            let covariance_of_mean = (0..n * n)
                .map(|j| precise_sum(estimates.iter().map(|v| v.covariance_of_mean[j])))
                .collect::<Result<Vec<_>>>()?;
            let standard_error = (0..n)
                .map(|j| covariance_of_mean[j * n + j].sqrt())
                .collect();
            QmcEstimate {
                mean,
                standard_error,
                covariance_of_mean,
                complete_shifts: 0,
                completed_points: 0,
                used_points: 0,
            }
        };
        Ok(VectorEstimate::from_qmc(
            &self.problem.orders,
            &self.problem.components,
            value,
            self.is_complete(),
        ))
    }

    fn progress_sectors(&self) -> Result<Vec<SectorSnapshot>> {
        self.problem
            .sectors
            .iter()
            .zip(&self.runs)
            .map(|(spec, run)| {
                Ok(SectorSnapshot {
                    id: spec.id,
                    dimension: spec.dimension,
                    completed_points: run.accumulator.completed_points(),
                    planned_points: Some(run.accumulator.plan().total_points()),
                    discrete_allocation: None,
                    complete_replicas: run.accumulator.complete_shift_ids().len(),
                    planned_replicas: run.accumulator.plan().shift_count(),
                    worker_seconds: run.seconds()?,
                })
            })
            .collect()
    }

    pub fn snapshot(&self) -> Result<IntegrationSnapshot> {
        let sectors = self.progress_sectors()?;
        let estimate = match self.estimate() {
            Ok(value) => Some(value),
            Err(IntegrationError::Unavailable(_))
            | Err(IntegrationError::Qmc(fastsecdec_qmc::QmcError::InsufficientShifts { .. })) => {
                None
            }
            Err(error) => return Err(error),
        };
        let uncertainty = if self.runs.is_empty() {
            UncertaintyStatus::Exact
        } else if self.stage == IntegrationStage::Pilot {
            UncertaintyStatus::PilotOnly
        } else if estimate.is_some() {
            UncertaintyStatus::Available
        } else {
            UncertaintyStatus::WaitingForCoverage
        };
        Ok(IntegrationSnapshot {
            method: self.method,
            stage: self.stage,
            completed_points: checked_count(sectors.iter().map(|s| s.completed_points))?,
            planned_points: checked_count(
                sectors
                    .iter()
                    .map(|s| s.planned_points.expect("fixed QMC quota")),
            )?,
            complete_sectors: self
                .runs
                .iter()
                .filter(|r| r.accumulator.is_complete())
                .count(),
            worker_seconds: precise_sum(sectors.iter().map(|s| s.worker_seconds))?,
            sectors,
            uncertainty,
            estimate,
            stop_reason: None,
            evaluation_diagnostics: None,
        })
    }
}

fn checked_count(mut values: impl Iterator<Item = u64>) -> Result<u64> {
    values.try_fold(0u64, |sum, value| {
        sum.checked_add(value)
            .ok_or_else(|| IntegrationError::Invalid("total point count overflow".into()))
    })
}

pub(super) fn observation_estimate(session: &QmcSession) -> Result<Option<VectorEstimate>> {
    crate::integration::observation::observed_estimate(
        session.estimate(),
        session.stage,
        session.runs.is_empty(),
    )
    .map(|x| x.0)
}
