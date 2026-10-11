use super::*;
use crate::integration::{WeightDiagnosticStatus as Status, WeightDiagnostics, weights};
impl HavanaDiscreteSession {
    pub fn weight_diagnostics(&self) -> Result<WeightDiagnostics> {
        let estimate = crate::integration::observation::observed_estimate(
            self.estimate(),
            self.stage,
            self.grid.is_none(),
        )?
        .0;
        let mut status = if self.stage == IntegrationStage::Pilot {
            Status::PilotOnly
        } else if self.grid.is_some() && self.records.is_empty() {
            Status::WaitingForCoverage
        } else {
            Status::Available
        };
        let mut group = WeightStatistics::new(WeightLayout::new(&self.problem).len());
        let mut count = 0u64;
        if self.stage == IntegrationStage::Production {
            for row in self.records.values() {
                count = count
                    .checked_add(row.task.points as u64)
                    .ok_or(IntegrationError::NumericRange)?;
                if let Some(stats) = &row.weights {
                    group.merge(stats)?;
                } else {
                    status = Status::LegacyUnavailable;
                }
            }
        }
        Ok(weights::report(
            &self.problem,
            estimate.as_ref(),
            status,
            count,
            &[group],
        ))
    }
}
