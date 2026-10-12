use super::{
    Event, Job, Residency, Returned, SerialOutcome, SerialRunSnapshot, TaskMetrics, checkpoint,
};
use crate::{
    CliResult,
    artifact::{Artifact, atomic_write},
    config::IntegrationInput,
    process::{
        ProcessPool, ProcessWorker,
        child::{self, WorkerCommand, WorkerEvent},
    },
};
use fastsecdec::{
    integration::{
        IntegrationProblem, OperationalMetrics, SectorOperationalMetrics, Tolerance,
        mc::HavanaSettings,
        serial::{SerialMethod, SerialSession, SerialSettings, SerialTask},
    },
    kernel::{KernelLoadOptions, ReplayState},
    status::EvaluationDiagnostics,
};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Slot {
    process: ProcessWorker<WorkerEvent<Event>>,
    sector: u64,
    residence: f64,
    loading: f64,
    task: Option<SerialTask>,
    accepted: bool,
    request_path: PathBuf,
    return_path: PathBuf,
    completed: u64,
    planned: u64,
    preparing: bool,
    contour_pilot: Option<crate::contour_pilot::Snapshot>,
    sampling_seconds: f64,
    live_metrics: Option<TaskMetrics>,
}
fn in_flight(slots: &[Option<Slot>]) -> usize {
    slots
        .iter()
        .flatten()
        .filter(|slot| slot.task.is_some())
        .count()
}
fn snapshot(
    session: &SerialSession,
    slots: &[Option<Slot>],
    diagnostics: &EvaluationDiagnostics,
    operational: &OperationalMetrics,
    started: Instant,
    checkpoint_path: &Path,
    saved: Option<Instant>,
) -> CliResult<SerialRunSnapshot> {
    let mut visible_operational = operational.clone();
    for slot in slots.iter().flatten().filter(|slot| !slot.accepted) {
        if let Some(metrics) = &slot.live_metrics {
            accept_metrics(&mut visible_operational, slot.sector, metrics)?;
        }
    }
    Ok(SerialRunSnapshot {
        native: session.snapshot()?,
        observation: session.observation()?,
        residents: slots
            .iter()
            .enumerate()
            .filter_map(|(worker, s)| {
                s.as_ref().map(|s| Residency {
                    worker,
                    pid: s.process.pid(),
                    sector: s.sector,
                    residence_seconds: s.residence + s.sampling_seconds,
                    loading_seconds: s.loading,
                    completed_points: s.completed,
                    planned_points: s.planned,
                    preparing: s.preparing,
                    contour_pilot: s.contour_pilot.clone(),
                })
            })
            .collect(),
        diagnostics: diagnostics.clone(),
        operational: visible_operational,
        elapsed_seconds: started.elapsed().as_secs_f64(),
        checkpoint_path: checkpoint_path.to_path_buf(),
        checkpoint_age_seconds: saved.map(|t| t.elapsed().as_secs_f64()),
    })
}
pub(super) fn native_settings(settings: &IntegrationInput) -> CliResult<SerialSettings> {
    let method =
        match settings.method.replace('-', "_").as_str() {
            "qmc" => SerialMethod::Qmc(settings.qmc_settings()?),
            "adaptive_qmc" => SerialMethod::AdaptiveQmc(settings.qmc_settings()?),
            "mc" | "adaptive_mc" => {
                let native = HavanaSettings {
                    points_per_batch: settings.points.try_into()?,
                    batches: settings.shifts,
                    seed: settings.seed,
                    ..Default::default()
                };
                if settings.method.replace('-', "_") == "adaptive_mc" {
                    SerialMethod::AdaptiveMc(native)
                } else {
                    SerialMethod::Mc(native)
                }
            }
            "discrete_mc" => return Err(
                "--serial uses per-sector Havana; select mc or adaptive_mc instead of discrete_mc"
                    .into(),
            ),
            _ => return Err("unknown serial integration method".into()),
        };
    Ok(SerialSettings {
        method,
        double_points: settings.double_points,
        max_rounds: settings.max_rounds,
        max_in_flight: settings.workers,
        target: settings.accuracy_target,
        tolerance: Tolerance::new(settings.absolute_tolerance, settings.relative_tolerance)?,
        pilot_iterations: 1,
        learning_rate: 0.5,
    })
}
fn accept_metrics(
    metrics: &mut OperationalMetrics,
    sector: u64,
    returned: &TaskMetrics,
) -> CliResult<()> {
    if [
        returned.worker_seconds,
        returned.integrand_seconds,
        returned.load_seconds,
    ]
    .iter()
    .any(|value| !value.is_finite() || *value < 0.)
        || returned
            .maxima
            .values()
            .any(|value| !value.is_finite() || *value < 0.)
    {
        return Err("invalid resident worker operational metrics".into());
    }
    let seconds = returned.worker_seconds + returned.load_seconds;
    let evaluator = returned.diagnostics.f64_timing.seconds()
        + returned.diagnostics.double_float_timing.seconds()
        + returned.diagnostics.arbitrary_timing.seconds()
        + returned.diagnostics.conditioning_timing.seconds();
    metrics.worker_seconds += seconds;
    metrics.integrand_seconds += returned.integrand_seconds + returned.load_seconds;
    metrics.evaluator_seconds += evaluator;
    metrics.evaluations = metrics
        .evaluations
        .checked_add(returned.diagnostics.evaluations)
        .ok_or("serial operational count overflow")?;
    metrics.diagnostics.merge(&returned.diagnostics)?;
    let index = metrics
        .sectors
        .iter()
        .position(|s| s.id == sector)
        .unwrap_or_else(|| {
            metrics.sectors.push(SectorOperationalMetrics {
                id: sector,
                ..Default::default()
            });
            metrics.sectors.len() - 1
        });
    let row = &mut metrics.sectors[index];
    row.worker_seconds += seconds;
    row.integrand_seconds += returned.integrand_seconds + returned.load_seconds;
    row.evaluator_seconds += evaluator;
    row.evaluations = row
        .evaluations
        .checked_add(returned.diagnostics.evaluations)
        .ok_or("sector operational count overflow")?;
    row.diagnostics.merge(&returned.diagnostics)?;
    for (&order, &value) in &returned.maxima {
        row.maximum_weighted_contribution
            .entry(order)
            .and_modify(|v| *v = v.max(value))
            .or_insert(value);
    }
    if [
        metrics.worker_seconds,
        metrics.integrand_seconds,
        metrics.evaluator_seconds,
        row.worker_seconds,
        row.integrand_seconds,
        row.evaluator_seconds,
    ]
    .iter()
    .any(|v| !v.is_finite())
    {
        return Err("serial operational duration overflow".into());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn integrate(
    artifact: &Artifact,
    base: &Path,
    problem: IntegrationProblem,
    settings: &IntegrationInput,
    checkpoint_path: &Path,
    resume: bool,
    load_options: KernelLoadOptions,
    initial_contour_pilots: Vec<fastsecdec::status::ContourPilotProvenance>,
    initial_diagnostics: EvaluationDiagnostics,
    mut observe: impl FnMut(&SerialRunSnapshot) -> CliResult<bool>,
) -> CliResult<SerialOutcome> {
    let mut timed_stop = crate::driver::timing::TimedStop::new(settings.max_seconds)?;
    let catalogue=artifact.catalogue().ok_or("serial integration requires a sector-addressable artifact; regenerate this legacy artifact")?;
    let minimum = settings
        .serial_seconds
        .ok_or("serial integration requires a residence duration")?;
    if !minimum.is_finite() || minimum <= 0. || settings.evaluation_batch_size == 0 {
        return Err("serial residence and evaluation batch size must be positive".into());
    }
    let directory = checkpoint_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(directory)?;
    let mut lock_name = checkpoint_path.as_os_str().to_os_string();
    lock_name.push(".lock");
    // This stable file must never be unlinked: surviving children inherit the
    // OS lock, preventing crash recovery from replaying their live streams.
    let residency_lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(PathBuf::from(lock_name))?;
    residency_lock.try_lock().map_err(|error| {
        format!(
            "serial checkpoint still belongs to another coordinator or surviving workers: {error}"
        )
    })?;
    let native = native_settings(settings)?;
    let (mut session, mut replay, mut diagnostics, mut operational, mut contour_pilots) = if resume
    {
        let (session, checkpoint) = checkpoint::restore(checkpoint_path, settings, &problem)?;
        (
            session,
            checkpoint.replay,
            checkpoint.diagnostics,
            OperationalMetrics::default(),
            checkpoint
                .contour
                .into_iter()
                .flat_map(|entry| entry.pilots)
                .map(|entry| (entry.kernel_content_id.clone(), entry))
                .collect::<BTreeMap<_, _>>(),
        )
    } else {
        (
            SerialSession::new(problem, native)?,
            BTreeMap::<u64, ReplayState>::new(),
            EvaluationDiagnostics::default(),
            OperationalMetrics::default(),
            BTreeMap::new(),
        )
    };
    diagnostics.merge(&initial_diagnostics)?;
    operational.diagnostics.merge(&initial_diagnostics)?;
    let started = Instant::now();
    let mut last_checkpoint = Instant::now();
    let mut saved = resume.then(Instant::now);
    let mut publication = Instant::now();
    let data_path = fs::canonicalize(artifact.data_path(base)?)?;
    let staging_guard = tempfile::Builder::new()
        .prefix("fastsecdec-serial-")
        .tempdir_in(directory)?;
    let staging = fs::canonicalize(staging_guard.path())?;
    let run = format!(
        "{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    );
    let pool = ProcessPool::with_residency_lock(settings.workers, &residency_lock)?;
    let mut slots: Vec<Option<Slot>> = (0..settings.workers).map(|_| None).collect();
    let mut process_lease = 0u64;
    let mut cancelled = false;
    let mut failure = None;
    // Retain one latest completed preflight per bound sector owner. Reloads
    // and concurrent replicas must not grow this catalogue with run duration.
    for report in initial_contour_pilots {
        contour_pilots.insert(report.kernel_content_id.clone(), report);
    }
    'integration: loop {
        timed_stop.observe(started.elapsed(), in_flight(&slots));
        let mut changed = false;
        for slot in slots.iter_mut().flatten() {
            loop {
                let event = match slot.process.poll() {
                    Ok(event) => event,
                    Err(error) => {
                        failure = Some(error.to_string());
                        break 'integration;
                    }
                };
                let Some(event) = event else {
                    break;
                };
                changed = true;
                match event {
                    WorkerEvent::Ready { build, .. } => {
                        if build != child::build_identity() {
                            failure = Some("resident worker executable dependencies differ".into());
                            break 'integration;
                        }
                    }
                    WorkerEvent::Update(event) => {
                        let current = slot
                            .task
                            .as_ref()
                            .ok_or("worker returned an event without a reservation")?;
                        let identity = match &event {
                            Event::Loaded { identity, .. }
                            | Event::ContourPilot { identity, .. }
                            | Event::ContourPilotComplete { identity, .. }
                            | Event::Progress { identity, .. }
                            | Event::Completed { identity, .. } => identity,
                        };
                        if current.identity() != identity {
                            failure = Some("stale resident worker replica event".into());
                            break 'integration;
                        }
                        match event {
                            Event::ContourPilotComplete { report, .. } => {
                                if !slot.preparing || !report.complete {
                                    return Err("invalid resident contour pilot completion".into());
                                }
                                contour_pilots.insert(report.kernel_content_id.clone(), report);
                            }
                            Event::ContourPilot { progress, .. } => {
                                if !slot.preparing || progress.completed > progress.total {
                                    return Err("invalid resident contour pilot progress".into());
                                }
                                slot.contour_pilot = Some(progress);
                            }
                            Event::Loaded { seconds, .. } => {
                                if !seconds.is_finite() || seconds < 0. {
                                    return Err("invalid worker loading duration".into());
                                }
                                slot.loading += seconds;
                                slot.preparing = false;
                                slot.live_metrics = Some(TaskMetrics {
                                    load_seconds: seconds,
                                    ..Default::default()
                                });
                            }
                            Event::Progress {
                                completed,
                                planned,
                                sampling_seconds,
                                metrics,
                                ..
                            } => {
                                if planned != current.point_count()
                                    || completed < slot.completed
                                    || completed > planned
                                    || !sampling_seconds.is_finite()
                                    || sampling_seconds < slot.sampling_seconds
                                {
                                    return Err("invalid resident worker point progress".into());
                                }
                                slot.completed = completed;
                                slot.planned = planned;
                                slot.sampling_seconds = sampling_seconds;
                                let mut validation = OperationalMetrics::default();
                                accept_metrics(&mut validation, slot.sector, &metrics)?;
                                slot.live_metrics = Some(*metrics);
                            }
                            Event::Completed { path, .. } => {
                                if path != slot.return_path || slot.accepted {
                                    return Err(
                                        "duplicate or unexpected serial completion file".into()
                                    );
                                }
                                let returned: Returned =
                                    serde_json::from_reader(File::open(&path)?)?;
                                session.validate_return(&returned.value)?;
                                let mut candidate = replay
                                    .get(&slot.sector)
                                    .cloned()
                                    .unwrap_or_else(|| returned.replay.clone());
                                candidate.merge(&returned.replay)?;
                                let mut next_diagnostics = diagnostics.clone();
                                next_diagnostics.merge(&returned.metrics.diagnostics)?;
                                let mut next_operational = operational.clone();
                                if returned.metrics.worker_seconds
                                    != returned.value.worker_seconds()
                                {
                                    return Err(
                                        "serial receipt timing differs from numerical return"
                                            .into(),
                                    );
                                }
                                accept_metrics(
                                    &mut next_operational,
                                    slot.sector,
                                    &returned.metrics,
                                )?;
                                let residence = slot.residence + returned.value.worker_seconds();
                                if !residence.is_finite() {
                                    return Err("residence duration overflow".into());
                                }
                                // Numerical admission and all prepared metadata commit together.
                                session.submit(returned.value)?;
                                replay.insert(slot.sector, candidate);
                                diagnostics = next_diagnostics;
                                operational = next_operational;
                                slot.residence = residence;
                                slot.sampling_seconds = 0.;
                                slot.live_metrics = None;
                                slot.accepted = true;
                            }
                        }
                    }
                    WorkerEvent::Finished => {
                        if !slot.accepted {
                            return Err(
                                "resident worker finished without a complete numerical return"
                                    .into(),
                            );
                        }
                        slot.task = None;
                        slot.accepted = false;
                        let _ = fs::remove_file(&slot.request_path);
                        let _ = fs::remove_file(&slot.return_path);
                    }
                    WorkerEvent::Failed { message } => {
                        failure = Some(message);
                        break 'integration;
                    }
                }
            }
            if let Some(status) = slot.process.try_wait()? {
                failure = Some(format!("resident worker exited unexpectedly: {status}"));
                break 'integration;
            }
            if slot.process.output_closed() {
                failure = Some("resident worker control connection closed".into());
                break 'integration;
            }
        }
        if changed || publication.elapsed() >= Duration::from_millis(100) {
            cancelled |= observe(&snapshot(
                &session,
                &slots,
                &diagnostics,
                &operational,
                started,
                checkpoint_path,
                saved,
            )?)?;
            publication = Instant::now();
        }
        if cancelled {
            break;
        }
        if session.meets_target()? || session.exhausted() {
            if timed_stop.enabled() {
                timed_stop.begin_drain(started.elapsed());
            } else {
                break;
            }
        }
        timed_stop.observe(started.elapsed(), in_flight(&slots));
        if timed_stop.stopping() && in_flight(&slots) == 0 {
            timed_stop.drained(started.elapsed());
            break;
        }
        if last_checkpoint.elapsed() >= Duration::from_secs(5) {
            checkpoint::save(
                checkpoint_path,
                settings,
                &session,
                &replay,
                &diagnostics,
                &operational,
                &contour_pilots,
            )?;
            last_checkpoint = Instant::now();
            saved = Some(last_checkpoint);
        }
        let mut scheduled = false;
        for index in 0..slots.len() {
            if timed_stop.observe(started.elapsed(), in_flight(&slots)) {
                break;
            }
            if slots[index].as_ref().is_some_and(|s| s.task.is_some()) {
                continue;
            }
            let occupied = slots
                .iter()
                .filter_map(|s| s.as_ref().map(|s| s.sector))
                .collect::<Vec<_>>();
            let resident = slots[index].as_ref().map(|s| s.sector);
            let residence = slots[index].as_ref().map_or(0., |s| s.residence);
            let Some(sector) = session.choose_sector(resident, residence, minimum, &occupied)?
            else {
                continue;
            };
            let Some(task) = session.reserve(sector)? else {
                continue;
            };
            if resident != Some(sector) {
                if let Some(mut previous) = slots[index].take() {
                    previous.process.terminate()?;
                    drop(previous);
                }
                let lease = process_lease;
                process_lease = process_lease
                    .checked_add(1)
                    .ok_or("resident process lease exhausted")?;
                let mut command = Command::new(crate::process::executable()?);
                command
                    .arg("__integrate-worker")
                    .arg("--run-id")
                    .arg(&run)
                    .arg("--lease-id")
                    .arg(lease.to_string());
                let process = pool.spawn_native(
                    &mut command,
                    run.clone(),
                    lease,
                    &staging.join(format!("worker-{index}.log")),
                )?;
                slots[index] = Some(Slot {
                    process,
                    sector,
                    residence: 0.,
                    loading: 0.,
                    task: None,
                    accepted: false,
                    request_path: PathBuf::new(),
                    return_path: PathBuf::new(),
                    completed: 0,
                    planned: 0,
                    preparing: true,
                    contour_pilot: None,
                    sampling_seconds: 0.,
                    live_metrics: None,
                });
            }
            let slot = slots[index].as_mut().unwrap();
            let lease = task.identity().lease;
            slot.request_path = staging.join(format!("request-{lease}.json"));
            slot.return_path = staging.join(format!("return-{lease}.json"));
            let job = Job {
                validation_seed: settings.seed,
                contour: settings.contour.clone(),
                contour_diagnostics: settings.contour_diagnostics,
                task: task.clone(),
                data_path: data_path.clone(),
                catalogue_id: catalogue.content_id.clone(),
                archive_id: artifact.kernel_content_id.clone(),
                recipe: artifact
                    .selected_recipe()
                    .ok_or("missing selected recipe")?,
                validate_artifact: load_options.validate,
                parameters: settings.parameters.clone(),
                policy: settings.replay.clone(),
                stability: settings.stability.clone(),
                batch_size: settings.evaluation_batch_size,
                replay: replay.get(&sector).cloned(),
                return_path: slot.return_path.clone(),
            };
            atomic_write(&slot.request_path, &serde_json::to_vec(&job)?)?;
            slot.completed = 0;
            slot.sampling_seconds = 0.;
            slot.live_metrics = None;
            slot.planned = task.point_count();
            slot.task = Some(task);
            slot.process
                .send(WorkerCommand::Execute(slot.request_path.clone()))?;
            scheduled = true;
        }
        if !scheduled
            && slots
                .iter()
                .all(|s| s.as_ref().is_none_or(|s| s.task.is_none()))
        {
            if timed_stop.stopping() {
                timed_stop.drained(started.elapsed());
                break;
            }
            failure = Some(
                "serial scheduler has no admissible work before convergence or exhaustion".into(),
            );
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let counts_complete = !cancelled && failure.is_none() && in_flight(&slots) == 0;
    // Reap every child before abandoning/reissuing its exact random reservation.
    for slot in &mut slots {
        let Some(mut slot) = slot.take() else {
            continue;
        };
        slot.process.terminate()?;
        if !slot.accepted
            && let Some(metrics) = slot.live_metrics.take()
        {
            // Preserve actual callback work even though this reservation adds
            // no accepted statistical points. Other historical counters keep
            // their existing completed-return convention.
            diagnostics.merge(&EvaluationDiagnostics {
                contour_runtime: metrics.diagnostics.contour_runtime.clone(),
                ..Default::default()
            })?;
            accept_metrics(&mut operational, slot.sector, &metrics)?;
        }
        if let Some(task) = &slot.task
            && !slot.accepted
        {
            session.release(task)?;
        }
    }
    checkpoint::save(
        checkpoint_path,
        settings,
        &session,
        &replay,
        &diagnostics,
        &operational,
        &contour_pilots,
    )?;
    let result = snapshot(
        &session,
        &slots,
        &diagnostics,
        &operational,
        started,
        checkpoint_path,
        Some(Instant::now()),
    )?;
    observe(&result)?;
    Ok(SerialOutcome {
        contour_pilots: contour_pilots.into_values().collect(),
        snapshot: result,
        cancelled,
        failure,
        timed_stop: timed_stop.report(started.elapsed(), counts_complete),
    })
}
