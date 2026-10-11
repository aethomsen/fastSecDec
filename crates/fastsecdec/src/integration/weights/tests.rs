use super::*;
use crate::integration::{mc::*, mc_discrete::*, serial::*, *};
fn problem(sectors: usize) -> IntegrationProblem {
    IntegrationProblem::new_with_components(
        "complex-weight-test".into(),
        vec![-1, -1, 0, 0],
        vec![
            CoefficientComponent::Real,
            CoefficientComponent::Imag,
            CoefficientComponent::Real,
            CoefficientComponent::Imag,
        ],
        (0..sectors)
            .map(|id| SectorSpec {
                id: id as u64,
                dimension: 1,
            })
            .collect(),
        vec![0., 0., 1., 0.],
    )
    .unwrap()
}
fn qmc() -> QmcSettings {
    QmcSettings {
        points: 8,
        shifts: 2,
        package_points: 3,
        seed: 43,
        periodization: Periodization::None,
        rule: RuleSource::Supplied(vec![1]),
    }
}
fn mc() -> HavanaSettings {
    HavanaSettings {
        points_per_batch: 32,
        batches: 2,
        bins: 4,
        seed: 43,
        ..Default::default()
    }
}
fn values(id: u64) -> [f64; 4] {
    if id == 0 {
        [0., 5., 3., 4.]
    } else {
        [0., -5., -3., -4.]
    }
}
fn assert_diag(d: &WeightDiagnostics, p: &IntegrationProblem, e: &VectorEstimate, n: u64) {
    d.validate(&p.orders, &p.components, &p.exact_coefficients, Some(e), n)
        .unwrap();
    assert_eq!(d.status, WeightDiagnosticStatus::Available);
    assert_eq!(d.orders[0].cancellation_ratio, None); // genuine exact cancellation
    assert_eq!(d.orders[1].stochastic_absolute_mean, Some(10.));
    assert_eq!(d.orders[1].absolute_integral_estimate, Some(11.));
    assert_eq!(d.orders[1].sampled_sector_local_maximum, Some(5.));
    assert_eq!(d.orders[1].cancellation_ratio, Some(11.));
    assert_eq!(d.orders[1].maximum_ratio, Some(5.));
}
#[test]
fn complete_complex_qmc_common_shifts_reorder_retry_and_checkpoint() {
    let p = problem(2);
    let mut session = QmcSession::democratic(p.clone(), qmc()).unwrap();
    let mut returns = Vec::new();
    while let Some(task) = session.next_work().unwrap() {
        let mut worker = session.worker_context(task.sector_id()).unwrap();
        let id = task.sector_id();
        returns.push(
            worker
                .evaluate(task, |_, out| {
                    out.copy_from_slice(&values(id));
                    Ok::<_, String>(())
                })
                .unwrap(),
        );
    }
    let rejected = returns[0].clone();
    // one completed package is not a complete shared lattice
    session.submit(returns.remove(0)).unwrap();
    assert_eq!(session.weight_diagnostics().unwrap().accepted_points, 0);
    let mut restored = QmcSession::restore(&session.checkpoint().unwrap(), &p).unwrap();
    // restore reissues the same unfinished intervals; existing returns require reservations
    while restored.next_work().unwrap().is_some() {}
    for r in returns.into_iter().rev() {
        restored.submit(r).unwrap();
    }
    let before = restored.checkpoint().unwrap();
    assert!(restored.submit(rejected).is_err());
    assert_eq!(before, restored.checkpoint().unwrap());
    assert_diag(
        &restored.weight_diagnostics().unwrap(),
        &p,
        &restored.estimate().unwrap(),
        32,
    );
    restored.extend_production_shifts(2).unwrap();
    assert_diag(
        &restored.weight_diagnostics().unwrap(),
        &p,
        &restored.estimate().unwrap(),
        32,
    );
    while let Some(task) = restored.next_work().unwrap() {
        let id = task.sector_id();
        let r = restored
            .worker_context(id)
            .unwrap()
            .evaluate(task, |_, out| {
                out.copy_from_slice(&values(id));
                Ok::<_, String>(())
            })
            .unwrap();
        restored.submit(r).unwrap();
    }
    assert_diag(
        &restored.weight_diagnostics().unwrap(),
        &p,
        &restored.estimate().unwrap(),
        64,
    );
    let mut bad = restored.weight_diagnostics().unwrap();
    bad.orders[1].maximum_ratio = Some(9.);
    assert!(
        bad.validate(
            &p.orders,
            &p.components,
            &p.exact_coefficients,
            Some(&restored.estimate().unwrap()),
            64
        )
        .is_err()
    );
}
#[test]
fn asymmetric_discrete_probabilities_keep_abs_full_weight_and_local_max() {
    let p = problem(2);
    let mut s = HavanaDiscreteSession::production(
        p.clone(),
        HavanaDiscreteSettings {
            batch: mc(),
            ..Default::default()
        },
    )
    .unwrap()
    .with_sector_probabilities(&[0.1, 0.9])
    .unwrap();
    let mut expected = 0.;
    let mut count = 0u64;
    while let Some(task) = s.next_work() {
        let r = s
            .worker_context()
            .unwrap()
            .evaluate_with_weight(task, |id, _, weight, out| {
                out.copy_from_slice(&values(id));
                expected += 5. * weight;
                count += 1;
                Ok::<_, String>(())
            })
            .unwrap();
        s.submit(r).unwrap();
    }
    let d = s.weight_diagnostics().unwrap();
    assert!(
        (d.orders[1].stochastic_absolute_mean.unwrap() - expected / count as f64).abs() < 1e-13
    );
    assert!((d.orders[1].sampled_sector_local_maximum.unwrap() - 5.).abs() < 1e-14);
    d.validate(
        &p.orders,
        &p.components,
        &p.exact_coefficients,
        Some(&s.estimate().unwrap()),
        count,
    )
    .unwrap();
    let restored = HavanaDiscreteSession::restore(&s.checkpoint().unwrap(), &p).unwrap();
    assert_eq!(d, restored.weight_diagnostics().unwrap());
}
#[test]
fn havana_production_includes_continuous_weight_excludes_pilot() {
    let p = problem(1);
    let mut s = HavanaSession::pilot(p.clone(), mc()).unwrap();
    while let Some(task) = s.next_work() {
        let r = s
            .worker_context(0)
            .unwrap()
            .evaluate(task, |x, out| {
                out.fill(100. + x[0].powi(4));
                Ok::<_, String>(())
            })
            .unwrap();
        s.submit(r).unwrap();
    }
    assert_eq!(
        s.weight_diagnostics().unwrap().status,
        WeightDiagnosticStatus::PilotOnly
    );
    s.freeze_production(0.5, 32, 2).unwrap();
    let mut sum = 0.;
    let mut max = 0.0_f64;
    while let Some(task) = s.next_work() {
        let r = s
            .worker_context(0)
            .unwrap()
            .evaluate_weighted(task, |_, w, out| {
                for (a, b) in out.iter_mut().zip(values(0)) {
                    *a = b * w;
                }
                sum += 5. * w;
                max = max.max(5. * w);
                Ok::<_, String>(())
            })
            .unwrap();
        s.submit(r).unwrap();
    }
    let d = s.weight_diagnostics().unwrap();
    assert!((d.orders[1].stochastic_absolute_mean.unwrap() - sum / 64.).abs() < 1e-13);
    assert_eq!(d.orders[1].sampled_sector_local_maximum, Some(max));
    let r = HavanaSession::restore(&s.checkpoint().unwrap(), &p).unwrap();
    assert_eq!(d, r.weight_diagnostics().unwrap());
}
fn serial_settings(mc_mode: bool) -> SerialSettings {
    SerialSettings {
        method: if mc_mode {
            SerialMethod::Mc(mc())
        } else {
            SerialMethod::Qmc(qmc())
        },
        double_points: true,
        max_rounds: None,
        max_in_flight: 2,
        target: AccuracyTarget::LaurentOrder(0),
        tolerance: Tolerance::new(0., 0.).unwrap(),
        pilot_iterations: 1,
        learning_rate: 0.5,
    }
}
fn serial_value(task: &SerialTask, id: u64) -> SerialReturn {
    task.evaluate_weighted_batch(3, |_, w, out| {
        for (row, weight) in out.chunks_exact_mut(4).zip(w) {
            for (a, b) in row.iter_mut().zip(values(id)) {
                *a = b * weight;
            }
        }
        Ok::<_, String>(())
    })
    .unwrap()
}
#[test]
fn serial_selected_previous_epoch_and_stale_reservations_match_estimate() {
    for mc_mode in [false, true] {
        let p = problem(2);
        let mut s = SerialSession::new(p.clone(), serial_settings(mc_mode)).unwrap();
        for id in 0..2 {
            for _ in 0..2 {
                let task = s.reserve(id).unwrap().unwrap();
                let r = serial_value(&task, id);
                s.submit(r).unwrap();
            }
        }
        let old = s.weight_diagnostics().unwrap();
        let task = s.reserve(0).unwrap().unwrap();
        let stale = serial_value(&task, 0);
        s.release(&task).unwrap();
        let retry = s.reserve(0).unwrap().unwrap();
        assert!(s.submit(stale).is_err());
        s.submit(serial_value(&retry, 0)).unwrap();
        assert_eq!(old, s.weight_diagnostics().unwrap()); // previous selected until refinement complete
        let bytes = s.checkpoint().unwrap();
        let mut restored = SerialSession::restore(&bytes, &p).unwrap();
        assert_eq!(old, restored.weight_diagnostics().unwrap());
        let task = restored.reserve(0).unwrap().unwrap();
        restored.submit(serial_value(&task, 0)).unwrap();
        let d = restored.weight_diagnostics().unwrap();
        let expected = if mc_mode { 192 } else { 48 };
        assert_diag(&d, &p, &restored.estimate().unwrap(), expected);
    }
}
#[test]
fn genuine_legacy_checkpoint_keeps_digest_and_unavailable_epoch() {
    let bytes = include_bytes!("legacy-serial.json");
    let json: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    let p: IntegrationProblem = serde_json::from_value(json["session"]["problem"].clone()).unwrap();
    let mut s = SerialSession::restore(bytes, &p).unwrap();
    assert_eq!(
        s.weight_diagnostics().unwrap().status,
        WeightDiagnosticStatus::LegacyUnavailable
    );
    let task = s.reserve(7).unwrap().unwrap();
    let r = task
        .evaluate_weighted_batch(2, |x, _, out| {
            for (a, b) in out.iter_mut().zip(x) {
                *a = *b + 1.;
            }
            Ok::<_, String>(())
        })
        .unwrap();
    s.submit(r).unwrap();
    assert_eq!(
        s.weight_diagnostics().unwrap().status,
        WeightDiagnosticStatus::LegacyUnavailable
    );
    let mut tamper = json;
    tamper["session"]["sectors"][0]["points"] = serde_json::json!(8);
    assert!(SerialSession::restore(&serde_json::to_vec(&tamper).unwrap(), &p).is_err());
}
#[test]
fn exact_only_zero_denominator_range_and_pointwise_complex_max() {
    let p = problem(0);
    let s = QmcSession::democratic(p.clone(), qmc()).unwrap();
    let d = s.weight_diagnostics().unwrap();
    assert_eq!(d.orders[0].cancellation_ratio, None);
    assert_eq!(d.orders[1].cancellation_ratio, Some(1.));
    assert_eq!(d.orders[1].sampled_sector_local_maximum, None);
    let layout = WeightLayout::new(&problem(1));
    let mut stats = WeightStatistics::new(2);
    let mut m = Vec::new();
    for values in [[0., 0., 3., 0.], [0., 0., 0., 4.]] {
        layout.magnitudes(&values, 1., &mut m);
        stats.push(&m, &m).unwrap();
    }
    assert_eq!(stats.maxima[1], 4.);
    assert_eq!(stats.means[1].to_f64(), 3.5);
    let mut huge = WeightStatistics::new(1);
    for _ in 0..10 {
        huge.push(&[1e308], &[1e308]).unwrap();
    }
    assert!(huge.finite);
    assert_eq!(huge.means[0].to_f64(), 1e308);
}
#[test]
fn norm_uncertainty_uses_cross_covariance_and_handles_singular_projection() {
    let mut e = VectorEstimate {
        orders: vec![0, 0],
        components: vec![CoefficientComponent::Imag, CoefficientComponent::Real],
        mean: vec![4., 3.],
        standard_error: vec![2., 1.],
        covariance_of_mean: vec![4., -1.5, -1.5, 1.],
        production_complete: true,
    };
    let d = e.complex_norm_uncertainty(0);
    assert_eq!(d.mean_magnitude, Some(5.));
    assert!((d.radial_standard_error.unwrap() - 1.48_f64.sqrt()).abs() < 1e-15);
    assert!((d.joint_relative_error.unwrap() - 5.0_f64.sqrt() / 5.).abs() < 1e-15);
    e.mean = vec![0., 0.];
    assert!(
        e.complex_norm_uncertainty(0)
            .radial_relative_error
            .is_none()
    );
    e.mean = vec![1., 1.];
    e.covariance_of_mean = vec![1., -2., -2., 1.];
    assert!(
        e.complex_norm_uncertainty(0)
            .radial_standard_error
            .is_none()
    );
    e.mean = vec![1e308, 0.];
    e.covariance_of_mean = vec![1e308, 0., 0., 1e308];
    assert!(
        e.complex_norm_uncertainty(0)
            .radial_standard_error
            .unwrap()
            .is_finite()
    );
    assert!(e.complex_norm_uncertainty(0).joint_relative_error.is_some());
    // Both unscaled trace and radial projection overflow, but their standard
    // errors are finite. The off-diagonal entries must participate as well.
    e.mean = vec![1., 1.];
    e.covariance_of_mean = vec![1e308; 4];
    let d = e.complex_norm_uncertainty(0);
    assert!((d.radial_standard_error.unwrap() / 1e154 - 2.0_f64.sqrt()).abs() < 1e-14);
    assert!((d.radial_relative_error.unwrap() / 1e154 - 1.).abs() < 1e-14);
    assert!((d.joint_relative_error.unwrap() / 1e154 - 1.).abs() < 1e-14);
    e.covariance_of_mean.fill(0.);
    let d = e.complex_norm_uncertainty(0);
    assert_eq!(d.radial_standard_error, Some(0.));
    assert_eq!(d.joint_relative_error, Some(0.));
    // Global scaling must not erase a tiny active variance beside a large
    // inactive component. The wide-exponent native fallback preserves it.
    e.mean = vec![1., 0.];
    e.covariance_of_mean = vec![1e-308, 0., 0., 1e308];
    let d = e.complex_norm_uncertainty(0);
    assert!((d.radial_standard_error.unwrap() / 1e-154 - 1.).abs() < 1e-14);
    assert!((d.joint_relative_error.unwrap() / 1e154 - 1.).abs() < 1e-14);
    // A nonzero propagated SE below f64 range is unavailable, never zero.
    e.mean = vec![1e308, 1e-308];
    e.covariance_of_mean = vec![0., 0., 0., 1e308];
    let d = e.complex_norm_uncertainty(0);
    assert_eq!(d.radial_standard_error, None);
    assert_eq!(d.radial_relative_error, None);
    assert!(d.linearization_warning);
    // Even Float128 projection loses this small positive exact residual after
    // signed cancellation. Native rational arithmetic precedes the square root.
    e.mean = vec![1., 1.];
    e.covariance_of_mean = vec![1e-308, -5e307, -5e307, 1e308];
    let d = e.complex_norm_uncertainty(0);
    assert!((d.radial_standard_error.unwrap() / 1e-154 - 0.5_f64.sqrt()).abs() < 1e-14);
    // Near cancellation at ordinary exponents also selects the exact fallback.
    e.covariance_of_mean = vec![1., -1. + f64::EPSILON, -1. + f64::EPSILON, 1.];
    let d = e.complex_norm_uncertainty(0);
    assert_eq!(d.radial_standard_error, Some(f64::EPSILON.sqrt()));
    e.covariance_of_mean = vec![1., -1., -1., 1.];
    assert_eq!(
        e.complex_norm_uncertainty(0).radial_standard_error,
        Some(0.)
    );
}

