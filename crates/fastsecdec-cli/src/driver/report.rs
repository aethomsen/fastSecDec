use fastsecdec::{
    integration::{
        ContributionReport, IntegrationObservation, QmcDesign, Tolerance, VectorEstimate,
    },
    status::{EvaluationDiagnostics, GenerationTimings, IntegrationSnapshot, StoppingReason},
};
use serde::Serialize;

#[derive(Serialize)]
pub struct IntegrationReport {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_selection: Option<fastsecdec::generation::SourceSectorSelection>,
    /// Independent validation evidence and actual worker-owned check counters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contour: Option<fastsecdec::status::ContourRunReport>,
    pub stability_mode: fastsecdec::kernel::StabilityMode,
    pub process_cpu_seconds: Option<f64>,
    pub accuracy_target: fastsecdec::integration::AccuracyTarget,
    pub operational: fastsecdec::integration::OperationalMetrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timed_stop: Option<super::timing::TimedStopReport>,
    pub content_id: String,
    pub elapsed_seconds: f64,
    pub loading_seconds: f64,
    pub generation_timings: Option<GenerationTimings>,
    /// Only a full-integral target can establish unqualified convergence.
    pub converged: bool,
    pub scoped_target_reached: bool,
    pub scope: fastsecdec::results::ResultScope,
    pub stopping_reason: String,
    pub estimate: Option<VectorEstimate>,
    pub snapshot: IntegrationSnapshot,
    pub resume_status: ResumeStatus,
    /// Effective final-round design; adaptive sector allocations are authoritative.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qmc_design: Option<QmcDesign>,
    /// Earlier completed design while the current refinement is unfinished.
    /// Its statistics are separate and never determine current convergence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_complete: Option<super::refinement::PreviousProduction>,
    /// Native accepted-sector statistics, with authoritative total covariance.
    pub contributions: ContributionReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight_diagnostics: Option<fastsecdec::integration::WeightDiagnostics>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResumeStatus {
    CheckpointSaved,
    /// Havana does not export mutable pilot training state. Restart the pilot.
    PilotRestartRequired,
}

pub(super) fn with_diagnostics(
    mut snapshot: IntegrationSnapshot,
    diagnostics: &EvaluationDiagnostics,
) -> IntegrationSnapshot {
    snapshot.evaluation_diagnostics = Some(diagnostics.clone());
    snapshot
}

pub(crate) struct ExecutionOutcome {
    pub stability_mode: fastsecdec::kernel::StabilityMode,
    pub accuracy_target: fastsecdec::integration::AccuracyTarget,
    pub operational: fastsecdec::integration::OperationalMetrics,
    pub scope: fastsecdec::results::ResultScope,
    pub cancelled: bool,
    pub failure: Option<String>,
    pub resume_status: ResumeStatus,
    pub qmc_design: Option<QmcDesign>,
    pub timed_stop: Option<super::timing::TimedStopReport>,
}

pub(crate) fn finish(
    artifact: &crate::artifact::Artifact,
    observation: IntegrationObservation,
    diagnostics: &EvaluationDiagnostics,
    tolerance: Tolerance,
    elapsed_seconds: f64,
    mut outcome: ExecutionOutcome,
) -> crate::CliResult<IntegrationReport> {
    let mut snapshot = with_diagnostics(observation.snapshot, diagnostics);
    if let fastsecdec::status::UncertaintyStatus::StatisticalFailure { reason } =
        &snapshot.uncertainty
    {
        outcome.failure.get_or_insert_with(|| reason.clone());
    }
    if (outcome.failure.is_some() || outcome.cancelled)
        && let Some(timed_stop) = &mut outcome.timed_stop
    {
        timed_stop.counts_complete = false;
    }
    let scoped_target_reached = outcome.failure.is_none()
        && !outcome.cancelled
        && outcome
            .timed_stop
            .as_ref()
            .is_none_or(|stop| !stop.deadline_reached())
        && snapshot
            .estimate
            .as_ref()
            .map(|estimate| estimate.meets_target(outcome.accuracy_target, tolerance))
            .transpose()?
            .unwrap_or(false);
    let (stop, stopping_reason) = if let Some(message) = outcome.failure {
        (
            StoppingReason::NumericalFailure(message.clone()),
            format!("numerical failure: {message}"),
        )
    } else if outcome.cancelled {
        (
            StoppingReason::Cancelled,
            if outcome.resume_status == ResumeStatus::PilotRestartRequired {
                "cancelled during MC pilot; restart the pilot to continue"
            } else {
                "cancelled"
            }
            .into(),
        )
    } else if scoped_target_reached {
        (StoppingReason::TargetReached, "accuracy reached".into())
    } else {
        (StoppingReason::WorkLimit, "work limit".into())
    };
    snapshot.stop_reason = Some(stop);
    Ok(IntegrationReport {
        source_selection: artifact
            .catalogue()
            .and_then(|catalogue| catalogue.source_selection().cloned()),
        contour: None,
        stability_mode: outcome.stability_mode,
        process_cpu_seconds: None,
        accuracy_target: outcome.accuracy_target,
        operational: outcome.operational,
        timed_stop: outcome.timed_stop,
        content_id: artifact.content_id.clone(),
        elapsed_seconds,
        loading_seconds: artifact.loading_seconds,
        generation_timings: artifact.generation_timings.clone(),
        converged: scoped_target_reached && outcome.scope.is_full_integral(),
        scoped_target_reached,
        scope: outcome.scope,
        stopping_reason,
        estimate: snapshot.estimate.clone(),
        snapshot,
        resume_status: outcome.resume_status,
        qmc_design: outcome.qmc_design,
        previous_complete: None,
        contributions: observation.contributions,
        weight_diagnostics: observation.weight_diagnostics,
    })
}

impl IntegrationReport {
    /// Call after collecting ordinary or serial pilot owners. Production counts
    /// come from workers, never from an unused coordinator evaluator clone.
    pub fn set_contour_provenance(
        &mut self,
        settings: &fastsecdec::contour::ContourSettings,
        pilots: Vec<fastsecdec::status::ContourPilotProvenance>,
    ) {
        let previous =
            self.contour
                .take()
                .map(|report| fastsecdec::status::ContourCheckpointProvenance {
                    validation: report.validation,
                    pilots: report.pilots,
                });
        let evidence = fastsecdec::status::ContourCheckpointProvenance::update(
            previous.as_ref(),
            &settings.validation,
            pilots,
        );
        self.contour = (settings.deformation != fastsecdec::contour::ContourMode::Off).then(|| {
            fastsecdec::status::ContourRunReport::new(
                settings,
                evidence.pilots,
                &self.operational.diagnostics,
                self.snapshot
                    .evaluation_diagnostics
                    .as_ref()
                    .unwrap_or(&EvaluationDiagnostics::default()),
            )
        });
    }

    pub fn failed(&self) -> bool {
        matches!(
            self.snapshot.stop_reason,
            Some(StoppingReason::NumericalFailure(_))
        )
    }
}
