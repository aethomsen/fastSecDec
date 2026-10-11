#[cfg(test)]
mod contour_runtime_tests;
#[cfg(test)]
mod contour_tests;
mod discrete_mc;
mod mc;
mod observations;
mod qmc;
use super::{
    IntegrationReport,
    checkpoint::{RestoredCheckpoint, restore_checkpoint},
    replay::AcceptedReplay,
};
use crate::{CliResult, artifact::Artifact, config::IntegrationInput, display::Dashboard};
use fastsecdec::{
    integration::{IntegrationError, IntegrationProblem, Tolerance},
    kernel::{KernelSet, ReplayState, WeightedEvaluationContext},
    status::{ContourCheckpointProvenance, ContourPilotProvenance, EvaluationDiagnostics},
};
use std::{path::Path, time::Instant};

struct Context<'a> {
    artifact: &'a Artifact,
    kernels: &'a KernelSet,
    settings: &'a IntegrationInput,
    checkpoint: &'a Path,
    dashboard: &'a mut Dashboard,
    pool: rayon::ThreadPool,
    problem: IntegrationProblem,
    tolerance: Tolerance,
    started: Instant,
    last_checkpoint: Instant,
    restored: Option<RestoredCheckpoint>,
    contour_provenance: Option<ContourCheckpointProvenance>,
    diagnostics: EvaluationDiagnostics,
    replay: AcceptedReplay,
    operations: observations::Operations,
}

