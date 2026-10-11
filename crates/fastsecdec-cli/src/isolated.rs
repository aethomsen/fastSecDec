//! Recyclable setup work: native global state never survives into serial sampling.
use crate::{
    CliResult,
    artifact::Artifact,
    config::IntegrationInput,
    display::Dashboard,
    process::{
        ProcessPool,
        child::{self, WorkerCommand, WorkerEvent},
    },
};
use fastsecdec::{
    integration::SectorSpec, kernel::KernelLoadOptions, results::KernelResultManifest,
    status::GenerationSnapshot,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

#[cfg(test)]
mod parameter_tests;

#[derive(Serialize, Deserialize)]
enum Request {
    Exact {
        artifact: PathBuf,
        expected: String,
        settings: Box<IntegrationInput>,
        validate: bool,
        result: PathBuf,
    },
    Generate {
        input: PathBuf,
        output: PathBuf,
        workers: usize,
        reference: Option<PathBuf>,
        #[serde(default)]
        overrides: crate::config::GenerationOverrides,
    },
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) enum Progress {
    Generation {
        snapshot: Box<GenerationSnapshot>,
        workers: Option<crate::generate::dispatch::Progress>,
    },
    Contour {
        snapshot: crate::contour_pilot::Snapshot,
        elapsed_seconds: f64,
    },
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ExactSetup {
    pub manifest: KernelResultManifest,
    pub contour_pilots: Vec<fastsecdec::status::ContourPilotProvenance>,
    #[serde(default)]
    pub diagnostics: fastsecdec::status::EvaluationDiagnostics,
}

pub(crate) fn exact(
    path: &Path,
    artifact: &Artifact,
    settings: &IntegrationInput,
    options: KernelLoadOptions,
    dashboard: &mut Dashboard,
) -> CliResult<ExactSetup> {
    let staging = tempfile::tempdir()?;
    let result = staging.path().join("manifest.json");
    run(
        Request::Exact {
            artifact: path.into(),
            expected: artifact.kernel_content_id.clone(),
            settings: Box::new(settings.clone()),
            validate: options.validate,
            result: result.clone(),
        },
        dashboard,
    )?;
    let setup: ExactSetup = serde_json::from_reader(File::open(result)?)?;
    setup.manifest.validate()?;
    Ok(setup)
}
pub(crate) fn generate(
    input: &Path,
    output: &Path,
    workers: usize,
    reference: Option<&Path>,
    overrides: crate::config::GenerationOverrides,
    dashboard: &mut Dashboard,
) -> CliResult<()> {
    run(
        Request::Generate {
            input: input.into(),
            output: output.into(),
            workers,
            reference: reference.map(Path::to_owned),
            overrides,
        },
        dashboard,
    )
}

fn run(request: Request, dashboard: &mut Dashboard) -> CliResult<()> {
    let staging = tempfile::tempdir()?;
    let path = staging.path().join("request.json");
    crate::artifact::atomic_write(&path, &serde_json::to_vec(&request)?)?;
    let run = format!("setup-{}-{:?}", std::process::id(), Instant::now());
    let pool = ProcessPool::new(1)?;
    let mut command = Command::new(crate::process::executable()?);
    command.args(["__setup-worker", "--run-id", &run, "--lease-id", "0"]);
    let mut worker = pool.spawn_native::<WorkerEvent<Progress>>(
        &mut command,
        run,
        0,
        &staging.path().join("worker.log"),
    )?;
    dashboard.worker_processes(&[worker.pid()]);
    let outcome = (|| {
        let mut ready = false;
        let mut finished = false;
        loop {
            if dashboard.cancelled() {
                return Err("setup cancelled".into());
            }
            while let Some(event) = worker.poll()? {
                match event {
                    WorkerEvent::Ready { pid, build } => {
                        if ready || pid != worker.pid() || build != child::build_identity() {
                            return Err("setup worker identity differs".into());
                        }
                        ready = true;
                        worker.send(WorkerCommand::Execute(path.clone()))?;
                    }
                    WorkerEvent::Update(progress) => {
                        if !ready || finished {
                            return Err("unissued setup progress".into());
                        }
                        match progress {
                            Progress::Generation { snapshot, workers } => {
                                if let Some(workers) = workers {
                                    dashboard.generation_workers(&workers);
                                }
                                dashboard.generation(&snapshot)?;
                            }
                            Progress::Contour {
                                snapshot,
                                elapsed_seconds,
                            } => {
                                if snapshot.completed == 0 {
                                    dashboard.begin_loading();
                                }
                                dashboard.loading(&crate::loading::Snapshot {
                                    phase: crate::loading::Phase::ContourValidation,
                                    completed: Some(snapshot.completed),
                                    total: Some(snapshot.total),
                                    elapsed_seconds,
                                    primary_evaluators: None,
                                })?;
                            }
                        }
                    }
                    WorkerEvent::Finished => {
                        if !ready || finished {
                            return Err("duplicate setup completion".into());
                        }
                        finished = true;
                        worker.send(WorkerCommand::<PathBuf>::Shutdown)?;
                    }
                    WorkerEvent::Failed { message } => return Err(message.into()),
                }
            }
            if let Some(exit) = worker.try_wait()? {
                if !exit.success() {
                    return Err(format!("setup worker exited: {exit}").into());
                }
                if finished {
                    return Ok(());
                }
                if worker.output_closed() {
                    return Err("setup worker exited without a receipt".into());
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    })();
    // Reap before either returning its compact result or starting sampling.
    worker.terminate()?;
    dashboard.worker_processes(&[]);
    outcome
}

pub(crate) fn worker(run: String, lease: u64) -> CliResult<()> {
    child::serve::<PathBuf, Progress>(run, lease, |path, emit| {
        execute(path, emit).map_err(|error| std::io::Error::other(error.to_string()))
    })?;
    Ok(())
}
fn execute(path: PathBuf, emit: &mut dyn FnMut(Progress) -> std::io::Result<()>) -> CliResult<()> {
    match serde_json::from_reader(File::open(path)?)? {
        Request::Exact {
            artifact: path,
            expected,
            settings,
            validate,
            result,
        } => {
            let mut artifact =
                Artifact::load_metadata_with_options(&path, KernelLoadOptions { validate })?;
            crate::contour_cli::select_program(&mut artifact, &settings.contour)?;
            let catalogue = artifact
                .catalogue()
                .ok_or("setup requires an indexed artifact")?;
            if artifact.kernel_content_id != expected {
                return Err("artifact changed during exact setup".into());
            }
            let mut archive =
                artifact.open_program_archive(&path, KernelLoadOptions { validate })?;
            let mut reader = archive.select(
                artifact
                    .selected_recipe()
                    .ok_or("missing selected recipe")?,
            )?;
            let mut contour_pilots = Vec::new();
            let mut diagnostics = fastsecdec::status::EvaluationDiagnostics::default();
            // Validate exact-only chart records independently, then release
            // their maps/check programs before restoring the compact offset.
            // The aggregate intentionally carries no heavyweight chart data.
            if settings.contour.deformation != fastsecdec::contour::ContourMode::Off
                && settings.contour.validation.policy != fastsecdec::contour::ContourValidation::Off
                && !matches!(
                    settings.scope,
                    fastsecdec::results::ResultScope::SelectedSectors {
                        exact_policy: fastsecdec::results::ExactContributionPolicy::ExcludeAll,
                        ..
                    }
                )
            {
                let exact_records = reader
                    .catalogue()
                    .records
                    .iter()
                    .enumerate()
                    .filter_map(|(index, record)| record.sector.is_none().then_some(index))
                    .collect::<Vec<_>>();
                for index in exact_records {
                    let mut record = reader.load_record(index)?;
                    crate::bind_parameters(&mut record, &settings)?;
                    let started = Instant::now();
                    let pilot = crate::contour_pilot::run(
                        &mut record,
                        &settings.contour,
                        settings.seed,
                        None,
                        |snapshot| {
                            emit(Progress::Contour {
                                snapshot: snapshot.clone(),
                                elapsed_seconds: started.elapsed().as_secs_f64(),
                            })?;
                            Ok(())
                        },
                    )?;
                    if let Some(report) = pilot {
                        contour_pilots.push(crate::contour_pilot::provenance(
                            &record,
                            settings.seed,
                            &report,
                        ));
                    }
                    if settings.contour_diagnostics
                        == fastsecdec::contour::ContourDiagnosticsMode::Aggregate
                        && let Some(report) = record.take_contour_runtime_report()?
                    {
                        diagnostics.record_contour_runtime(
                            fastsecdec::status::IntegrationStage::Pilot,
                            &report,
                        )?;
                    }
                }
            }
            let mut exact = reader.load_exact()?;
            let mut aggregate_settings = settings.clone();
            aggregate_settings.contour.validation.policy =
                fastsecdec::contour::ContourValidation::Off;
            let values = crate::bind_parameters(&mut exact, &aggregate_settings)?;
            if settings.contour_diagnostics
                == fastsecdec::contour::ContourDiagnosticsMode::Aggregate
                && let Some(report) = exact.take_contour_runtime_report()?
            {
                diagnostics
                    .record_contour_runtime(fastsecdec::status::IntegrationStage::Pilot, &report)?;
            }
            let threshold_scope = catalogue.threshold_scope()?;
            // The native owner checks this exact-only carrier against the full
            // catalogue and its admitted, already-bound point. No programs or
            // point state are reconstructed by the CLI.
            let kernel_content_id = if threshold_scope.is_some() {
                reader
                    .catalogue()
                    .bound_threshold_content_id(&exact, &values)?
            } else {
                exact.content_id().into()
            };
            let manifest = KernelResultManifest {
                threshold_scope,
                source_selection: catalogue.source_selection().cloned(),
                kernel_content_id,
                orders: catalogue.orders.clone(),
                components: catalogue.components.clone(),
                exact_coefficients: exact.exact_coefficients().to_vec(),
                sectors: catalogue
                    .records
                    .iter()
                    .filter_map(|record| {
                        record.sector.map(|id| SectorSpec {
                            id: id as u64,
                            dimension: record
                                .receipt
                                .dimension
                                .expect("validated numerical record"),
                        })
                    })
                    .collect(),
            };
            manifest.validate()?;
            crate::artifact::atomic_write(
                &result,
                &serde_json::to_vec(&ExactSetup {
                    manifest,
                    contour_pilots,
                    diagnostics,
                })?,
            )?;
            Ok(())
        }
        Request::Generate {
            input,
            output,
            workers,
            reference,
            overrides,
        } => {
            let (send, receive) = std::sync::mpsc::sync_channel(2);
            std::thread::scope(|scope| {
                let job = scope.spawn(move || -> Result<(), String> {
                    let work = || -> CliResult<()> {
                        let mut dashboard = Dashboard::new(false, false)?;
                        dashboard.forward_generation(send);
                        let reference = crate::reference::from_card(&input, reference.as_deref())?;
                        crate::generate::generate_with_overrides(
                            &input,
                            &output,
                            &mut dashboard,
                            reference.as_ref(),
                            workers,
                            overrides,
                        )?;
                        Ok(())
                    };
                    work().map_err(|error| error.to_string())
                });
                let mut error = None;
                for progress in receive {
                    if let Err(failure) = emit(progress) {
                        error = Some(failure);
                        break;
                    }
                }
                // A failed control receiver disconnects the bounded producer.
                let generated = job.join().map_err(|_| "isolated generation panicked")?;
                if let Some(error) = error {
                    return Err(error.into());
                }
                generated.map_err(Into::into)
            })
        }
    }
}
