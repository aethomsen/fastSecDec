use super::*;
use crate::integration::{WeightDiagnosticStatus as Status, WeightDiagnostics, weights};
impl HavanaSession {
    pub fn weight_diagnostics(&self) -> Result<WeightDiagnostics> {
        let estimate = crate::integration::observation::observed_estimate(
            self.estimate(),
            self.stage,
            self.runs.is_empty(),
        )?
        .0;
        let mut status = if self.stage == IntegrationStage::Pilot {
            Status::PilotOnly
        } else {
            Status::Available
        };
        let mut groups = Vec::new();
        let mut count = 0u64;
        if self.stage == IntegrationStage::Production {
            for run in &self.runs {
                if run.records.is_empty() && status == Status::Available {
                    status = Status::WaitingForCoverage;
                }
                let mut group = WeightStatistics::new(WeightLayout::new(&self.problem).len());
                for row in run.records.values() {
                    count = count
                        .checked_add(row.task.points as u64)
                        .ok_or(IntegrationError::NumericRange)?;
                    if let Some(stats) = &row.weights {
                        group.merge(stats)?;
                    } else {
                        status = Status::LegacyUnavailable;
                    }
                }
                groups.push(group);
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
