use super::*;
use crate::{
    integration::{Periodization, QmcSession, QmcSettings, RuleSource},
    results::{KernelResultManifest, ResultScope},
};
use std::io::Cursor;
fn settings() -> CompilationSettings {
    CompilationSettings {
        backend: EvaluatorBackend::Eager,
        ..Default::default()
    }
}
fn options() -> GenerationOptions {
    GenerationOptions {
        mode: GenerationMode::Symbolic,
        max_order: 1,
        ..Default::default()
    }
}
fn compile(family: &NormalizedFamily) -> KernelSet {
    let continued = family.continue_symbolically(&options()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    KernelSet::compile_threshold_family(
        &continued,
        dir.path(),
        1,
        PrecisionPolicy::default(),
        settings(),
    )
    .unwrap()
}
fn oracle(value: f64, numerator: bool) -> f64 {
    let algebra = symgcad::algebra::Algebra::new(&["y".into()]).unwrap();
    let polynomial = algebra
        .parse(&format!(
            "y^5+y-{}",
            Rational::try_from(value - 2.).unwrap()
        ))
        .unwrap();
    let fiber = algebra.specialize_univariate(&polynomial, 0, &[]).unwrap();
    let mut roots = symgcad::roots::isolate_union(vec![(0, fiber)]).unwrap();
    assert_eq!(roots.len(), 1);
    for _ in 0..160 {
        symgcad::roots::refine_once(&mut roots[0]);
    }
    let root = (&roots[0].interval.0 + &roots[0].interval.1) / Rational::from(2);
    let q = Rational::try_from(value).unwrap();
    let n = if numerator {
        (Rational::one() + &q) * (Rational::one() + q * &root)
    } else {
        Rational::one()
    };
    std::f64::consts::PI * (n / (Rational::one() + Rational::from(5) * root.pow(4))).to_f64()
}
#[test]
fn parameter_numerator_and_prefactor_match_fixed_source_full_vectors() {
    let (family, p) = family_kind(true, None);
    let mut set = compile(&family);
    drop(family);
    let identity = set.template_content_id().to_owned();
    let ir = programs(&set);
    for value in [2.375, 2.4, 2.3125, 2.375] {
        set.bind_parameters(&BTreeMap::from([(p, value)])).unwrap();
        let (fixed, _) = family_kind(true, Some(value));
        let mut reference = compile(&fixed);
        drop(fixed);
        for x in [0.125, 0.37, 0.875] {
            let mut a = set.exact_coefficients().to_vec();
            let mut b = reference.exact_coefficients().to_vec();
            for s in set.sectors_mut() {
                let mut out = vec![0.; 6];
                s.replay_scaled(&[x], &mut out, 1., 128).unwrap();
                for (v, w) in a.iter_mut().zip(out) {
                    *v += w;
                }
            }
            for s in reference.sectors_mut() {
                let mut out = vec![0.; 6];
                s.replay_scaled(&[x], &mut out, 1., 128).unwrap();
                for (v, w) in b.iter_mut().zip(out) {
                    *v += w;
                }
            }
            for (v, w) in a.iter().zip(&b) {
                assert!(
                    (v - w).abs() < 2e-11 * (1. + w.abs()),
                    "{value} {x}: {a:?} {b:?}"
                );
            }
            assert!((a[3] - oracle(value, true)).abs() < 2e-11);
        }
        assert_eq!(set.template_content_id(), identity);
        assert_eq!(programs(&set), ir);
    }
}
#[test]
fn complete_vector_covariance_checkpoint_and_four_native_execution_lanes() {
    let (family, p) = family_kind(true, None);
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
    for value in [2.375, 2.3125] {
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
                if value != 2.375
                    && let Some(checkpoint) = &prior_checkpoint
                {
                    assert!(QmcSession::restore(checkpoint, &problem).is_err());
                }
                let settings = QmcSettings {
                    points: 256,
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
                assert_eq!(count, 2048);
                for i in 0..2 {
                    assert!(
                        estimate.mean[i].abs() < 8. * estimate.standard_error[i] + 2e-11,
                        "{estimate:?}"
                    );
                }
                let expected = oracle(value, true);
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
                if value == 2.375 {
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
