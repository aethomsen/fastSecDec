use super::*;
use crate::status::{
    IntegrationMethod, IntegrationSnapshot, IntegrationStage, SectorSnapshot, UncertaintyStatus,
};
use serde::{Deserialize, Serialize};

/// Failure-safe native observation. Statistical range failures are explicit
/// statuses; structural state errors still return `Err`. No failed worker
/// prefix or synthetic estimate enters the accepted coverage.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IntegrationObservation {
    pub snapshot: IntegrationSnapshot,
    pub contributions: ContributionReport,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight_diagnostics: Option<WeightDiagnostics>,
}

pub(crate) fn observed_estimate(
    result: Result<VectorEstimate>,
    stage: IntegrationStage,
    exact: bool,
) -> Result<(Option<VectorEstimate>, UncertaintyStatus)> {
    Ok(match result {
        Ok(estimate) => (
            Some(estimate),
            if exact {
                UncertaintyStatus::Exact
            } else {
                UncertaintyStatus::Available
            },
        ),
        Err(IntegrationError::Unavailable(_))
        | Err(IntegrationError::Qmc(fastsecdec_qmc::QmcError::InsufficientShifts { .. })) => (
            None,
            if stage == IntegrationStage::Pilot {
                UncertaintyStatus::PilotOnly
            } else {
                UncertaintyStatus::WaitingForCoverage
            },
        ),
        Err(error) if error.is_statistical_range() => (
            None,
            UncertaintyStatus::StatisticalFailure {
                reason: error.to_string(),
            },
        ),
        Err(error) => return Err(error),
    })
}

pub(crate) fn snapshot(
    problem: &IntegrationProblem,
    method: IntegrationMethod,
    stage: IntegrationStage,
    sectors: Vec<SectorSnapshot>,
    estimate: Result<VectorEstimate>,
) -> Result<IntegrationSnapshot> {
    let (estimate, uncertainty) = observed_estimate(estimate, stage, problem.sectors.is_empty())?;
    let count = |planned: bool| {
        sectors.iter().try_fold(0u64, |sum, row| {
            sum.checked_add(if planned {
                row.planned_points.ok_or_else(|| {
                    IntegrationError::Invalid(
                        "fixed allocation observation requires sector quotas".into(),
                    )
                })?
            } else {
                row.completed_points
            })
            .ok_or_else(|| IntegrationError::Invalid("observation point count overflow".into()))
        })
    };
    Ok(IntegrationSnapshot {
        method,
        stage,
        completed_points: count(false)?,
        planned_points: count(true)?,
        complete_sectors: sectors
            .iter()
            .filter(|s| Some(s.completed_points) == s.planned_points)
            .count(),
        worker_seconds: estimate::precise_sum(sectors.iter().map(|s| s.worker_seconds))?,
        sectors,
        uncertainty,
        estimate,
        stop_reason: None,
        evaluation_diagnostics: None,
    })
}

impl SectorContribution {
    pub(crate) fn diagnostic(
        problem: &IntegrationProblem,
        stage: IntegrationStage,
        progress: SectorSnapshot,
        means: Result<Vec<Vec<f64>>>,
        selected: usize,
        points: u64,
    ) -> Result<Self> {
        let used_replicas = if stage == IntegrationStage::Pilot {
            0
        } else {
            selected
        };
        let used_points = (used_replicas as u64).checked_mul(points).ok_or_else(|| {
            IntegrationError::Invalid("observation selected count overflow".into())
        })?;
        let result = means.and_then(|means| {
            Self::from_replicas(problem, stage, progress.clone(), &means, points)
        });
        match result {
            Ok(row) => {
                if row.used_replicas != used_replicas {
                    return Err(IntegrationError::Invalid(
                        "native estimate and metadata coverage differ".into(),
                    ));
                }
                Ok(row)
            }
            Err(error) if error.is_statistical_range() => Ok(Self {
                progress,
                used_replicas,
                used_points,
                uncertainty: UncertaintyStatus::StatisticalFailure {
                    reason: error.to_string(),
                },
                estimate: None,
            }),
            Err(error) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structural_errors_are_not_disguised_as_statistical_range_failures() {
        let structural = IntegrationError::Qmc(fastsecdec_qmc::QmcError::OutputDimension {
            expected: 2,
            actual: 1,
        });
        assert!(matches!(
            observed_estimate(Err(structural), IntegrationStage::Production, false),
            Err(IntegrationError::Qmc(
                fastsecdec_qmc::QmcError::OutputDimension { .. }
            ))
        ));
        let observed = observed_estimate(
            Err(IntegrationError::NumericRange),
            IntegrationStage::Production,
            false,
        )
        .unwrap();
        assert!(matches!(
            observed.1,
            UncertaintyStatus::StatisticalFailure { .. }
        ));
        assert!(observed.0.is_none());
    }
}
