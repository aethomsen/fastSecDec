use super::*;
use crate::integration::{
    WeightDiagnosticStatus as Status, WeightDiagnostics,
    weights::{self, WeightLayout, WeightStatistics},
};
impl QmcSession {
    pub fn weight_diagnostics(&self) -> Result<WeightDiagnostics> {
        let estimate = super::results::observation_estimate(self)?;
        let mut status = if self.stage == IntegrationStage::Pilot {
            Status::PilotOnly
        } else {
            Status::Available
        };
        let ids: Vec<_> = self
            .runs
            .iter()
            .map(|r| r.accumulator.complete_shift_ids())
            .collect();
        let common: BTreeSet<_> = ids
            .first()
            .into_iter()
            .flatten()
            .copied()
            .filter(|id| ids.iter().all(|r| r.binary_search(id).is_ok()))
            .collect();
        let mut groups = Vec::new();
        let mut count = 0u64;
        if self.stage == IntegrationStage::Production {
            for (run, ids) in self.runs.iter().zip(ids) {
                let selected: BTreeSet<_> = ids
                    .into_iter()
                    .filter(|id| {
                        self.method != IntegrationMethod::DemocraticQmc || common.contains(id)
                    })
                    .collect();
                let points = (selected.len() as u64)
                    .checked_mul(run.accumulator.plan().rule().points())
                    .ok_or(IntegrationError::NumericRange)?;
                count = count
                    .checked_add(points)
                    .ok_or(IntegrationError::NumericRange)?;
                if selected.is_empty() && status == Status::Available {
                    status = Status::WaitingForCoverage;
                }
                let Some(rows) = &run.weights else {
                    status = Status::LegacyUnavailable;
                    continue;
                };
                let mut stats = WeightStatistics::new(WeightLayout::new(&self.problem).len());
                for partial in rows.values() {
                    for (i, row) in partial.groups.iter().enumerate() {
                        if selected.contains(&(partial.first_shift as usize + i)) {
                            stats.merge(row)?;
                        }
                    }
                }
                stats.validate(WeightLayout::new(&self.problem).len(), points)?;
                groups.push(stats);
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
