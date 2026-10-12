use super::*;
use crate::{
    artifact::Artifact,
    config::IntegrationInput,
    display::Dashboard,
    driver::{
        checkpoint::{restore_checkpoint, save_checkpoint},
        execution::{evaluate_tracked, integrate, problem, submit_package},
    },
};
use fastsecdec::{integration::VectorEstimate, status::EvaluationDiagnostics};
use std::{collections::BTreeMap, fs};

pub(in crate::driver::execution::qmc) fn fixture()
-> (tempfile::TempDir, Artifact, KernelSet, IntegrationInput) {
    let dir = tempfile::tempdir().unwrap();
    let card = dir.path().join("input.toml");
    fs::write(
        &card,
        "[direct]\ndomain='unit_cube'\nparameters=['x','y']\n\
         [[direct.terms]]\nmonomial_powers=['1','0']\n\
         [[direct.terms.factors]]\npolynomial='x+y'\nexponent='-1+eps'\n\
         [generation]\norder=1\n",
    )
    .unwrap();
    let (artifact, mut kernels) = crate::generate::generate(
        &card,
        &dir.path().join("artifact.fsd"),
        &mut Dashboard::new(false, false).unwrap(),
        None,
    )
    .unwrap();
    // Slot eviction must retain the original validated replay envelope.
    kernels
        .set_stability_settings(&fastsecdec::kernel::StabilitySettings::validated())
        .unwrap();
    assert_eq!(kernels.sectors().len(), 2);
    assert_eq!(kernels.orders(), [0, 1]);
    let settings = IntegrationInput {
        points: 1024,
        shifts: 4,
        package_points: 1024,
        max_rounds: Some(1),
        stability: fastsecdec::kernel::StabilitySettings::validated(),
        ..Default::default()
    };
    (dir, artifact, kernels, settings)
}

#[test]
fn eviction_restores_accepted_envelope_and_failed_replacement_leaves_empty() {
    let (_dir, artifact, kernels, settings) = fixture();
    let mut accepted = AcceptedReplay::new(&kernels, settings.replay.clone()).unwrap();
    let mut session = QmcSession::democratic(
        problem(&artifact, &kernels, &settings.scope).unwrap(),
        settings.qmc_settings().unwrap(),
    )
    .unwrap();
    let mut slot = QmcSlot::default();
    assert!(slot.active.is_none());
    let task = session.next_work().unwrap().unwrap();
    assert_eq!(task.sector_id(), 0);
    slot.prepare(0, &kernels, &session, &accepted).unwrap();
    let active = slot.active.as_mut().unwrap();
    let returned = active
        .worker
        .evaluate_weighted(task, |point, weight, output| {
            active
                .context
                .evaluate_weighted(point, weight, output)
                .map(|_| ())
        });
    let state = active.context.state().clone();
    submit_package(returned, 0, Some(state), &mut accepted, |result| {
        session.submit(result)
    })
    .unwrap();
    let prior = accepted.state(0).clone();
    assert!(prior.verified());
    assert!(prior.maxima().iter().any(|value| *value > 0.0));

    slot.prepare(0, &kernels, &session, &accepted).unwrap();
    assert_eq!(slot.active.as_ref().unwrap().context.state(), &prior);
    slot.prepare(1, &kernels, &session, &accepted).unwrap();
    assert_eq!(slot.active.as_ref().unwrap().id, 1);
    assert!(!slot.active.as_ref().unwrap().context.state().verified());
    slot.prepare(0, &kernels, &session, &accepted).unwrap();
    assert_eq!(slot.active.as_ref().unwrap().context.state(), &prior);
    assert!(
        slot.prepare(u64::MAX, &kernels, &session, &accepted)
            .is_err()
    );
    assert!(slot.active.is_none());
    assert_eq!(accepted.state(0), &prior);
}

