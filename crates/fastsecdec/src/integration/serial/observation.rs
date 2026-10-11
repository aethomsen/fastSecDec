//! Accepted scientific observations retain the design that supplied each row;
//! current residency/refinement progress is exposed separately by snapshot().
use super::SerialSession;
use crate::{
    integration::{
        ContributionReport, IntegrationObservation, ReplicaRelation, Result, SectorContribution,
        observation,
    },
    status::{IntegrationMethod, IntegrationStage, SectorSnapshot, UncertaintyStatus},
};
impl SerialSession {
    pub fn observation(&self) -> Result<IntegrationObservation> {
        let method = match self.settings.method {
            super::SerialMethod::Qmc(_) | super::SerialMethod::AdaptiveQmc(_) => {
                IntegrationMethod::AdaptiveQmc
            }
            _ => IntegrationMethod::HavanaMc,
        };
        let stage = if !self.sectors.is_empty()
            && self.sectors.iter().all(|s| s.pilot && s.previous.is_none())
        {
            IntegrationStage::Pilot
        } else {
            IntegrationStage::Production
        };
        let mut rows = Vec::with_capacity(self.sectors.len());
        for s in &self.sectors {
            let previous = s
                .previous
                .as_ref()
                .filter(|_| s.pilot || s.moments.count() < s.target);
            let (estimate, replicas, points, target) = if let Some(previous) = previous {
                (
                    Some(previous.estimate.clone()),
                    previous.replicas,
                    previous.points_per_replica,
                    previous.replicas,
                )
            } else if s.pilot {
                (None, 0, s.points, s.target)
            } else {
                (
                    self.sector_estimate(s)?,
                    s.moments.count(),
                    s.points,
                    s.target,
                )
            };
            let count = |n: u64| {
                n.checked_mul(points).ok_or_else(|| {
                    crate::integration::IntegrationError::Invalid(
                        "serial observation count overflow".into(),
                    )
                })
            };
            let progress = SectorSnapshot {
                id: s.id,
                dimension: s.dimension,
                completed_points: count(replicas)?,
                planned_points: Some(count(target)?),
                discrete_allocation: None,
                complete_replicas: replicas.try_into().map_err(|_| {
                    crate::integration::IntegrationError::Invalid(
                        "replica count exceeds addressable range".into(),
                    )
                })?,
                planned_replicas: target.try_into().map_err(|_| {
                    crate::integration::IntegrationError::Invalid(
                        "replica target exceeds addressable range".into(),
                    )
                })?,
                worker_seconds: s.worker_seconds,
            };
            rows.push(SectorContribution {
                used_replicas: progress.complete_replicas,
                used_points: progress.completed_points,
                uncertainty: if estimate.is_some() {
                    UncertaintyStatus::Available
                } else if stage == IntegrationStage::Pilot {
                    UncertaintyStatus::PilotOnly
                } else {
                    UncertaintyStatus::WaitingForCoverage
                },
                progress,
                estimate,
            });
        }
        let snapshot = observation::snapshot(
            &self.problem,
            method,
            stage,
            rows.iter().map(|r| r.progress.clone()).collect(),
            self.estimate(),
        )?;
        let contributions = ContributionReport::from_snapshot(
            &self.problem,
            snapshot.clone(),
            ReplicaRelation::IndependentAcrossSectors,
            rows,
        );
        Ok(IntegrationObservation {
            snapshot,
            contributions,
            weight_diagnostics: Some(self.weight_diagnostics()?),
        })
    }
}
