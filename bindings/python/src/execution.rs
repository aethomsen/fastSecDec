//! Common native problem/precision ownership for caller-stepped execution lanes.
use fastsecdec::{
    integration::IntegrationProblem,
    kernel::{KernelSet, ReplayPolicy, ReplayState},
    results::{KernelResultManifest, ResultScope},
};
use pyo3::prelude::*;

/// Shared binding boundary for native vector batches. Python signals stop at
/// point-routing boundaries; speculative matrix attempts remain diagnostics,
/// while the session admits only whole successful statistical packages.
#[allow(clippy::too_many_arguments)]
pub(crate) fn evaluate_batch(
    py: Python<'_>,
    context: &mut fastsecdec::kernel::WeightedEvaluationContext,
    stage: fastsecdec::status::IntegrationStage,
    points: &[f64],
    weights: &[f64],
    output: &mut [f64],
    diagnostics: &mut fastsecdec::status::EvaluationDiagnostics,
    interrupted: &mut Option<PyErr>,
) -> Result<(), String> {
    let before = context.evaluation_metrics();
    let result = context.evaluate_weighted_batch_controlled(points, weights, output, || {
        if interrupted.is_some() {
            return true;
        }
        if let Err(e) = py.check_signals() {
            *interrupted = Some(e);
            return true;
        }
        false
    });
    let timings = context.evaluation_metrics().since(before);
    let reports = match &result {
        Ok(reports) => reports,
        Err(failure) => &failure.completed,
    };
    let mut local = fastsecdec::status::EvaluationDiagnostics::default();
    if let Some(report) = context.take_contour_validation_report() {
        local
            .record_contour(stage, &report)
            .map_err(|e| e.to_string())?;
    }
    if context.contour_diagnostics_mode() == fastsecdec::contour::ContourDiagnosticsMode::Aggregate
        && let Some(report) = context
            .take_contour_runtime_report()
            .map_err(|e| e.to_string())?
    {
        local
            .record_contour_runtime(stage, &report)
            .map_err(|e| e.to_string())?;
    }
    for report in reports {
        local.record_replay(*report).map_err(|e| e.to_string())?;
    }
    if result
        .as_ref()
        .is_err_and(|e| !matches!(e.error, fastsecdec::kernel::KernelError::Cancelled))
    {
        local.record_failure().map_err(|e| e.to_string())?;
    }
    local.f64_timing = timings.f64;
    local.double_float_timing = timings.double_float;
    local.arbitrary_timing = timings.arbitrary;
    local.conditioning_timing = timings.conditioning;
    diagnostics.merge(&local).map_err(|e| e.to_string())?;
    result.map(|_| ()).map_err(|e| e.error.to_string())
}

pub(crate) fn problem(py: Python<'_>, kernels: &KernelSet) -> PyResult<IntegrationProblem> {
    KernelResultManifest::integration_problem_from_kernels(
        kernels,
        &ResultScope::FullIntegral,
        kernels.content_id(),
    )
    .map_err(|e| crate::error::native(py, "configuration", e))
}

pub(crate) fn replay_states(
    py: Python<'_>,
    kernels: &KernelSet,
    policy: &ReplayPolicy,
) -> PyResult<Vec<ReplayState>> {
    (0..kernels.sectors().len())
        .map(|sector| kernels.replay_state(sector, policy.clone()))
        .collect::<Result<_, _>>()
        .map_err(|e| crate::error::native(py, "configuration", e))
}
