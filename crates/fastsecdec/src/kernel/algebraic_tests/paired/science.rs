use super::*;

fn check_complete_vector(kind: u8) {
    use fastsecdec::{
        integration::{Periodization, QmcSession, QmcSettings, RuleSource},
        results::{KernelResultManifest, ResultScope},
    };
    let a = symgcad::algebra::Algebra::new(&["x".into(), "y".into()]).unwrap();
    let p = a.parse("y^6-y+(1+x)/16").unwrap();
    let mut endpoint_roots = Vec::new();
    let mut intervals = Vec::new();
    for x in [0, 1] {
        let fiber = a
            .specialize_univariate(&p, 1, &[Rational::from(x)])
            .unwrap();
        let mut roots = symgcad::roots::isolate_union(vec![(0, fiber)]).unwrap();
        assert_eq!(roots.len(), 2);
        for root in &mut roots {
            for _ in 0..112 {
                symgcad::roots::refine_once(root);
            }
            endpoint_roots
                .push(((&root.interval.0 + &root.interval.1) / Rational::from(2)).to_f64());
            intervals.push([root.interval.0.to_string(), root.interval.1.to_string()]);
        }
    }
    let imaginary = 16.
        * std::f64::consts::PI
        * ((endpoint_roots[2] - endpoint_roots[0]) - (endpoint_roots[3] - endpoint_roots[1]));
    assert!(imaginary > 0.);
    {
        let started = std::time::Instant::now();
        let bytes = archived(kind, true);
        let kernels = KernelSet::from_bytes(&bytes).unwrap();
        let problem = KernelResultManifest::integration_problem_from_kernels(
            &kernels,
            &ResultScope::FullIntegral,
            kernels.content_id(),
        )
        .unwrap();
        let orders = problem.orders.clone();
        if kind == 2 {
            assert_eq!(orders, [0, 0, 1, 1, 2, 2]);
        } else {
            assert_eq!(orders, [-1, -1, 0, 0, 1, 1, 2, 2]);
        }
        let mut contexts = (0..kernels.sectors().len())
            .map(|i| kernels.evaluation_context(i, Default::default()).unwrap())
            .collect::<Vec<_>>();
        let settings = QmcSettings {
            points: if kind == 0 { 1024 } else { 256 },
            shifts: 4,
            package_points: 128,
            seed: 202610119061,
            rule: if kind == 0 {
                RuleSource::Kuo
            } else {
                RuleSource::Supplied(vec![1, 73])
            },
            periodization: Periodization::Korobov3,
        };
        let mut session = QmcSession::democratic(problem, settings.clone()).unwrap();
        let mut rescues = 0usize;
        while let Some(task) = session.next_work().unwrap() {
            let id = task.sector_id();
            let result = session
                .worker_context(id)
                .unwrap()
                .evaluate_weighted_batch(task, 64, |points, weights, output| {
                    let reports = contexts[id as usize].evaluate_weighted_batch(points, weights, output)
                        .inspect_err(|error| {
                            let row = error.completed.len();
                            eprintln!("FAILED_ROW {}", serde_json::json!({"kind":kind,"sector":id,"row":row,"point": &points[2*row..2*row+2],"weight":weights[row],"error":error.to_string()}));
                        })?;
                    rescues += reports.iter().filter(|r| r.precision.rescued).count();
                    Ok::<_, fastsecdec::kernel::BatchEvaluationError>(())
                })
                .unwrap();
            session.submit(result).unwrap();
        }
        let estimate = session.estimate().unwrap();
        estimate.validate().unwrap();
        assert!(estimate.production_complete);
        assert_eq!(
            estimate.covariance_of_mean.len(),
            orders.len() * orders.len()
        );
        assert_eq!(
            session.snapshot().unwrap().completed_points,
            4 * settings.points * settings.shifts as u64
        );
        let check = |i: usize, oracle: f64| {
            assert!(
                (estimate.mean[i] - oracle).abs() <= 8. * estimate.standard_error[i] + 2e-8,
                "kind={kind} component={i} oracle={oracle}: {estimate:?}"
            )
        };
        if kind != 2 {
            check(0, 0.);
            check(1, 0.);
        }
        if kind == 0 {
            check(3, imaginary);
            assert!(estimate.standard_error[3] < 0.02);
        } else {
            let l = 2.0f64.ln();
            let reference = if kind == 1 {
                vec![0.; 8]
            } else {
                vec![1., 0., 1. + 2. * l, 0., (1. + l).powi(2), 0.]
            };
            for (i, value) in reference.into_iter().enumerate() {
                check(i, value);
            }
            assert!(estimate.standard_error[2] < 0.03);
        }
        let reference = match kind {
            0 => {
                serde_json::json!({"finite_imaginary":imaginary,"endpoint_root_intervals":intervals})
            }
            1 => serde_json::json!({"complete_vector":vec![0.;8]}),
            _ => {
                serde_json::json!({"complete_vector":[1.,0.,1.+2.*2.0f64.ln(),0.,(1.+2.0f64.ln()).powi(2),0.]})
            }
        };
        println!(
            "{}",
            serde_json::json!({"fixture":kind,"settings":settings,"estimate":estimate,"rescue_rows":rescues,"seconds":started.elapsed().as_secs_f64(),"independent_reference":reference})
        );
    }
}

#[test]
fn scalar_complete_vector_and_independent_phase() {
    check_complete_vector(0);
}
#[test]
fn derivative_complete_vector() {
    check_complete_vector(1);
}
#[test]
fn regulated_derivative_complete_vector() {
    check_complete_vector(2);
}
