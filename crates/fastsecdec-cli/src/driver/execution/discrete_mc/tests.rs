use super::*;
use crate::driver::{replay::AcceptedReplay, timing::TimedStop};
use fastsecdec::{
    generation::{GenerationOptions, generate},
    integration::{IntegrationProblem, SectorSpec},
    kernel::{CompilationSettings, EvaluatorBackend, ReplayPolicy},
    parametric::{ParametricDomain, ParametricIntegrand, ParametricTerm},
};
use std::{ops::ControlFlow, time::Duration};
use symbolica::{atom::Atom, symbol};

#[test]
fn timed_wave_drains_native_returns_before_admission_but_cancel_still_aborts() {
    let x = symbol!("timed_wave_test::x");
    let source = ParametricIntegrand::new(
        vec![x],
        symbol!("timed_wave_test::eps"),
        ParametricDomain::UnitCube,
        vec![ParametricTerm::new(Atom::one(), vec![Atom::num(1)], vec![])],
    )
    .unwrap();
    let kernels = generate(&source, &GenerationOptions::default(), |_| {
        ControlFlow::Continue(())
    })
    .unwrap()
    .compile_with_settings(CompilationSettings {
        backend: EvaluatorBackend::Eager,
        ..Default::default()
    })
    .unwrap();
    let problem = IntegrationProblem::new(
        "timed-wave".into(),
        kernels.orders().to_vec(),
        vec![SectorSpec {
            id: 0,
            dimension: 1,
        }],
        vec![0.; kernels.orders().len()],
    )
    .unwrap();
    let settings = HavanaDiscreteSettings {
        batch: HavanaSettings {
            points_per_batch: 64,
            batches: 4,
            bins: 4,
            ..Default::default()
        },
        ..Default::default()
    };
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    for cancelled in [false, true] {
        let mut session =
            HavanaDiscreteSession::production(problem.clone(), settings.clone()).unwrap();
        let operations =
            super::super::observations::Operations::new(2, kernels.orders(), Duration::ZERO);
        let mut slots = (0..2)
            .map(|id| Slot {
                contexts: BTreeMap::new(),
                worker: session.worker_context().unwrap(),
                meter: operations.worker(id),
            })
            .collect::<Vec<_>>();
        let tasks = (0..2)
            .map(|_| session.next_work().unwrap())
            .collect::<Vec<_>>();
        let expected = tasks.iter().map(|t| t.point_count() as u64).sum::<u64>();
        let replay = AcceptedReplay::new(&kernels, ReplayPolicy::default()).unwrap();
        let mut stop = TimedStop::new(Some(1.)).unwrap();
        // These tasks were already reserved when the deadline arrived. The
        // actual wave callback sees the deadline before spawning workers.
        let returns = wave::run(
            &pool,
            &mut slots,
            tasks,
            &kernels,
            &replay,
            &replay,
            7,
            session.stage(),
            |_, _, _| {
                assert!(stop.observe(Duration::from_secs(1), 2));
                Ok(cancelled) // deadline alone must never become an abort
            },
        )
        .unwrap();
        if cancelled {
            assert!(returns.is_empty());
            assert!(
                !stop
                    .report(Duration::from_secs(2), false)
                    .unwrap()
                    .counts_complete
            );
            assert_eq!(operations.snapshot().unwrap().evaluations, 0);
            continue;
        }
        assert_eq!(returns.len(), 2);
        assert!(
            !stop
                .report(Duration::from_secs(2), true)
                .unwrap()
                .counts_complete
        );
        for returned in returns {
            let completed = returned.unwrap();
            assert!(!completed.aborted_prefix);
            assert!(completed.states.is_some());
            let value = completed.result.unwrap();
            session.submit(value.clone()).unwrap();
            assert!(
                session.submit(value).is_err(),
                "late return must retain native duplicate fencing"
            );
        }
        stop.drained(Duration::from_secs(3));
        let report = stop.report(Duration::from_secs(3), true).unwrap();
        assert!(report.counts_complete);
        assert_eq!(report.in_flight_at_deadline, 2);
        assert!(
            !session.is_complete(),
            "unscheduled next wave must remain absent"
        );
        let metrics = operations.snapshot().unwrap();
        assert_eq!(metrics.evaluations, expected);
        assert_eq!(metrics.diagnostics.f64_timing.calls, expected);
        let restored =
            HavanaDiscreteSession::restore(&session.checkpoint().unwrap(), &problem).unwrap();
        assert_eq!(
            restored.diagnostic_observation().unwrap(),
            session.diagnostic_observation().unwrap()
        );
    }
}