/// Snapshot/reduction is observational and can be expensive for many sectors.
/// Package acceptance and cancellation remain checked by the caller at each
/// completion and bounded wait poll (or each batch for the MC path).
fn observe(
    dashboard: &mut Dashboard,
    operations: &observations::Operations,
    diagnostics: &EvaluationDiagnostics,
    started: Instant,
    force: bool,
    snapshot: impl FnOnce() -> Result<fastsecdec::integration::IntegrationObservation, IntegrationError>,
    failure: &mut Option<String>,
) -> CliResult<()> {
    if dashboard.integration_due(started.elapsed(), force) {
        dashboard.set_operational(operations.snapshot()?);
        match snapshot() {
            Ok(mut observation) => {
                observation.snapshot =
                    super::report::with_diagnostics(observation.snapshot, diagnostics);
                dashboard.integration_observation(&observation, started.elapsed().as_secs_f64())?;
            }
            Err(error) => {
                failure.get_or_insert_with(|| error.to_string());
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn observe_live(
    dashboard: &mut Dashboard,
    operations: &observations::Operations,
    diagnostics: &EvaluationDiagnostics,
    started: Instant,
    force: bool,
    live: impl FnOnce() -> Result<fastsecdec::integration::LiveObservation, IntegrationError>,
    snapshot: impl FnOnce() -> Result<fastsecdec::integration::IntegrationObservation, IntegrationError>,
    failure: &mut Option<String>,
) -> CliResult<()> {
    if !dashboard.integration_due(started.elapsed(), force) {
        return Ok(());
    }
    dashboard.set_operational(operations.snapshot()?);
    match live() {
        Ok(live) => dashboard.set_live_observation(Some(live)),
        Err(error) if error.is_statistical_range() => dashboard.set_live_observation(None),
        Err(error) => {
            failure.get_or_insert_with(|| error.to_string());
        }
    }
    match snapshot() {
        Ok(mut value) => {
            value.snapshot = super::report::with_diagnostics(value.snapshot, diagnostics);
            dashboard.integration_observation(&value, started.elapsed().as_secs_f64())?;
        }
        Err(error) => {
            failure.get_or_insert_with(|| error.to_string());
        }
    }
    Ok(())
}

fn final_report(
    dashboard: &mut Dashboard,
    mut report: IntegrationReport,
) -> CliResult<IntegrationReport> {
    report.process_cpu_seconds = dashboard.process_cpu_seconds();
    dashboard.set_live_observation(None);
    dashboard.set_operational(report.operational.clone());
    dashboard.integration_due(
        std::time::Duration::from_secs_f64(report.elapsed_seconds),
        true,
    );
    dashboard.integration_observation(
        &fastsecdec::integration::IntegrationObservation {
            snapshot: report.snapshot.clone(),
            contributions: report.contributions.clone(),
            weight_diagnostics: report.weight_diagnostics.clone(),
        },
        report.elapsed_seconds,
    )?;
    Ok(report)
}

#[cfg(test)]
fn evaluate_tracked(
    kernel: &mut WeightedEvaluationContext,
    point: &[f64],
    weight: f64,
    output: &mut [f64],
    diagnostics: &mut EvaluationDiagnostics,
) -> Result<(), String> {
    match kernel.evaluate_weighted(point, weight, output) {
        Ok(report) => diagnostics
            .record_replay(report)
            .map_err(|error| error.to_string()),
        Err(error) => {
            diagnostics
                .record_failure()
                .map_err(|error| error.to_string())?;
            Err(error.to_string())
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn evaluate_batch_observed(
    kernel: &mut WeightedEvaluationContext,
    sector: u64,
    stage: fastsecdec::status::IntegrationStage,
    points: &[f64],
    weights: &[f64],
    output: &mut [f64],
    diagnostics: &mut EvaluationDiagnostics,
    meter: &observations::WorkerMeter,
    stop: &std::sync::atomic::AtomicBool,
    aborted: &mut bool,
) -> Result<(), String> {
    let before = kernel.evaluation_metrics();
    let started = Instant::now();
    let result = kernel.evaluate_weighted_batch_controlled(points, weights, output, || {
        stop.load(std::sync::atomic::Ordering::Relaxed)
    });
    let elapsed = started.elapsed().as_secs_f64();
    let timings = kernel.evaluation_metrics().since(before);
    let reports = match &result {
        Ok(reports) => reports,
        Err(failure) => &failure.completed,
    };
    let mut local = EvaluationDiagnostics::default();
    // Mode is immutable for this worker invocation. Failed/discarded batches
    // still performed work; drain before any adaptation/production transition.
    if kernel.contour_diagnostics_mode() == fastsecdec::contour::ContourDiagnosticsMode::Aggregate
        && let Some(report) = kernel
            .take_contour_runtime_report()
            .map_err(|e| e.to_string())?
    {
        local
            .record_contour_runtime(stage, &report)
            .map_err(|e| e.to_string())?;
    }
    if let Some(report) = kernel.take_contour_validation_report() {
        local
            .record_contour(stage, &report)
            .map_err(|e| e.to_string())?;
    }
    for report in reports {
        local.record_replay(*report).map_err(|e| e.to_string())?;
    }
    let cancelled = result
        .as_ref()
        .is_err_and(|e| matches!(e.error, fastsecdec::kernel::KernelError::Cancelled));
    *aborted |= cancelled;
    if result.is_err() && !cancelled {
        local.record_failure().map_err(|e| e.to_string())?;
    }
    // Matrix primaries can include rows beyond a later failed point. Account
    // actual attempted native calls once, independently of classified points.
    local.f64_timing = timings.f64;
    local.double_float_timing = timings.double_float;
    local.arbitrary_timing = timings.arbitrary;
    meter.record_batch(
        sector,
        elapsed,
        timings.total_nanoseconds() as f64 / 1e9,
        &local,
        &output[..reports.len() * kernel.output_count()],
    )?;
    diagnostics.merge(&local).map_err(|e| e.to_string())?;
    result.map(|_| ()).map_err(|e| e.error.to_string())
}

/// Evaluation failure or rejected numerical submission must not advance the
/// accepted replay envelope. Validate metadata before either accepted update.
pub(super) fn submit_package<T>(
    result: Result<T, IntegrationError>,
    sector: usize,
    state: Option<ReplayState>,
    replay: &mut AcceptedReplay,
    submit: impl FnOnce(T) -> Result<(), IntegrationError>,
) -> CliResult<()> {
    let result = result?;
    let state = state.ok_or("successful work package has no replay state")?;
    replay.validate_candidate(sector, &state)?;
    submit(result)?;
    replay.accept(sector, &state)
}

pub(super) fn problem(
    artifact: &Artifact,
    kernels: &KernelSet,
    scope: &fastsecdec::results::ResultScope,
) -> CliResult<IntegrationProblem> {
    Ok(
        fastsecdec::results::KernelResultManifest::from_kernels(kernels).integration_problem(
            scope,
            if artifact.catalogue().is_some() {
                format!(
                    "{}:{}:{}",
                    artifact.content_id,
                    artifact.kernel_content_id,
                    kernels.content_id()
                )
            } else if kernels.runtime_parameters().is_empty() {
                artifact.content_id.clone()
            } else {
                format!("{}:{}", artifact.content_id, kernels.content_id())
            },
        )?,
    )
}

#[cfg(test)]
pub fn integrate(
    artifact: &Artifact,
    kernels: &KernelSet,
    settings: &IntegrationInput,
    checkpoint: &Path,
    resume: bool,
    dashboard: &mut Dashboard,
) -> CliResult<IntegrationReport> {
    integrate_with_pilot(
        artifact,
        kernels,
        settings,
        checkpoint,
        resume,
        dashboard,
        None,
        EvaluationDiagnostics::default(),
    )
}

/// The pilot remains separate from production statistics and RNG state. Earlier
/// checkpoint evidence is retained even when this invocation disables checking.
#[allow(clippy::too_many_arguments)]
pub fn integrate_with_pilot(
    artifact: &Artifact,
    kernels: &KernelSet,
    settings: &IntegrationInput,
    checkpoint: &Path,
    resume: bool,
    dashboard: &mut Dashboard,
    pilot: Option<ContourPilotProvenance>,
    initial_diagnostics: EvaluationDiagnostics,
) -> CliResult<IntegrationReport> {
    settings.validate_execution()?;
    let problem = problem(artifact, kernels, &settings.scope)?;
    settings.accuracy_target.validate_layout(&problem.orders)?;
    dashboard.set_scope(settings.scope.clone());
    let tolerance = Tolerance::new(settings.absolute_tolerance, settings.relative_tolerance)?;
    let started = Instant::now();
    dashboard.begin_integration();
    dashboard.set_stability_mode(settings.stability.mode);
    dashboard.set_target_order(match settings.accuracy_target {
        fastsecdec::integration::AccuracyTarget::AllComponents => None,
        fastsecdec::integration::AccuracyTarget::LaurentOrder(order) => Some(order),
    });
    let last_checkpoint = Instant::now();
    let operations = observations::Operations::new(
        settings.workers,
        &problem.orders,
        dashboard.integration_interval(),
    );
    operations.record_preparation(&initial_diagnostics)?;
    let setup = operations.coordinator(false);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(settings.workers)
        .build()?;
    let method = settings.method.replace('-', "_");
    let restored = if resume {
        Some(restore_checkpoint(checkpoint, artifact, settings)?)
    } else {
        None
    };
    let mut diagnostics = restored
        .as_ref()
        .map(|checkpoint| checkpoint.diagnostics.clone())
        .unwrap_or_default();
    diagnostics.merge(&initial_diagnostics)?;
    let contour_provenance =
        (settings.contour.deformation != fastsecdec::contour::ContourMode::Off).then(|| {
            ContourCheckpointProvenance::update(
                restored
                    .as_ref()
                    .and_then(|checkpoint| checkpoint.contour.as_ref()),
                &settings.contour.validation,
                pilot,
            )
        });
    let replay = if let Some(checkpoint) = &restored {
        checkpoint.replay.validate(kernels, &settings.replay)?;
        checkpoint.replay.clone()
    } else {
        AcceptedReplay::new(kernels, settings.replay.clone())?
    };
    drop(setup);
    let context = Context {
        artifact,
        kernels,
        settings,
        checkpoint,
        dashboard,
        pool,
        problem,
        tolerance,
        started,
        last_checkpoint,
        restored,
        contour_provenance: contour_provenance.clone(),
        diagnostics,
        replay,
        operations,
    };
    let mut report = match method.as_str() {
        "mc" | "adaptive_mc" => mc::run(context, &method),
        "discrete_mc" => discrete_mc::run(context),
        "qmc" | "adaptive_qmc" => qmc::run(context, &method),
        _ => Err(
            "integration method must be qmc, adaptive_qmc, mc, adaptive_mc or discrete_mc".into(),
        ),
    }?;
    if let Some(provenance) = contour_provenance {
        report.set_contour_provenance(&settings.contour, provenance.pilots);
    }
    Ok(report)
}
