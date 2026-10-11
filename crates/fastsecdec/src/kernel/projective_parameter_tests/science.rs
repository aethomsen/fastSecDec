use super::*;
use crate::{
    integration::{Periodization, QmcSession, QmcSettings, RuleSource},
    results::{KernelResultManifest, ResultScope},
};
use std::io::Cursor;
use symbolica::domains::float::Complex;
fn settings() -> CompilationSettings {
    CompilationSettings {
        backend: EvaluatorBackend::Eager,
        ..Default::default()
    }
}
fn master(value: f64) -> [Complex<f64>; 3] {
    let mut out = [Complex::new(0., 0.); 3];
    oneloop::evaluate_with_backend(
        oneloop::ScalarIntegral::B0,
        &[value, 3., 3., 1.].map(|v| Complex::new(v, 0.)),
        &mut out,
        oneloop::EvaluationBackend::Expression,
    )
    .unwrap();
    if value == 16. {
        assert!((out[0].re - (2. - 1.5 * 3f64.ln())).abs() < 1e-12);
        assert!((out[0].im - std::f64::consts::PI / 2.).abs() < 1e-12);
    }
    out
}
#[test]
fn complete_vector_covariance_checkpoint_and_four_native_execution_lanes() {
    let (family, p) = family();
    let continued = family.continue_symbolically(&options()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let resident = KernelSet::compile_threshold_family(
        &continued,
        dir.path(),
        1,
        PrecisionPolicy::default(),
        settings(),
    )
    .unwrap();
    let resident_bytes = resident.to_bytes().unwrap();
    let identity = resident.template_content_id().to_owned();
    let ir = programs(&resident);
    let plan_dir = tempfile::tempdir().unwrap();
    let plan = ThresholdCompilationPlan::prepare_family(
        &continued,
        plan_dir.path(),
        1,
        PrecisionPolicy::default(),
        settings(),
    )
    .unwrap();
    let owner = Arc::downgrade(family.owner());
    drop(continued);
    drop(family);
    assert!(owner.upgrade().is_none());
    let descriptor = plan.publication_descriptor().unwrap();
    let publication = ThresholdPublicationPlan::from_trusted_preparer(
        plan_dir.path(),
        descriptor.clone(),
        &descriptor.source_identity,
        &descriptor.prepared_identity,
        16 * 1024 * 1024,
    )
    .unwrap();
    let mut writer = publication.archive_writer(Cursor::new(Vec::new())).unwrap();
    for i in (0..plan.job_count()).rev() {
        let work: ThresholdCompilationWork =
            serde_json::from_slice(&serde_json::to_vec(&plan.work(i).unwrap()).unwrap()).unwrap();
        let mut bytes = Vec::new();
        assert!(work.compile_record(plan_dir.path(), 1, &mut bytes).is_err());
        assert!(bytes.is_empty());
        let receipt = work
            .compile_record(plan_dir.path(), 16 * 1024 * 1024, &mut bytes)
            .unwrap();
        let receipt: ThresholdWorkReceipt =
            serde_json::from_slice(&serde_json::to_vec(&receipt).unwrap()).unwrap();
        plan.validate_work_receipt(&receipt).unwrap();
        publication.validate_work_receipt(&receipt).unwrap();
        for mutation in 0..4 {
            let mut json = serde_json::to_value(&receipt).unwrap();
            match mutation {
                0 => {
                    json["record"]["threshold"]["runtime_guard"]["semantic"] = "0".repeat(64).into()
                }
                1 => json["record"]["threshold"]["runtime_guard"] = serde_json::Value::Null,
                2 => json["record"]["version"] = 3.into(),
                _ => {
                    json["record"]["runtime_parameters"] = serde_json::json!(["foreign::p"]);
                    json["record"]["threshold"]["runtime_guard"]["parameters"] =
                        serde_json::json!(["foreign::p"]);
                }
            }
            let altered: ThresholdWorkReceipt = serde_json::from_value(json).unwrap();
            assert!(plan.validate_work_receipt(&altered).is_err());
            assert!(publication.validate_work_receipt(&altered).is_err());
        }
        publication
            .append_work_record(&mut writer, &mut bytes.as_slice(), receipt)
            .unwrap();
    }
    let (indexed, _) = writer.finish().unwrap();
    let indexed = indexed.into_inner();
    let detached = KernelSet::from_bytes(&indexed).unwrap();
    assert_eq!(detached.template_content_id(), identity);
    assert_eq!(programs(&detached), ir);
    let mut prior_checkpoint: Option<Vec<u8>> = None;
    let mut results = Vec::new();
    for value in [16., 17.] {
        let mut reference = None;
        for (generation, bytes) in [("resident", &resident_bytes), ("detached", &indexed)] {
            for selected in [false, true] {
                let mut kernels = KernelSet::from_bytes(bytes).unwrap();
                kernels
                    .bind_parameters(&BTreeMap::from([(p, value)]))
                    .unwrap();
                let problem = KernelResultManifest::integration_problem_from_kernels(
                    &kernels,
                    &ResultScope::FullIntegral,
                    kernels.content_id(),
                )
                .unwrap();
                assert_eq!(problem.orders, [-1, -1, 0, 0, 1, 1]);
                if value != 16.
                    && let Some(checkpoint) = &prior_checkpoint
                {
                    assert!(QmcSession::restore(checkpoint, &problem).is_err());
                }
                let settings = QmcSettings {
                    points: 512,
                    shifts: 4,
                    package_points: 64,
                    seed: 202610110651,
                    rule: RuleSource::Supplied(vec![1]),
                    periodization: Periodization::Korobov3,
                };
                let mut session = QmcSession::democratic(problem.clone(), settings).unwrap();
                let mut contexts = (0..kernels.sectors().len())
                    .map(|i| kernels.evaluation_context(i, Default::default()).unwrap())
                    .collect::<Vec<_>>();
                let mut archive = indexed::ProgramArchiveReader::from_reader(
                    Cursor::new(indexed.clone()),
                    KernelLoadOptions { validate: true },
                )
                .unwrap();
                let mut restored = false;
                let started = std::time::Instant::now();
                let mut count = 0u64;
                while let Some(task) = session.next_work().unwrap() {
                    let id = task.sector_id();
                    let mut local = if selected {
                        let mut set = archive
                            .select(ProgramRecipe::ThresholdV1)
                            .unwrap()
                            .load_sector(id as usize)
                            .unwrap();
                        set.bind_parameters(&BTreeMap::from([(p, value)])).unwrap();
                        Some(set.evaluation_context(0, Default::default()).unwrap())
                    } else {
                        None
                    };
                    let context = local.as_mut().unwrap_or(&mut contexts[id as usize]);
                    let result = session
                        .worker_context(id)
                        .unwrap()
                        .evaluate_weighted_batch(task, 64, |points, weights, output| {
                            let reports =
                                context.evaluate_weighted_batch(points, weights, output)?;
                            count += reports.len() as u64;
                            Ok::<_, BatchEvaluationError>(())
                        })
                        .unwrap();
                    session.submit(result).unwrap();
                    if !restored {
                        let checkpoint = session.checkpoint().unwrap();
                        session = QmcSession::restore(&checkpoint, &problem).unwrap();
                        restored = true;
                    }
                }
                assert!(session.is_complete());
                let estimate = session.estimate().unwrap();
                estimate.validate().unwrap();
                assert!(estimate.production_complete);
                assert_eq!(estimate.covariance_of_mean.len(), 36);
                assert_eq!(count, 8192);
                let expected_master = master(value);
                for (order, reference) in [0, -1, -2].into_iter().zip(expected_master) {
                    for (component, reference) in [
                        (crate::status::CoefficientComponent::Real, reference.re),
                        (crate::status::CoefficientComponent::Imag, reference.im),
                    ] {
                        let actual = estimate
                            .orders
                            .iter()
                            .zip(&estimate.components)
                            .position(|(o, c)| *o == order && *c == component);
                        let (mean, error) = actual
                            .map(|i| (estimate.mean[i], estimate.standard_error[i]))
                            .unwrap_or((0., 0.));
                        assert!(
                            (mean - reference).abs() <= 8. * error + 2e-8 * (1. + reference.abs()),
                            "s={value} eps^{order} {component:?}: {mean} +/- {error}, OneLOop {reference}"
                        );
                    }
                }
                let expected = expected_master[0].im;
                assert!(
                    (estimate.mean[3] - expected).abs() < 8. * estimate.standard_error[3] + 2e-10,
                    "{estimate:?} vs {expected}"
                );
                assert!(estimate.standard_error[3] < 0.01);
                let scientific = (
                    estimate.mean.clone(),
                    estimate.standard_error.clone(),
                    estimate.covariance_of_mean.clone(),
                );
                if let Some(ref old) = reference {
                    assert_eq!(old, &scientific);
                } else {
                    reference = Some(scientific);
                }
                results.push(serde_json::json!({"parameter":value,"generation":generation,"selected_record_execution":selected,"seconds":started.elapsed().as_secs_f64(),"accepted_rows":count,"finite_imaginary_oracle":expected,"estimate":estimate}));
                if value == 16. {
                    prior_checkpoint = Some(session.checkpoint().unwrap());
                }
            }
        }
    }
    println!(
        "{}",
        serde_json::json!({"template":identity,"compiled_programs_unchanged":true,"runs":results})
    );
}
