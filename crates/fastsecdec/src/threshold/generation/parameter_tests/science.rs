use super::*;
use crate::{
    integration::{Periodization, QmcSession, QmcSettings, RuleSource},
    kernel::BatchEvaluationError,
    results::{KernelResultManifest, ResultScope},
};

#[test]
fn automatic_projective_template_retains_complete_qmc_covariance_and_oneloop() {
    let directory = tempfile::tempdir().unwrap();
    let (source, options, p) = input(true);
    let receipt = prepare(source, options, directory.path(), |_| {
        ControlFlow::Continue(())
    })
    .unwrap()
    .receipt;
    let mut kernels = compile(directory.path(), &receipt);
    let template = kernels.template_content_id().to_owned();
    let mut previous: Option<Vec<u8>> = None;
    let mut runs = Vec::new();
    for value in [16., 17.] {
        kernels
            .bind_parameters(&BTreeMap::from([(p, value)]))
            .unwrap();
        assert_eq!(kernels.template_content_id(), template);
        let problem = KernelResultManifest::integration_problem_from_kernels(
            &kernels,
            &ResultScope::FullIntegral,
            kernels.content_id(),
        )
        .unwrap();
        assert_eq!(problem.orders, [-1, -1, 0, 0, 1, 1]);
        if let Some(checkpoint) = &previous {
            assert!(QmcSession::restore(checkpoint, &problem).is_err());
        }
        let mut session = QmcSession::democratic(
            problem.clone(),
            QmcSettings {
                points: 512,
                shifts: 4,
                package_points: 64,
                seed: 202610111413,
                rule: RuleSource::Supplied(vec![1]),
                periodization: Periodization::Korobov3,
            },
        )
        .unwrap();
        let mut contexts = (0..kernels.sectors().len())
            .map(|i| kernels.evaluation_context(i, Default::default()).unwrap())
            .collect::<Vec<_>>();
        let mut count = 0;
        let mut restored = false;
        let start = std::time::Instant::now();
        while let Some(task) = session.next_work().unwrap() {
            let id = task.sector_id();
            let result = session
                .worker_context(id)
                .unwrap()
                .evaluate_weighted_batch(task, 64, |points, weights, output| {
                    let reports =
                        contexts[id as usize].evaluate_weighted_batch(points, weights, output)?;
                    count += reports.len();
                    Ok::<_, BatchEvaluationError>(())
                })
                .unwrap();
            session.submit(result).unwrap();
            if !restored {
                session = QmcSession::restore(&session.checkpoint().unwrap(), &problem).unwrap();
                restored = true;
            }
        }
        let estimate = session.estimate().unwrap();
        estimate.validate().unwrap();
        assert!(session.is_complete() && estimate.production_complete);
        assert_eq!(estimate.covariance_of_mean.len(), 36);
        assert_eq!(count, 8192);
        let mut master = [Complex::new(0., 0.); 3];
        oneloop::evaluate_with_backend(
            oneloop::ScalarIntegral::B0,
            &[value, 3., 3., 1.].map(|v| Complex::new(v, 0.)),
            &mut master,
            oneloop::EvaluationBackend::Expression,
        )
        .unwrap();
        for (order, reference) in [0, -1, -2].into_iter().zip(master) {
            for (component, reference) in [
                (crate::status::CoefficientComponent::Real, reference.re),
                (crate::status::CoefficientComponent::Imag, reference.im),
            ] {
                let row = estimate
                    .orders
                    .iter()
                    .zip(&estimate.components)
                    .position(|(o, c)| *o == order && *c == component);
                let (mean, error) = row
                    .map(|i| (estimate.mean[i], estimate.standard_error[i]))
                    .unwrap_or((0., 0.));
                assert!(
                    (mean - reference).abs() <= 8. * error + 2e-8 * (1. + reference.abs()),
                    "s={value} eps^{order} {component:?}: {mean} +/- {error}, OneLOop={reference}"
                );
            }
        }
        assert!(estimate.standard_error[3] < 0.01);
        previous = Some(session.checkpoint().unwrap());
        runs.push(serde_json::json!({"parameter":value,"accepted_rows":count,
            "seconds":start.elapsed().as_secs_f64(),"finite_imaginary_oracle":master[0].im,
            "estimate":estimate}));
    }
    println!(
        "{}",
        serde_json::json!({"automatic_prepared_template":template,"runs":runs,
        "analytic_scope":"OneLOop finite real/imaginary and poles; full epsilon1 retained, no analytic epsilon1 claim"})
    );
}