#[test]
fn failed_package_prefix_is_not_restored_after_eviction() {
    let (_dir, artifact, kernels, settings) = fixture();
    let mut accepted = AcceptedReplay::new(&kernels, settings.replay.clone()).unwrap();
    let initial = accepted.clone();
    let mut session = QmcSession::democratic(
        problem(&artifact, &kernels, &settings.scope).unwrap(),
        settings.qmc_settings().unwrap(),
    )
    .unwrap();
    let mut slot = QmcSlot::default();
    slot.prepare(0, &kernels, &session, &accepted).unwrap();
    let active = slot.active.as_mut().unwrap();
    let mut calls = 0;
    let result = active.worker.evaluate_weighted(
        session.next_work().unwrap().unwrap(),
        |point, weight, output| {
            calls += 1;
            if calls == 3 {
                return Err("injected failure".to_owned());
            }
            active
                .context
                .evaluate_weighted(point, weight, output)
                .map(|_| ())
                .map_err(|error| error.to_string())
        },
    );
    assert!(active.context.state().verified());
    let failed_state = active.context.state().clone();
    assert!(
        submit_package(result, 0, Some(failed_state), &mut accepted, |result| {
            session.submit(result)
        })
        .is_err()
    );
    assert_eq!(accepted, initial);
    slot.prepare(1, &kernels, &session, &accepted).unwrap();
    slot.prepare(0, &kernels, &session, &accepted).unwrap();
    assert_eq!(
        slot.active.as_ref().unwrap().context.state(),
        initial.state(0)
    );
    assert_eq!(session.snapshot().unwrap().completed_points, 0);
}

// Test-only prior eager owner, used under exactly the same native task schedule.
struct EagerSlot {
    contexts: BTreeMap<u64, WeightedEvaluationContext>,
    workers: BTreeMap<u64, QmcWorker>,
}

fn run_schedule(
    artifact: &Artifact,
    kernels: &KernelSet,
    settings: &IntegrationInput,
    eager: bool,
) -> (VectorEstimate, EvaluationDiagnostics, AcceptedReplay) {
    let mut accepted = AcceptedReplay::new(kernels, settings.replay.clone()).unwrap();
    let mut session = QmcSession::democratic(
        problem(artifact, kernels, &settings.scope).unwrap(),
        settings.qmc_settings().unwrap(),
    )
    .unwrap();
    let mut bounded: Vec<_> = (0..settings.workers).map(|_| QmcSlot::default()).collect();
    let mut old: Vec<_> = (0..if eager { settings.workers } else { 0 })
        .map(|_| EagerSlot {
            contexts: accepted
                .contexts(kernels, &session.problem().sectors)
                .unwrap(),
            workers: session
                .problem()
                .sectors
                .iter()
                .map(|sector| (sector.id, session.worker_context(sector.id).unwrap()))
                .collect(),
        })
        .collect();
    let mut diagnostics = EvaluationDiagnostics::default();
    while !session.is_complete() {
        let tasks: Vec<_> = (0..settings.workers)
            .filter_map(|_| session.next_work().unwrap())
            .collect();
        assert!(!tasks.is_empty());
        let mut returns = Vec::new();
        for (index, task) in tasks.into_iter().enumerate() {
            let id = task.sector_id();
            let (worker, context) = if eager {
                let slot = &mut old[index];
                let context = slot.contexts.get_mut(&id).unwrap();
                context.merge_state(accepted.state(id as usize)).unwrap();
                (slot.workers.get_mut(&id).unwrap(), context)
            } else {
                bounded[index]
                    .prepare(id, kernels, &session, &accepted)
                    .unwrap();
                let active = bounded[index].active.as_mut().unwrap();
                (&mut active.worker, &mut active.context)
            };
            let mut local = EvaluationDiagnostics::default();
            let result = worker.evaluate_weighted(task, |point, weight, output| {
                evaluate_tracked(context, point, weight, output, &mut local)
            });
            returns.push((id as usize, result, local, context.state().clone()));
        }
        for (id, result, local, state) in returns {
            diagnostics.merge(&local).unwrap();
            submit_package(result, id, Some(state), &mut accepted, |result| {
                session.submit(result)
            })
            .unwrap();
        }
    }
    (session.estimate().unwrap(), diagnostics, accepted)
}

