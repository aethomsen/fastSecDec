use serde::{Deserialize, Serialize};

use crate::kernel::{EvaluatorTiming, PrecisionClass, PrecisionReport, ReplayReport};

/// Caller-aggregated numerical kernel diagnostics. Evaluations include failed
/// attempts; conditioning checks and rescues count successful reports.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EvaluationDiagnostics {
    /// Present when worker contour diagnostics were recorded. Historical
    /// records without this field retain unknown rather than inferred counts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contour: Option<super::ContourEvaluationDiagnostics>,
    /// Optional operational work, separate from causal checks and accepted
    /// samples. Boxing preserves the small disabled/default diagnostics path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contour_runtime: Option<Box<super::ContourRuntimeDiagnostics>>,
    pub evaluations: u64,
    pub conditioning_checks: u64,
    pub rescues: u64,
    pub max_precision_bits: u32,
    pub failures: u64,
    pub weighted_checks: u64,
    pub additional_replays: u64,
    pub f64_points: u64,
    pub double_float_points: u64,
    pub arbitrary_points: u64,
    pub unstable_points: u64,
    pub cutoff_zero_points: u64,
    pub f64_timing: EvaluatorTiming,
    pub double_float_timing: EvaluatorTiming,
    pub arbitrary_timing: EvaluatorTiming,
    /// Existing native tracked-evaluator work, including algebraic root checks.
    /// A zero/default bucket can mean unrecorded or disabled, not measured zero.
    /// Omission preserves historical JSON checkpoint re-encoding and digests.
    #[serde(default, skip_serializing_if = "timing_is_empty")]
    pub conditioning_timing: EvaluatorTiming,
}

fn timing_is_empty(timing: &EvaluatorTiming) -> bool {
    *timing == EvaluatorTiming::default()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("evaluation diagnostics counter overflow")]
pub struct DiagnosticsOverflow;

impl std::fmt::Display for EvaluationDiagnostics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} evaluations; {} conditioning checks; {} rescues; {} weighted checks; {} additional replays; {} bits maximum; {} failures",
            self.evaluations,
            self.conditioning_checks,
            self.rescues,
            self.weighted_checks,
            self.additional_replays,
            self.max_precision_bits,
            self.failures
        )
    }
}

impl EvaluationDiagnostics {
    /// Incorporate one drained native worker delta without changing sampling.
    pub fn record_contour_runtime(
        &mut self,
        stage: super::IntegrationStage,
        report: &crate::contour::ContourRuntimeReport,
    ) -> Result<(), DiagnosticsOverflow> {
        let mut next = self.contour_runtime.clone().unwrap_or_default();
        next.record(stage, report)?;
        self.contour_runtime = Some(next);
        Ok(())
    }

    /// Incorporate one drained worker report, preserving sampling phase.
    pub fn record_contour(
        &mut self,
        stage: super::IntegrationStage,
        report: &crate::kernel::ContourProductionReport,
    ) -> Result<(), DiagnosticsOverflow> {
        let mut next = self.contour.unwrap_or_default();
        next.record(stage, report)?;
        self.contour = Some(next);
        Ok(())
    }

    pub fn record(&mut self, report: PrecisionReport) -> Result<(), DiagnosticsOverflow> {
        self.merge(&Self {
            evaluations: 1,
            conditioning_checks: u64::from(report.checked),
            rescues: u64::from(report.rescued),
            max_precision_bits: report.bits,
            failures: 0,
            f64_points: u64::from(report.class == PrecisionClass::F64),
            double_float_points: u64::from(report.class == PrecisionClass::DoubleFloat),
            arbitrary_points: u64::from(report.class == PrecisionClass::Arbitrary),
            unstable_points: u64::from(report.class == PrecisionClass::Unstable),
            cutoff_zero_points: u64::from(report.class == PrecisionClass::Unstable),
            f64_timing: report.timings.f64,
            double_float_timing: report.timings.double_float,
            arbitrary_timing: report.timings.arbitrary,
            conditioning_timing: report.timings.conditioning,
            ..Self::default()
        })
    }

    /// Record one sampled point once. A weighted check can already be satisfied
    /// by ordinary precision rescue; only an extra evaluation counts as replay.
    pub fn record_replay(&mut self, report: ReplayReport) -> Result<(), DiagnosticsOverflow> {
        self.merge(&Self {
            evaluations: 1,
            conditioning_checks: u64::from(report.precision.checked),
            rescues: u64::from(report.precision.rescued),
            max_precision_bits: report.precision.bits,
            weighted_checks: u64::from(report.weighted_check),
            additional_replays: u64::from(report.replayed),
            f64_points: u64::from(report.precision.class == PrecisionClass::F64),
            double_float_points: u64::from(report.precision.class == PrecisionClass::DoubleFloat),
            arbitrary_points: u64::from(report.precision.class == PrecisionClass::Arbitrary),
            unstable_points: u64::from(report.precision.class == PrecisionClass::Unstable),
            cutoff_zero_points: u64::from(report.precision.class == PrecisionClass::Unstable),
            f64_timing: report.precision.timings.f64,
            double_float_timing: report.precision.timings.double_float,
            arbitrary_timing: report.precision.timings.arbitrary,
            conditioning_timing: report.precision.timings.conditioning,
            ..Self::default()
        })
    }

    pub fn record_failure(&mut self) -> Result<(), DiagnosticsOverflow> {
        self.record_failure_with_timings(Default::default())
    }

