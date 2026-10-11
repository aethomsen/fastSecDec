use super::SerialSession;
use crate::{
    integration::{
        IntegrationError, Result, WeightDiagnosticStatus as Status, WeightDiagnostics, weights,
    },
    status::IntegrationStage,
};
impl SerialSession {
    pub fn weight_diagnostics(&self) -> Result<WeightDiagnostics> {
        let estimate = crate::integration::observation::observed_estimate(
            self.estimate(),
            IntegrationStage::Production,
            self.sectors.is_empty(),
        )?
        .0;
        let mut status = if !self.sectors.is_empty()
            && self.sectors.iter().all(|s| s.pilot && s.previous.is_none())
        {
            Status::PilotOnly
        } else {
            Status::Available
        };
        let mut count = 0u64;
        let mut groups = Vec::new();
        for s in &self.sectors {
            let (stats, points) = if let Some(previous) = s
                .previous
                .as_ref()
                .filter(|_| s.pilot || s.moments.count() < s.target)
            {
                (
                    &previous.weights,
                    previous.replicas.checked_mul(previous.points_per_replica),
                )
            } else if s.pilot {
                if status == Status::Available {
                    status = Status::WaitingForCoverage;
                }
                continue;
            } else {
                if s.moments.count() == 0 && status == Status::Available {
                    status = Status::WaitingForCoverage;
                }
                (&s.weights, s.moments.count().checked_mul(s.points))
            };
            count = count
                .checked_add(points.ok_or(IntegrationError::NumericRange)?)
                .ok_or(IntegrationError::NumericRange)?;
            if let Some(stats) = stats {
                groups.push(stats.clone());
            } else {
                status = Status::LegacyUnavailable;
            }
        }
        Ok(weights::report(
            &self.problem,
            estimate.as_ref(),
            status,
            count,
            &groups,
        ))
    }
}
