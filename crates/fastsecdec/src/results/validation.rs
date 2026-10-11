use super::*;
use crate::{
    integration::{ReplicaRelation, SectorContribution, VectorEstimate},
    status::{IntegrationMethod, IntegrationStage, StoppingReason, UncertaintyStatus},
};
use std::collections::{BTreeMap, BTreeSet};

fn require(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(ResultError::Invalid(message.into()))
    }
}

impl SavedIntegrationResult {
    pub fn validate(&self) -> Result<()> {
        self.manifest.validate()?;
        require(
            self.manifest.source_selection.is_none() || !self.scope.is_full_integral(),
            "a generated source subset cannot be reported as the full original integral",
        )?;
        self.provenance.validate()?;
        if let crate::reference::ReferenceValidation::Checked { evidence } = &self.validation {
            require(
                !evidence.trim().is_empty(),
                "checked computation requires evidence",
            )?;
        }
        let manifest = &self.manifest;
        let report = &self.contributions;
        require(
            report.orders == manifest.orders && report.components == manifest.components,
            "contribution and parent coefficient layouts differ",
        )?;
        let all: BTreeMap<_, _> = manifest
            .sectors
            .iter()
            .map(|s| (s.id, s.dimension))
            .collect();
        let projected = manifest.integration_problem(&self.scope, &manifest.kernel_content_id)?;
        let selected: BTreeSet<_> = projected.sectors.iter().map(|s| s.id).collect();
        let exact = &projected.exact_coefficients;
        require(
            report.exact_coefficients.as_slice() == exact.as_slice(),
            "exact contribution differs from the declared scope policy",
        )?;
        let row_ids: BTreeSet<_> = report.sectors.iter().map(|s| s.progress.id).collect();
        require(
            row_ids == selected && row_ids.len() == report.sectors.len(),
            "contribution rows must cover exactly the selected sectors",
        )?;
        require(
            report.replica_relation
                == if matches!(
                    report.method,
                    IntegrationMethod::DemocraticQmc | IntegrationMethod::HavanaDiscreteMc
                ) {
                    ReplicaRelation::SharedAcrossSectors
                } else {
                    ReplicaRelation::IndependentAcrossSectors
                },
            "replica dependence differs from integration method",
        )?;
        require(
            !(report.method == IntegrationMethod::DemocraticQmc
                && report.stage == IntegrationStage::Pilot),
            "democratic QMC has no pilot",
        )?;
        for row in &report.sectors {
            require(
                all[&row.progress.id] == row.progress.dimension,
                "sector dimension differs from parent manifest",
            )?;
            self.validate_row(row)?;
        }
        if report.replica_relation == ReplicaRelation::SharedAcrossSectors {
            require(
                report.sectors.windows(2).all(|rows| {
                    rows[0].used_replicas == rows[1].used_replicas
                        && rows[0].progress.planned_replicas == rows[1].progress.planned_replicas
                }),
                "democratic sectors require the same common selected shifts",
            )?;
            if report.stage == IntegrationStage::Production
                && let Some(first) = report.sectors.first()
            {
                let minimum_common =
                    report
                        .sectors
                        .iter()
                        .fold(first.progress.planned_replicas, |minimum, row| {
                            minimum.saturating_sub(
                                row.progress.planned_replicas - row.progress.complete_replicas,
                            )
                        });
                require(
                    first.used_replicas >= minimum_common,
                    "common replica count is incompatible with sector completion counts",
                )?;
            }
        }
        if report.method == IntegrationMethod::HavanaDiscreteMc {
            self.validate_discrete_allocation()?;
        }
        let complete = self.production_complete();
        validate_status(
            &report.uncertainty,
            report.total.as_ref(),
            report.stage,
            report.sectors.is_empty(),
        )?;
        if report.uncertainty == UncertaintyStatus::WaitingForCoverage {
            require(
                report.sectors.iter().any(|row| row.used_replicas < 2),
                "missing total with sufficient coverage requires statistical-failure status",
            )?;
        }
        if let Some(total) = &report.total {
            self.validate_estimate(total)?;
            require(
                total.production_complete == complete,
                "total completion flag differs from accepted scoped coverage",
            )?;
            require(
                report.sectors.iter().all(|row| row.used_replicas >= 2),
                "total estimate needs two complete replicas from every stochastic sector",
            )?;
            if report.sectors.is_empty() {
                require(
                    total.mean.as_slice() == exact.as_slice()
                        && total.standard_error.iter().all(|v| *v == 0.0)
                        && total.covariance_of_mean.iter().all(|v| *v == 0.0),
                    "exact-only estimate differs from exact vector/zero covariance",
                )?;
            }
        }
        if let Some(weights) = &self.weight_diagnostics {
            let points = report.sectors.iter().try_fold(0_u64, |sum, sector| {
                sum.checked_add(sector.used_points).ok_or_else(|| {
                    ResultError::Invalid("accepted weight point count overflow".into())
                })
            })?;
            weights.validate(
                &report.orders,
                &report.components,
                &report.exact_coefficients,
                report.total.as_ref(),
                points,
            )?;
        }
        if let Some(tolerance) = self.requested_tolerance {
            tolerance.validate()?;
        }
        self.requested_accuracy_target
            .validate_layout(&manifest.orders)?;
        match &self.stopping_reason {
            StoppingReason::TargetReached => {
                let tolerance = self.requested_tolerance.ok_or_else(|| {
                    ResultError::Invalid("target stop requires recorded tolerance".into())
                })?;
                require(
                    report.total.as_ref().is_some_and(|v| {
                        v.meets_target(self.requested_accuracy_target, tolerance)
                            .unwrap_or(false)
                    }),
                    "target stop is unsupported by the native complete estimate",
                )?;
            }
            StoppingReason::PlannedWorkComplete => require(
                complete,
                "planned-complete stop requires full production coverage within scope",
            )?,
            StoppingReason::NumericalFailure(reason) => require(
                !reason.trim().is_empty(),
                "numerical failure requires a reason",
            )?,
            _ => {}
        }
        if let Some(design) = &self.qmc_design {
            self.validate_design(design)?;
        }
        if let Some(stored) = &self.stored_reference {
            require(
                stored.context.kernel_content_id == manifest.kernel_content_id,
                "stored comparison context differs from parent kernel",
            )?;
            stored.context.validate_reference(&stored.reference)?;
        }
        for time in [
            self.timings.elapsed_seconds,
            self.timings.artifact_load_seconds,
        ]
        .into_iter()
        .flatten()
        {
            require(
                time.is_finite() && time >= 0.0,
                "result timings must be finite and nonnegative",
            )?;
        }
        Ok(())
    }