    pub fn record_failure_with_timings(
        &mut self,
        timings: crate::kernel::EvaluationTimings,
    ) -> Result<(), DiagnosticsOverflow> {
        self.merge(&Self {
            evaluations: 1,
            failures: 1,
            unstable_points: 1,
            f64_timing: timings.f64,
            double_float_timing: timings.double_float,
            arbitrary_timing: timings.arbitrary,
            conditioning_timing: timings.conditioning,
            ..Self::default()
        })
    }

    pub fn classified_points(&self) -> u64 {
        self.f64_points
            .saturating_add(self.double_float_points)
            .saturating_add(self.arbitrary_points)
            .saturating_add(self.unstable_points)
    }

    /// Legacy records did not retain class counts; these remain explicitly unknown.
    pub fn unclassified_points(&self) -> u64 {
        self.evaluations.saturating_sub(self.classified_points())
    }

    /// Merge atomically: an overflow leaves all counters unchanged.
    pub fn merge(&mut self, other: &Self) -> Result<(), DiagnosticsOverflow> {
        let sum = |a: u64, b: u64| a.checked_add(b).ok_or(DiagnosticsOverflow);
        let timing = |a: EvaluatorTiming, b: EvaluatorTiming| {
            Ok::<_, DiagnosticsOverflow>(EvaluatorTiming {
                calls: sum(a.calls, b.calls)?,
                nanoseconds: sum(a.nanoseconds, b.nanoseconds)?,
                matrix_invocations: sum(a.matrix_invocations, b.matrix_invocations)?,
                matrix_points: sum(a.matrix_points, b.matrix_points)?,
            })
        };
        let contour = match (self.contour, other.contour) {
            (Some(mut a), Some(b)) => {
                a.merge(b)?;
                Some(a)
            }
            (a, b) => a.or(b),
        };
        let combined = Self {
            contour,
            contour_runtime: match (&self.contour_runtime, &other.contour_runtime) {
                (Some(a), Some(b)) => {
                    let mut next = a.clone();
                    next.merge(b)?;
                    Some(next)
                }
                (a, b) => a.as_ref().or(b.as_ref()).cloned(),
            },
            evaluations: sum(self.evaluations, other.evaluations)?,
            conditioning_checks: sum(self.conditioning_checks, other.conditioning_checks)?,
            rescues: sum(self.rescues, other.rescues)?,
            max_precision_bits: self.max_precision_bits.max(other.max_precision_bits),
            failures: sum(self.failures, other.failures)?,
            weighted_checks: sum(self.weighted_checks, other.weighted_checks)?,
            additional_replays: sum(self.additional_replays, other.additional_replays)?,
            f64_points: sum(self.f64_points, other.f64_points)?,
            double_float_points: sum(self.double_float_points, other.double_float_points)?,
            arbitrary_points: sum(self.arbitrary_points, other.arbitrary_points)?,
            unstable_points: sum(self.unstable_points, other.unstable_points)?,
            cutoff_zero_points: sum(self.cutoff_zero_points, other.cutoff_zero_points)?,
            f64_timing: timing(self.f64_timing, other.f64_timing)?,
            double_float_timing: timing(self.double_float_timing, other.double_float_timing)?,
            arbitrary_timing: timing(self.arbitrary_timing, other.arbitrary_timing)?,
            conditioning_timing: timing(self.conditioning_timing, other.conditioning_timing)?,
        };
        *self = combined;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_merge_success_failure_and_precision_without_partial_overflow() {
        let mut total = EvaluationDiagnostics::default();
        total
            .record(PrecisionReport {
                rescued: false,
                checked: false,
                bits: 53,
                ..Default::default()
            })
            .unwrap();
        let mut worker = EvaluationDiagnostics::default();
        worker
            .record(PrecisionReport {
                rescued: true,
                checked: true,
                bits: 512,
                class: crate::kernel::PrecisionClass::Arbitrary,
                ..Default::default()
            })
            .unwrap();
        worker.record_failure().unwrap();
        total.merge(&worker).unwrap();
        assert_eq!(
            total,
            EvaluationDiagnostics {
                evaluations: 3,
                conditioning_checks: 1,
                rescues: 1,
                max_precision_bits: 512,
                failures: 1,
                f64_points: 1,
                arbitrary_points: 1,
                unstable_points: 1,
                ..Default::default()
            }
        );
        let prior = total.clone();
        assert!(
            total
                .merge(&EvaluationDiagnostics {
                    failures: u64::MAX,
                    ..Default::default()
                })
                .is_err()
        );
        assert_eq!(total, prior);
    }

    #[test]
    fn weighted_checks_distinguish_existing_rescue_from_additional_replay() {
        let mut total = EvaluationDiagnostics::default();
        for replayed in [false, true] {
            total
                .record_replay(ReplayReport {
                    precision: PrecisionReport {
                        rescued: true,
                        checked: true,
                        bits: 256,
                        class: crate::kernel::PrecisionClass::Arbitrary,
                        ..Default::default()
                    },
                    weighted_check: true,
                    replayed,
                })
                .unwrap();
        }
        assert_eq!(total.evaluations, 2);
        assert_eq!(total.rescues, 2);
        assert_eq!(total.weighted_checks, 2);
        assert_eq!(total.additional_replays, 1);
        let prior = total.clone();
        assert!(
            total
                .merge(&EvaluationDiagnostics {
                    additional_replays: u64::MAX,
                    ..Default::default()
                })
                .is_err()
        );
        assert_eq!(total, prior);
        let old: EvaluationDiagnostics = serde_json::from_str("{\"evaluations\":1}").unwrap();
        assert!(old.contour.is_none());
        assert_eq!(old.weighted_checks, 0);
        assert_eq!(old.additional_replays, 0);
    }
}

#[cfg(test)]
mod timing_tests;
