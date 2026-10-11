use super::super::{
    IntegrationReport, ResumeStatus,
    checkpoint::save_checkpoint_with_previous as save_checkpoint,
    refinement::{PreviousProduction, adaptive_budget, qmc_design},
    report::{ExecutionOutcome, finish},
};
use super::{Context, final_report, observe};
use crate::CliResult;
use fastsecdec::{
    integration::QmcSession,
    status::{IntegrationMethod, IntegrationStage},
};
use std::time::Instant;

mod queue;
mod slot;
pub(super) fn run(context: Context<'_>, method: &str) -> CliResult<IntegrationReport> {
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
    let initialization = operations.coordinator(false);
    let options = settings.qmc_settings()?;
    let mut session = if let Some(checkpoint) = &restored {
        let session = QmcSession::restore(&checkpoint.session, &problem)?;
        let mut expected = options.clone();
        (expected.points, expected.shifts) = qmc_design(settings, checkpoint.round_index)?;
        let expected_method = if method == "adaptive_qmc" {
            IntegrationMethod::AdaptiveQmc
        } else {
            IntegrationMethod::DemocraticQmc
        };
        if session.design().settings != expected || session.method() != expected_method {
            return Err(
                "checkpoint native QMC design differs from the outer settings or refinement round"
                    .into(),
            );
        }
        session
    } else if method == "adaptive_qmc" {
        QmcSession::adaptive(problem.clone(), options.clone())?
    } else {
        QmcSession::democratic(problem.clone(), options.clone())?
    };
    let mut cancelled = false;
    let mut failure = None;
    let mut round = restored
        .as_ref()
        .map_or(0, |checkpoint| checkpoint.round_index);
    let mut previous_complete = restored.as_ref().and_then(|c| c.previous_complete.clone());
    if let Some(previous) = &previous_complete {
        previous.validate(&problem, round)?;
        dashboard
            .integration_observation(&previous.observation, started.elapsed().as_secs_f64())?;
    }
    drop(initialization);
    'rounds: loop {
        dashboard.set_live_observation(None);
        observe(
            dashboard,
            &operations,
            &diagnostics,
            started,
            true,
            || session.diagnostic_observation(),
            &mut failure,
        )?;
        if failure.is_some() {
            break;
        }
        let outcome = queue::Phase {
            pool: &pool,
            kernels,
            session: &mut session,
            workers: settings.workers,
            diagnostics: &mut diagnostics,
            replay: &mut replay,
            operations: &operations,
        }
        .run_batched(
            settings.evaluation_batch_size,
            |session, diagnostics, replay, force| {
                dashboard.integration_work(operations.activities());

                let cancelled = dashboard.cancelled();
                let mut failure = None;
                let bookkeeping = operations.coordinator(false);
                super::observe_live(
                    dashboard,
                    &operations,
                    diagnostics,
                    started,
                    force || cancelled,
                    || session.live_observation(),
                    || session.diagnostic_observation(),
                    &mut failure,
                )?;
                if last_checkpoint.elapsed().as_secs() >= 5 {
                    save_checkpoint(
                        checkpoint,
                        artifact,
                        settings,
                        round,
                        session.checkpoint()?,
                        diagnostics,
                        replay,
                        previous_complete.as_ref(),
                        contour_provenance.as_ref(),
                    )?;
                    last_checkpoint = Instant::now();
                }
                drop(bookkeeping);
                Ok(queue::Outcome { cancelled, failure })
            },
        )?;
        dashboard.integration_work(Vec::new());
        cancelled = outcome.cancelled;
        failure = outcome.failure;
        if cancelled || failure.is_some() {
            break 'rounds;
        }
        let _refinement_span = operations.coordinator(false);
        if session.stage() == IntegrationStage::Pilot {
            let (seconds, minimum_shifts) = adaptive_budget(settings, round)?;
            let allocation = match session.recommend_allocation(
                seconds,
                minimum_shifts,
                &vec![1.0; kernels.orders().len()],
            ) {
                Ok(allocation) => allocation,
                Err(error) => {
                    failure = Some(error.to_string());
                    break;
                }
            };
            session.freeze_production(allocation)?;
            continue;
        }
        let meets = match session
            .estimate()
            .and_then(|estimate| estimate.meets_target(settings.accuracy_target, tolerance))
        {
            Ok(meets) => meets,
            Err(error) => {
                failure = Some(error.to_string());
                break;
            }
        };
        if meets || round + 1 >= settings.ordinary_max_rounds() {
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
        let mut next = options.clone();
        (next.points, next.shifts) = qmc_design(settings, round)?;
        previous_complete = Some(PreviousProduction {
            round: round - 1,
            observation: session.diagnostic_observation()?,
            qmc_design: Some(session.design()),
        });
        if next.points == session.design().settings.points {
            session.extend_production_shifts(2)?;
            continue;
        }
        session = if method == "adaptive_qmc" {
            QmcSession::adaptive(problem.clone(), next)?
        } else {
            QmcSession::democratic(problem.clone(), next)?
        };
    }
    let checkpoint_span = operations.coordinator(false);
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
    drop(checkpoint_span);
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
            resume_status: ResumeStatus::CheckpointSaved,
            qmc_design: Some(session.design()),
            timed_stop: None,
        },
    )?;
    if session.stage() != IntegrationStage::Production || !session.is_complete() {
        report.previous_complete = previous_complete;
    }
    final_report(dashboard, report)
}