#[test]
fn bounded_slots_match_eager_full_vectors_for_identical_worker_schedules() {
    let (_dir, artifact, kernels, mut settings) = fixture();
    for workers in [1, 3] {
        settings.workers = workers;
        let mut eager = run_schedule(&artifact, &kernels, &settings, true);
        let mut bounded = run_schedule(&artifact, &kernels, &settings, false);
        assert!(bounded.0.production_complete);
        assert_eq!(bounded.0.mean.len(), 2);
        assert_eq!(bounded.0.covariance_of_mean.len(), 4);
        assert!(bounded.1.rescues > 0);
        assert!(bounded.1.max_precision_bits > 53);
        // Timing observations vary between runs; keep all scientific counters
        // and native evaluator point/invocation counts in the equality control.
        for diagnostics in [&mut eager.1, &mut bounded.1] {
            diagnostics.f64_timing.nanoseconds = 0;
            diagnostics.double_float_timing.nanoseconds = 0;
            diagnostics.arbitrary_timing.nanoseconds = 0;
            diagnostics.conditioning_timing.nanoseconds = 0;
        }
        assert_eq!(bounded, eager);
    }
}

#[test]
fn evicted_partial_checkpoint_resumes_full_scope_with_eight_workers() {
    let (dir, artifact, kernels, mut settings) = fixture();
    let mut accepted = AcceptedReplay::new(&kernels, settings.replay.clone()).unwrap();
    let mut session = QmcSession::democratic(
        problem(&artifact, &kernels, &settings.scope).unwrap(),
        settings.qmc_settings().unwrap(),
    )
    .unwrap();
    let mut slot = QmcSlot::default();
    let mut diagnostics = EvaluationDiagnostics::default();
    for _ in 0..3 {
        let task = session.next_work().unwrap().unwrap();
        let id = task.sector_id();
        slot.prepare(id, &kernels, &session, &accepted).unwrap();
        let active = slot.active.as_mut().unwrap();
        let result = active
            .worker
            .evaluate_weighted(task, |point, weight, output| {
                evaluate_tracked(&mut active.context, point, weight, output, &mut diagnostics)
            });
        let state = active.context.state().clone();
        submit_package(result, id as usize, Some(state), &mut accepted, |result| {
            session.submit(result)
        })
        .unwrap();
    }
    assert_eq!(session.snapshot().unwrap().completed_points, 3072);
    let prior = accepted.clone();
    let checkpoint = dir.path().join("evicted-checkpoint.json");
    save_checkpoint(
        &checkpoint,
        &artifact,
        &settings,
        0,
        session.checkpoint().unwrap(),
        &diagnostics,
        &accepted,
    )
    .unwrap();
    settings.workers = 8;
    let report = integrate(
        &artifact,
        &kernels,
        &settings,
        &checkpoint,
        true,
        &mut Dashboard::new(false, false).unwrap(),
    )
    .unwrap();
    assert_eq!(report.snapshot.completed_points, 8192);
    assert_eq!(report.snapshot.planned_points, 8192);
    let estimate = report.estimate.unwrap();
    assert!(estimate.production_complete);
    assert_eq!(estimate.mean.len(), 2);
    assert_eq!(estimate.covariance_of_mean.len(), 4);
    let restored = restore_checkpoint(&checkpoint, &artifact, &settings).unwrap();
    assert_eq!(restored.diagnostics.evaluations, 8192);
    for id in 0..2 {
        assert!(restored.replay.state(id).verified());
        assert!(
            restored
                .replay
                .state(id)
                .maxima()
                .iter()
                .zip(prior.state(id).maxima())
                .all(|(current, old)| current >= old)
        );
    }
}