    pub fn production_complete(&self) -> bool {
        self.contributions.stage == IntegrationStage::Production
            && self.contributions.sectors.iter().all(|row| {
                (self.contributions.method == IntegrationMethod::HavanaDiscreteMc
                    || Some(row.progress.completed_points) == row.progress.planned_points)
                    && row.used_replicas == row.progress.planned_replicas
            })
    }

    fn validate_estimate(&self, value: &VectorEstimate) -> Result<()> {
        value.validate()?;
        require(
            value.orders == self.manifest.orders && value.components == self.manifest.components,
            "estimate layout differs from manifest",
        )
    }

    fn validate_row(&self, row: &SectorContribution) -> Result<()> {
        let p = &row.progress;
        if self.contributions.method == IntegrationMethod::HavanaDiscreteMc {
            let allocation = p.discrete_allocation.as_ref().ok_or_else(|| {
                ResultError::Invalid("discrete allocation metadata is missing".into())
            })?;
            require(
                p.planned_points.is_none()
                    && p.planned_replicas >= 2
                    && allocation.points_per_batch >= 2
                    && allocation.probability.is_finite()
                    && allocation.probability > 0.0
                    && allocation.probability <= 1.0,
                "invalid stochastic sector allocation",
            )?;
            require(
                p.complete_replicas <= p.planned_replicas
                    && row.used_replicas
                        == if self.contributions.stage == IntegrationStage::Pilot {
                            0
                        } else {
                            p.complete_replicas
                        },
                "discrete batch coverage mismatch",
            )?;
            require(
                row.used_points
                    == if self.contributions.stage == IntegrationStage::Pilot {
                        0
                    } else {
                        p.completed_points
                    },
                "discrete selected point count mismatch",
            )?;
        } else {
            require(
                p.discrete_allocation.is_none(),
                "fixed-sector method cannot claim discrete allocation",
            )?;
            let planned_points = p
                .planned_points
                .ok_or_else(|| ResultError::Invalid("fixed-sector quota is missing".into()))?;
            require(
                p.planned_replicas > 0
                    && planned_points > 0
                    && planned_points.is_multiple_of(p.planned_replicas as u64),
                "invalid equal-size replica allocation",
            )?;
            let points = planned_points / p.planned_replicas as u64;
            require(
                p.completed_points <= planned_points
                    && p.complete_replicas <= p.planned_replicas
                    && row.used_replicas <= p.complete_replicas
                    && p.complete_replicas as u64 * points <= p.completed_points,
                "accepted or selected coverage exceeds allocation",
            )?;
            require(
                row.used_points == row.used_replicas as u64 * points,
                "selected point count differs from selected replicas",
            )?;
            require(
                p.completed_points != planned_points || p.complete_replicas == p.planned_replicas,
                "complete point coverage requires all replicas complete",
            )?;
            require(
                p.completed_points
                    <= planned_points - (p.planned_replicas - p.complete_replicas) as u64,
                "accepted points would necessarily complete more replicas",
            )?;
        }
        require(
            p.worker_seconds.is_finite() && p.worker_seconds >= 0.0,
            "invalid worker time",
        )?;
        validate_status(
            &row.uncertainty,
            row.estimate.as_ref(),
            self.contributions.stage,
            false,
        )?;
        if self.contributions.stage == IntegrationStage::Pilot {
            require(
                row.used_replicas == 0,
                "pilot observations cannot become selected production replicas",
            )?;
        }
        if let Some(estimate) = &row.estimate {
            self.validate_estimate(estimate)?;
            require(
                row.used_replicas >= 2
                    && estimate.production_complete
                        == (row.used_replicas == p.planned_replicas
                            && (self.contributions.method == IntegrationMethod::HavanaDiscreteMc
                                || Some(p.completed_points) == p.planned_points)),
                "marginal estimate has inconsistent replica coverage",
            )?;
        } else if row.uncertainty == UncertaintyStatus::WaitingForCoverage {
            require(
                row.used_replicas < 2,
                "complete marginal statistics cannot be labelled waiting for coverage",
            )?;
        }
        Ok(())
    }

