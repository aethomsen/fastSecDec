//! Compact artifact binding and presentation around the CLI-owned serial driver.
use crate::{
    CliResult, artifact::Artifact, config::IntegrationInput, display::Dashboard, driver,
    reference::PreparedReference,
};
use fastsecdec::{
    integration::Tolerance, kernel::KernelLoadOptions, results::KernelResultManifest,
};
use std::{path::Path, time::Instant};

#[allow(clippy::too_many_arguments)]
pub(crate) fn integrate(
    path: &Path,
    artifact: &Artifact,
    settings: &mut IntegrationInput,
    checkpoint: &Path,
    resume: bool,
    options: KernelLoadOptions,
    reference: Option<&PreparedReference>,
    dashboard: &mut Dashboard,
) -> CliResult<(driver::IntegrationReport, KernelResultManifest)> {
    let catalogue = artifact.catalogue().ok_or(
        "serial integration requires an indexed artifact; regenerate this integral with the current fastsecdec generate command (ordinary or --serial)",
    )?;
    if !artifact.dependencies_compatible() {
        return Err(
            "artifact dependency versions differ; regenerate it with this FastSecDec build".into(),
        );
    }
    let loading = Instant::now();
    dashboard.begin_loading();
    dashboard.loading(&crate::loading::Snapshot {
        phase: crate::loading::Phase::Metadata,
        completed: None,
        total: None,
        elapsed_seconds: 0.,
        primary_evaluators: None,
    })?;
    // Native state imports may retain Symbolica polynomial-variable tables.
    // A disposable child binds the offsets and returns only this compact vector.
    let setup = crate::isolated::exact(path, artifact, settings, options, dashboard)?;
    let manifest = setup.manifest;
    let contour_pilots = setup.contour_pilots;
    manifest.validate()?;
    settings.scope = manifest.canonical_scope(&settings.scope)?;
    if let Some(reference) = reference {
        reference.validate_identity(&manifest.kernel_content_id)?;
    }
    let identity = format!(
        "{}:{}:{}",
        artifact.content_id, catalogue.content_id, manifest.kernel_content_id
    );
    let problem = manifest.integration_problem(&settings.scope, identity)?;
    settings.accuracy_target.validate_layout(&problem.orders)?;
    let loading_seconds = loading.elapsed().as_secs_f64();
    dashboard.loading(&crate::loading::Snapshot {
        phase: crate::loading::Phase::Complete,
        completed: None,
        total: None,
        elapsed_seconds: loading_seconds,
        primary_evaluators: None,
    })?;
    dashboard.begin_integration();
    dashboard.set_scope(settings.scope.clone());
    dashboard.set_stability_mode(settings.stability.mode);
    dashboard.set_target_order(match settings.accuracy_target {
        fastsecdec::integration::AccuracyTarget::AllComponents => None,
        fastsecdec::integration::AccuracyTarget::LaurentOrder(order) => Some(order),
    });
    let outcome = driver::serial::integrate(
        artifact,
        path,
        problem,
        settings,
        checkpoint,
        resume,
        options,
        contour_pilots,
        setup.diagnostics,
        |snapshot| {
            dashboard.worker_processes(
                &snapshot
                    .residents
                    .iter()
                    .map(|row| row.pid)
                    .collect::<Vec<_>>(),
            );
            dashboard.set_operational(snapshot.operational.clone());
            dashboard.serial_execution(snapshot);
            let mut observation = snapshot.observation.clone();
            observation.snapshot.evaluation_diagnostics = Some(snapshot.diagnostics.clone());
            dashboard.integration_observation(&observation, snapshot.elapsed_seconds)?;
            Ok(dashboard.cancelled())
        },
    )?;
    dashboard.worker_processes(&[]);
    let contour_pilots = outcome.contour_pilots;
    let mut report = driver::report::finish(
        artifact,
        outcome.snapshot.observation,
        &outcome.snapshot.diagnostics,
        Tolerance::new(settings.absolute_tolerance, settings.relative_tolerance)?,
        outcome.snapshot.elapsed_seconds,
        driver::report::ExecutionOutcome {
            stability_mode: settings.stability.mode,
            accuracy_target: settings.accuracy_target,
            operational: outcome.snapshot.operational,
            scope: settings.scope.clone(),
            cancelled: outcome.cancelled,
            failure: outcome.failure,
            resume_status: driver::ResumeStatus::CheckpointSaved,
            qmc_design: None,
            timed_stop: outcome.timed_stop,
        },
    )?;
    report.loading_seconds = loading_seconds;
    report.process_cpu_seconds = dashboard.process_cpu_seconds();
    report.set_contour_provenance(&settings.contour, contour_pilots);
    Ok((report, manifest))
}
