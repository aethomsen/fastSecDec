//! Caller-owned global batches with native discrete sector/continuous sampling.
use super::super::{
    IntegrationReport, ResumeStatus,
    checkpoint::save_checkpoint_with_previous as save_checkpoint,
    refinement::{PreviousProduction, mc_design},
    report::{ExecutionOutcome, finish},
};
use super::{Context, final_report, observe};
use crate::CliResult;
use fastsecdec::{
    integration::{
        mc::HavanaSettings,
        mc_discrete::{HavanaDiscreteSession, HavanaDiscreteSettings, HavanaDiscreteWorker},
    },
    kernel::WeightedEvaluationContext,
    status::IntegrationStage,
};
mod wave;
use std::{collections::BTreeMap, time::Instant};
struct Slot {
    contexts: BTreeMap<u64, WeightedEvaluationContext>,
    worker: HavanaDiscreteWorker,
    meter: super::observations::WorkerMeter,
}
pub(super) fn run(context: Context<'_>) -> CliResult<IntegrationReport> {
    let Context {
        artifact,
        kernels,
        settings,
        checkpoint,
        dashboard,
        pool,
        problem,
        tolerance,
        started,
        mut last_checkpoint,
        restored,
        contour_provenance,
        mut diagnostics,
        mut replay,
        operations,
    } = context;
    let mut timed_stop = crate::driver::timing::TimedStop::new(settings.max_seconds)?;
    let initialization = operations.coordinator(false);
    let steering = settings.discrete_mc.clone().unwrap_or_default();
    if steering.pilot_iterations == 0
        || [
            steering.discrete_learning_rate,
            steering.continuous_learning_rate,
        ]
        .iter()
        .any(|r| !r.is_finite() || *r < 0.0)
    {
        return Err(
            "discrete MC requires positive pilot iterations and finite nonnegative learning rates"
                .into(),
        );
    }
    let pilot = HavanaDiscreteSettings {
        batch: HavanaSettings {
            points_per_batch: steering.pilot_points,
            batches: steering.pilot_batches,
            seed: settings.seed,
            bins: steering.bins,
            minimum_probability_density: steering.minimum_probability_density,
        },
        maximum_sector_probability_ratio: steering.maximum_sector_probability_ratio,
    };
    pilot.validate()?;
    let mut production = pilot.clone();
    production.batch.points_per_batch = settings.points.try_into()?;
    production.batch.batches = settings.shifts;
    production.validate()?;
    let mut round = restored.as_ref().map_or(0, |c| c.round_index);
    let mut session = if let Some(stored) = &restored {
        HavanaDiscreteSession::restore(&stored.session, &problem)?
    } else {
        HavanaDiscreteSession::pilot(problem.clone(), pilot.clone())?
    };
    if restored.is_some() {
        let (points, batches) = mc_design(settings, round)?;
        production.batch.points_per_batch = points.try_into()?;
        production.batch.batches = batches;
        if session.settings() != &production {
            return Err(
                "checkpoint native discrete MC design differs from the outer refinement settings"
                    .into(),
            );
        }
    }
    let mut pilot_iteration = 0usize;
    let mut previous_complete = restored.as_ref().and_then(|c| c.previous_complete.clone());
    if let Some(previous) = &previous_complete {
        previous.validate(&problem, round)?;
        dashboard
            .integration_observation(&previous.observation, started.elapsed().as_secs_f64())?;
    }
    let mut cancelled = false;
    let mut failure = None;
    let mut all_returns_complete = true;
    // Lazily cache only sectors actually selected by each caller-owned worker.
    // Caches survive grid adaptation; their worst-case size is all visited
    // sectors per worker, not a claim of one evaluator context per worker.
    let mut slots: Vec<Slot> = Vec::new();
    let mut resumed_phase = restored.is_some();
    drop(initialization);
    'rounds: loop {
        dashboard.set_live_observation(None);
        let frozen_replay = replay.clone();
        let mut live_ledger = super::observations::McLedger::default();
        let live_source = if resumed_phase {
            fastsecdec::integration::LiveSource::SinceResume
        } else {
            fastsecdec::integration::LiveSource::CurrentIteration
        };
        observe(
            dashboard,
            &operations,
            &diagnostics,
            started,
            true,
            || session.diagnostic_observation(),
            &mut failure,
        )?;
        cancelled |= dashboard.cancelled();
        if failure.is_some() || cancelled {
            break;
        }
        if timed_stop.observe(started.elapsed(), 0) {
            timed_stop.drained(started.elapsed());
            break;
        }
        let grid_span = operations.coordinator(false);
        if slots.is_empty() && !problem.sectors.is_empty() {
            slots = (0..settings.workers)
                .map(|id| {
                    Ok(Slot {
                        contexts: BTreeMap::new(),
                        worker: session.worker_context()?,
                        meter: operations.worker(id),
                    })
                })
                .collect::<CliResult<Vec<_>>>()?;
        } else {
            for slot in &mut slots {
                slot.worker = session.worker_context()?;
            }
        }
        drop(grid_span);
        while !session.is_complete() {
            cancelled |= dashboard.cancelled();
            if cancelled {
                break 'rounds;
            }
            if timed_stop.observe(started.elapsed(), 0) {
                timed_stop.drained(started.elapsed());
                break 'rounds;
            }
            let schedule_span = operations.coordinator(false);
            let tasks = (0..settings.workers)
                .filter_map(|_| session.next_work())
                .collect::<Vec<_>>();
            if tasks.is_empty() {
                return Err("discrete MC scheduler has no work before completion".into());
            }
            drop(schedule_span);
            let prepare_span = operations.coordinator(true);
            for slot in &mut slots {
                for (id, context) in &mut slot.contexts {
                    context.merge_state(replay.state(*id as usize))?;
                    context.set_reference_state(frozen_replay.state(*id as usize))?;
                }
            }
            drop(prepare_span);
            let wave_tasks = tasks.len();
            let returns = wave::run(
                &pool,
                &mut slots,
                tasks,
                kernels,
                &replay,
                &frozen_replay,
                settings.evaluation_batch_size,
                session.stage(),
                |activity, collect_live, completed| {
                    // A deadline stops the next wave, never this wave's native
                    // random batches. Only cancellation/failure signals abort.
                    timed_stop.observe(started.elapsed(), wave_tasks);
                    dashboard.integration_work(activity);
                    if completed {
                        live_ledger.update(collect_live().into_iter());
                    }

                    cancelled |= dashboard.cancelled();
                    let _span = operations.coordinator(false);
                    super::observe_live(
                        dashboard,
                        &operations,
                        &diagnostics,
                        started,
                        cancelled || failure.is_some(),
                        || {
                            if !completed {
                                live_ledger.update(collect_live().into_iter());
                            }
                            fastsecdec::integration::mc_live_observation(
                                &problem,
                                session.stage(),
                                live_source,
                                &live_ledger.batches(),
                                true,
                            )
                        },
                        || session.diagnostic_observation(),
                        &mut failure,
                    )?;
                    Ok(cancelled || failure.is_some())
                },
            )?;
            all_returns_complete &= returns.len() == wave_tasks;
            dashboard.integration_work(Vec::new());
            let admission = operations.coordinator(false);
            for returned in returns {
                let completed = match returned {
                    Ok(completed) => completed,
                    Err(error) => {
                        all_returns_complete = false;
                        failure
                            .get_or_insert_with(|| format!("discrete MC worker panicked: {error}"));
                        continue;
                    }
                };
                diagnostics.merge(&completed.diagnostics)?;
                if completed.aborted_prefix {
                    all_returns_complete = false;
                    live_ledger.discard(None, completed.batch);
                    continue;
                }
                let wave::Completed {
                    result,
                    states,
                    batch,
                    ..
                } = completed;
                let accepted = (|| -> CliResult<()> {
                    let value = result?;
                    let states = states.ok_or("successful global batch has no replay states")?;
                    // Validate and merge every sector into a private candidate;
                    // neither numerical coverage nor replay accepts a prefix.
                    let mut candidate = replay.clone();
                    for (id, state) in states {
                        candidate.accept(id, &state)?;
                    }
                    session.submit(value)?;
                    replay = candidate;
                    Ok(())
                })();
                if let Err(error) = accepted {
                    all_returns_complete = false;
                    live_ledger.discard(None, batch);
                    failure.get_or_insert_with(|| error.to_string());
                }
            }
            drop(admission);

            cancelled = dashboard.cancelled();
            super::observe_live(
                dashboard,
                &operations,
                &diagnostics,
                started,
                cancelled || failure.is_some() || session.is_complete(),
                || {
                    fastsecdec::integration::mc_live_observation(
                        &problem,
                        session.stage(),
                        live_source,
                        &live_ledger.batches(),
                        true,
                    )
                },
                || session.diagnostic_observation(),
                &mut failure,
            )?;
            if cancelled || failure.is_some() {
                break 'rounds;
            }
            if timed_stop.observe(started.elapsed(), 0) {
                timed_stop.drained(started.elapsed());
                break 'rounds;
            }
            if last_checkpoint.elapsed().as_secs() >= 5
                && session.stage() == IntegrationStage::Production
            {
                let _checkpoint_span = operations.coordinator(false);
                save_checkpoint(
                    checkpoint,
                    artifact,
                    settings,
                    round,
                    session.checkpoint()?,
                    &diagnostics,
                    &replay,
                    previous_complete.as_ref(),
                    contour_provenance.as_ref(),
                )?;
                last_checkpoint = Instant::now();
            }
        }
        resumed_phase = false;
        let _refinement_span = operations.coordinator(false);
        if session.stage() == IntegrationStage::Pilot {
            pilot_iteration += 1;
            if pilot_iteration < steering.pilot_iterations {
                session.adapt_pilot(
                    steering.discrete_learning_rate,
                    steering.continuous_learning_rate,
                )?;
            } else {
                let (points, batches) = mc_design(settings, round)?;
                session.freeze_production(
                    steering.discrete_learning_rate,
                    steering.continuous_learning_rate,
                    points.try_into()?,
                    batches,
                )?;
            }
            continue;
        }
        let meets = match session
            .estimate()
            .and_then(|e| e.meets_target(settings.accuracy_target, tolerance))
        {
            Ok(v) => v,
            Err(error) => {
                failure = Some(error.to_string());
                break;
            }
        };
        if meets || round + 1 >= settings.ordinary_max_rounds() {
            timed_stop.begin_drain(started.elapsed());
            timed_stop.drained(started.elapsed());
            break;
        }
        save_checkpoint(
            checkpoint,
            artifact,
            settings,
            round,
            session.checkpoint()?,
            &diagnostics,
            &replay,
            previous_complete.as_ref(),
            contour_provenance.as_ref(),
        )?;
        last_checkpoint = Instant::now();
        round += 1;
        previous_complete = Some(PreviousProduction {
            round: round - 1,
            observation: session.diagnostic_observation()?,
            qmc_design: None,
        });
        if !settings.double_points {
            session.extend_production_batches(mc_design(settings, round)?.1)?;
            continue;
        }
        session = HavanaDiscreteSession::pilot(problem.clone(), pilot.clone())?;
        pilot_iteration = 0;
    }
    let checkpoint_span = operations.coordinator(false);
    let resume_status = if session.stage() == IntegrationStage::Pilot {
        ResumeStatus::PilotRestartRequired
    } else {
        save_checkpoint(
            checkpoint,
            artifact,
            settings,
            round,
            session.checkpoint()?,
            &diagnostics,
            &replay,
            previous_complete.as_ref(),
            contour_provenance.as_ref(),
        )?;
        ResumeStatus::CheckpointSaved
    };
    drop(checkpoint_span);
    let timed_report = timed_stop.report(
        started.elapsed(),
        !cancelled && failure.is_none() && all_returns_complete,
    );
    let mut report = finish(
        artifact,
        session.diagnostic_observation()?,
        &diagnostics,
        tolerance,
        started.elapsed().as_secs_f64(),
        ExecutionOutcome {
            stability_mode: settings.stability.mode,
            accuracy_target: settings.accuracy_target,
            operational: operations.snapshot()?,
            scope: settings.scope.clone(),
            cancelled,
            failure,
            resume_status,
            qmc_design: None,
            timed_stop: timed_report,
        },
    )?;
    if session.stage() != IntegrationStage::Production || !session.is_complete() {
        report.previous_complete = previous_complete;
    }
    final_report(dashboard, report)
}

#[cfg(test)]
mod tests;