#[test]
fn diagnostic_overhead_preserves_identical_native_point_stream_and_estimator() {
    let mut settings = qmc();
    settings.points = 4096;
    settings.shifts = 8;
    settings.package_points = 32768;
    let mut session = QmcSession::democratic(problem(1), settings).unwrap();
    let task = session.next_work().unwrap().unwrap();
    let worker = session.worker_context(0).unwrap();
    let mut times = [0.0; 2];
    let mut outputs = Vec::new();
    for enabled in [false, true] {
        let mut worker = worker.clone();
        if !enabled {
            worker.weight_layout = None;
        }
        let start = std::time::Instant::now();
        let r = worker
            .evaluate(task.clone(), |x, out| {
                out.copy_from_slice(&[x[0], -x[0], x[0] * x[0], 1. + x[0]]);
                Ok::<_, String>(())
            })
            .unwrap();
        times[usize::from(enabled)] = start.elapsed().as_secs_f64();
        outputs.push(serde_json::to_vec(&r.partial).unwrap());
    }
    assert_eq!(outputs[0], outputs[1]);
    eprintln!(
        "weight diagnostic timing: 32768 native points, two complex orders, debug disabled={}s enabled={}s; illustrative callback, not a physical integrand benchmark",
        times[0], times[1]
    );
}