    fn validate_discrete_allocation(&self) -> Result<()> {
        let rows = &self.contributions.sectors;
        let Some(first) = rows.first() else {
            return Ok(());
        };
        let batch = first
            .progress
            .discrete_allocation
            .as_ref()
            .unwrap()
            .points_per_batch;
        let expected = (first.progress.complete_replicas as u64)
            .checked_mul(batch)
            .ok_or_else(|| ResultError::Invalid("discrete coverage overflow".into()))?;
        let counts = rows
            .iter()
            .try_fold(0u64, |sum, row| {
                sum.checked_add(row.progress.completed_points)
            })
            .ok_or_else(|| ResultError::Invalid("discrete coverage sum overflow".into()))?;
        let probabilities = rows
            .iter()
            .map(|r| r.progress.discrete_allocation.as_ref().unwrap().probability)
            .sum::<f64>();
        require(
            counts == expected
                && (probabilities - 1.0).abs() <= 16.0 * f64::EPSILON * rows.len() as f64,
            "global discrete counts or probabilities are inconsistent",
        )?;
        require(
            rows.iter().all(|row| {
                row.progress.complete_replicas == first.progress.complete_replicas
                    && row.progress.planned_replicas == first.progress.planned_replicas
                    && row
                        .progress
                        .discrete_allocation
                        .as_ref()
                        .unwrap()
                        .points_per_batch
                        == batch
            }),
            "discrete sectors require the same complete global batches",
        )
    }

    fn validate_design(&self, design: &crate::integration::QmcDesign) -> Result<()> {
        require(
            !matches!(
                self.contributions.method,
                IntegrationMethod::HavanaMc | IntegrationMethod::HavanaDiscreteMc
            ),
            "Havana results cannot claim a QMC design",
        )?;
        design.settings.validate()?;
        let rows: BTreeMap<_, _> = self
            .contributions
            .sectors
            .iter()
            .map(|row| (row.progress.id, &row.progress))
            .collect();
        let ids: BTreeSet<_> = design.allocations.iter().map(|a| a.sector_id).collect();
        require(
            ids.len() == design.allocations.len()
                && ids == rows.keys().copied().collect::<BTreeSet<_>>(),
            "QMC design must cover exactly the selected sectors",
        )?;
        for allocation in &design.allocations {
            let p = rows[&allocation.sector_id];
            design.settings.validate_allocation(
                p.dimension,
                allocation.points,
                allocation.shifts,
            )?;
            require(
                Some(allocation.points * allocation.shifts as u64) == p.planned_points
                    && allocation.shifts as usize == p.planned_replicas,
                "QMC design differs from accepted coverage metadata",
            )?;
        }
        Ok(())
    }
}

fn validate_status(
    status: &UncertaintyStatus,
    estimate: Option<&VectorEstimate>,
    stage: IntegrationStage,
    exact: bool,
) -> Result<()> {
    match status {
        UncertaintyStatus::Available => require(
            stage == IntegrationStage::Production && !exact && estimate.is_some(),
            "available uncertainty requires a stochastic production estimate",
        ),
        UncertaintyStatus::Exact => require(
            stage == IntegrationStage::Production && exact && estimate.is_some(),
            "exact status requires an exact-only production estimate",
        ),
        UncertaintyStatus::PilotOnly => require(
            stage == IntegrationStage::Pilot && estimate.is_none(),
            "pilot uncertainty cannot contain a production estimate",
        ),
        UncertaintyStatus::WaitingForCoverage => require(
            stage == IntegrationStage::Production && !exact && estimate.is_none(),
            "waiting status requires missing stochastic production estimate",
        ),
        UncertaintyStatus::StatisticalFailure { reason } => require(
            stage == IntegrationStage::Production
                && estimate.is_none()
                && !reason.trim().is_empty(),
            "statistical failure requires an absent estimate and explicit reason",
        ),
    }
}
