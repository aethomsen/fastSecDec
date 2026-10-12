use super::*;
use crate::kernel::EvaluationTimings;

fn timings(seed: u64) -> EvaluationTimings {
    let bucket = |n| EvaluatorTiming {
        calls: seed + n,
        nanoseconds: 100 * seed + n,
        matrix_invocations: seed + 2 * n,
        matrix_points: seed + 3 * n,
    };
    EvaluationTimings {
        f64: bucket(1),
        double_float: bucket(2),
        arbitrary: bucket(3),
        conditioning: bucket(4),
    }
}

fn recorded(diagnostics: &EvaluationDiagnostics) -> EvaluationTimings {
    EvaluationTimings {
        f64: diagnostics.f64_timing,
        double_float: diagnostics.double_float_timing,
        arbitrary: diagnostics.arbitrary_timing,
        conditioning: diagnostics.conditioning_timing,
    }
}

#[test]
fn all_native_timing_buckets_survive_worker_and_failure_aggregation_once() {
    let ordinary = timings(3);
    let replay = timings(7);
    let failure = timings(11);
    let mut first = EvaluationDiagnostics::default();
    first
        .record(PrecisionReport {
            timings: ordinary,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(recorded(&first), ordinary);
    let mut worker = EvaluationDiagnostics::default();
    worker
        .record_replay(ReplayReport {
            precision: PrecisionReport {
                rescued: true,
                checked: true,
                bits: 256,
                class: PrecisionClass::Arbitrary,
                timings: replay,
            },
            weighted_check: true,
            replayed: true,
        })
        .unwrap();
    assert_eq!(recorded(&worker), replay);
    worker.record_failure_with_timings(failure).unwrap();
    first.merge(&worker).unwrap();
    let expected = |a: EvaluatorTiming, b: EvaluatorTiming, c: EvaluatorTiming| EvaluatorTiming {
        calls: a.calls + b.calls + c.calls,
        nanoseconds: a.nanoseconds + b.nanoseconds + c.nanoseconds,
        matrix_invocations: a.matrix_invocations + b.matrix_invocations + c.matrix_invocations,
        matrix_points: a.matrix_points + b.matrix_points + c.matrix_points,
    };
    assert_eq!(
        recorded(&first),
        EvaluationTimings {
            f64: expected(ordinary.f64, replay.f64, failure.f64),
            double_float: expected(
                ordinary.double_float,
                replay.double_float,
                failure.double_float
            ),
            arbitrary: expected(ordinary.arbitrary, replay.arbitrary, failure.arbitrary),
            conditioning: expected(
                ordinary.conditioning,
                replay.conditioning,
                failure.conditioning
            ),
        }
    );
    assert_eq!(first.evaluations, 3);
    assert_eq!(first.failures, 1);
    assert_eq!(first.additional_replays, 1);
    assert_eq!(first.conditioning_checks, 1);
}

#[test]
fn each_conditioning_counter_overflow_preserves_the_entire_aggregate() {
    for field in 0..4 {
        let mut incoming = EvaluationDiagnostics {
            conditioning_timing: EvaluatorTiming::default(),
            evaluations: 1,
            ..Default::default()
        };
        let mut total = EvaluationDiagnostics::default();
        total.record_failure_with_timings(timings(3)).unwrap();
        let before = total.clone();
        match field {
            0 => incoming.conditioning_timing.calls = u64::MAX,
            1 => incoming.conditioning_timing.nanoseconds = u64::MAX,
            2 => incoming.conditioning_timing.matrix_invocations = u64::MAX,
            3 => incoming.conditioning_timing.matrix_points = u64::MAX,
            _ => unreachable!(),
        }
        assert_eq!(total.merge(&incoming), Err(DiagnosticsOverflow));
        assert_eq!(total, before);
    }
}

#[test]
fn historical_json_reencoding_preserves_checkpoint_digest_and_new_bucket_roundtrips() {
    // Exact pre-field struct serialization. CLI serial checkpoint validation
    // hashes the decoded payload after re-encoding, so defaults alone are not enough.
    let historical = r#"{"evaluations":1,"conditioning_checks":0,"rescues":0,"max_precision_bits":0,"failures":0,"weighted_checks":0,"additional_replays":0,"f64_points":0,"double_float_points":0,"arbitrary_points":0,"unstable_points":0,"cutoff_zero_points":0,"f64_timing":{"calls":0,"nanoseconds":0,"matrix_invocations":0,"matrix_points":0},"double_float_timing":{"calls":0,"nanoseconds":0,"matrix_invocations":0,"matrix_points":0},"arbitrary_timing":{"calls":0,"nanoseconds":0,"matrix_invocations":0,"matrix_points":0}}"#;
    let old: EvaluationDiagnostics = serde_json::from_str(historical).unwrap();
    assert_eq!(old.conditioning_timing, EvaluatorTiming::default());
    let encoded = serde_json::to_vec(&old).unwrap();
    assert_eq!(encoded, historical.as_bytes());
    assert_eq!(blake3::hash(&encoded), blake3::hash(historical.as_bytes()));
    let current = EvaluationDiagnostics {
        conditioning_timing: timings(5).conditioning,
        ..old
    };
    let encoded = serde_json::to_vec(&current).unwrap();
    assert_eq!(
        serde_json::from_slice::<EvaluationDiagnostics>(&encoded).unwrap(),
        current
    );
    assert_ne!(blake3::hash(&encoded), blake3::hash(historical.as_bytes()));
}

#[test]
fn real_native_weighted_replay_retains_conditioning_and_rescue_deltas_once() {
    use crate::{
        generation::{GenerationOptions, generate},
        kernel::{CompilationSettings, EvaluatorBackend, ReplayPolicy, StabilitySettings},
        parametric::{
            FactorRole, ParametricDomain, ParametricIntegrand, ParametricTerm, PolynomialFactor,
        },
    };
    use std::ops::ControlFlow;
    use symbolica::{atom::Atom, parse, symbol};

    // The established logarithmic Taylor-cancellation control, evaluated at
    // only two fixed points; no integration session or new timer is involved.
    let input = ParametricIntegrand::new(
        vec![symbol!("telemetry_native::x")],
        symbol!("telemetry_native::eps"),
        ParametricDomain::UnitCube,
        vec![ParametricTerm::new(
            Atom::one(),
            vec![parse!("-1+telemetry_native::eps")],
            vec![PolynomialFactor::new(
                parse!("1+telemetry_native::x"),
                parse!("-1-telemetry_native::eps"),
                FactorRole::Singularity,
            )],
        )],
    )
    .unwrap();
    let generated = generate(
        &input,
        &GenerationOptions {
            max_order: 1,
            ..Default::default()
        },
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    let mut kernels = generated
        .compile_with_settings(CompilationSettings {
            backend: EvaluatorBackend::Eager,
            ..Default::default()
        })
        .unwrap();
    kernels
        .set_stability_settings(&StabilitySettings::validated())
        .unwrap();
    let mut context = kernels
        .evaluation_context(0, ReplayPolicy::default())
        .unwrap();
    let before = context.evaluation_metrics();
    let mut worker = EvaluationDiagnostics::default();
    let mut values = vec![0.0; kernels.orders().len()];
    for point in [0.25, 1e-12] {
        let report = context
            .evaluate_weighted(&[point], 1.0, &mut values)
            .unwrap();
        worker.record_replay(report).unwrap();
    }
    let native_delta = context.evaluation_metrics().since(before);
    assert!(native_delta.conditioning.calls > 0);
    assert!(native_delta.arbitrary.calls > 0);
    assert_eq!(native_delta.f64.calls, 2);
    assert_eq!(recorded(&worker), native_delta);
    let expected = -(1e-12_f64.ln() + 1e-12_f64.ln_1p() / 1e-12) / (1.0 + 1e-12);
    assert!((values[2] - expected).abs() < 1e-12);
    let mut total = EvaluationDiagnostics::default();
    total.merge(&worker).unwrap();
    assert_eq!(recorded(&total), native_delta);
    assert_eq!(total.evaluations, 2);
}