#[test]
fn failed_prefixes_and_missing_return_payloads_never_fabricate_statistics() {
    let p = problem(1);
    let mut s = QmcSession::democratic(p.clone(), qmc()).unwrap();
    let task = s.next_work().unwrap().unwrap();
    let before = s.weight_diagnostics().unwrap();
    let mut calls = 0;
    let r = s
        .worker_context(0)
        .unwrap()
        .evaluate(task.clone(), |_, out| {
            calls += 1;
            out.fill(1e100);
            if calls == 3 {
                Err("rejected prefix")
            } else {
                Ok(())
            }
        });
    assert!(r.is_err());
    assert_eq!(before, s.weight_diagnostics().unwrap());
    s.retry(&task).unwrap();
    let task = s.next_work().unwrap().unwrap();
    let r = s
        .worker_context(0)
        .unwrap()
        .evaluate(task, |_, out| {
            out.copy_from_slice(&values(0));
            Ok::<_, String>(())
        })
        .unwrap();
    // Historical/manual native partial path carries no pointwise information.
    let old = QmcReturn::from_partial(r.task, r.partial, r.worker_seconds).unwrap();
    s.submit(old).unwrap();
    while let Some(task) = s.next_work().unwrap() {
        let r = s
            .worker_context(0)
            .unwrap()
            .evaluate(task, |_, out| {
                out.copy_from_slice(&values(0));
                Ok::<_, String>(())
            })
            .unwrap();
        s.submit(r).unwrap();
    }
    assert_eq!(
        s.weight_diagnostics().unwrap().status,
        WeightDiagnosticStatus::LegacyUnavailable
    );
    assert!(
        s.weight_diagnostics()
            .unwrap()
            .orders
            .iter()
            .all(|r| r.stochastic_absolute_mean.is_none())
    );
    let restored = QmcSession::restore(&s.checkpoint().unwrap(), &p).unwrap();
    assert_eq!(
        s.weight_diagnostics().unwrap(),
        restored.weight_diagnostics().unwrap()
    );
}
#[test]
fn diagnostic_interval_boundaries_are_checked_independently() {
    assert!(WeightPartial::new(u64::MAX, 2, 8, 1).is_err());
    assert!(WeightPartial::new(0, 1, 0, 1).is_err());
    assert!(WeightPartial::new(0, 0, 8, 1).is_err());
    let mut p = WeightPartial::new(u64::MAX - 3, 3, 8, 1).unwrap();
    for _ in 0..3 {
        p.groups[0].push(&[1.], &[1.]).unwrap();
    }
    p.validate(u64::MAX - 3, 3, 8, 1).unwrap();
    assert!(p.validate(u64::MAX, 2, 8, 1).is_err());
}

#[test]
fn mean_only_accumulator_count_overflow_is_atomic() {
    let mut full = WeightStatistics::new(1);
    full.points = u64::MAX;
    let before = serde_json::to_vec(&full).unwrap();
    assert!(full.push(&[1.], &[1.]).is_err());
    assert_eq!(before, serde_json::to_vec(&full).unwrap());
    let mut one = WeightStatistics::new(1);
    one.push(&[1.], &[1.]).unwrap();
    assert!(full.merge(&one).is_err());
    assert_eq!(before, serde_json::to_vec(&full).unwrap());
}
