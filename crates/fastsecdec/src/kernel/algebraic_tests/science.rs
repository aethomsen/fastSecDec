use crate::{
    integration::{Periodization, QmcSession, QmcSettings, RuleSource},
    kernel::{KernelSet, ProgramRecipe, indexed::ProgramArchiveReader},
    results::{KernelResultManifest, ResultScope},
};
use std::{collections::BTreeMap, io::Cursor};
use symbolica::{
    atom::{Atom, AtomCore},
    symbol,
};
#[test]
fn full_laurent_vector_matches_independent_quintic_integral() {
    quintic(1);
}
#[test]
fn affine_v2_full_vector_and_fresh_restore_match_quintic_oracle() {
    quintic(2);
}
#[test]
fn shifted_v3_full_vector_and_fresh_restore_match_quintic_oracle() {
    quintic(3);
}
fn quintic(version: u32) {
    let temporary = tempfile::tempdir().unwrap();
    super::artifacts::factory_version(temporary.path(), version);
    if version >= 2 {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "kernel::algebraic_tests::artifacts::restore_child",
                "--nocapture",
            ])
            .env("FASTSECDEC_ALGEBRAIC_CHILD", temporary.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        super::artifacts::restore(temporary.path());
        if let Some(path) = std::env::var_os("FASTSECDEC_AFFINE_ARTIFACT_EXPORT") {
            std::fs::create_dir_all(&path).unwrap();
            for backend in ["Eager", "Symjit"] {
                std::fs::copy(
                    temporary.path().join(format!("family-{backend}.bin")),
                    std::path::Path::new(&path).join(format!("family-{backend}.bin")),
                )
                .unwrap();
            }
        }
    }
    let directory = temporary.path().display().to_string();
    let algebra = symgcad::algebra::Algebra::new(&["y".into()]).unwrap();
    let p = algebra.parse("y^5+y-1").unwrap();
    let fiber = algebra.specialize_univariate(&p, 0, &[]).unwrap();
    let mut roots = symgcad::roots::isolate_union(vec![(0, fiber)]).unwrap();
    assert_eq!(roots.len(), 1);
    for _ in 0..96 {
        symgcad::roots::refine_once(&mut roots[0]);
    }
    let midpoint = (&roots[0].interval.0 + &roots[0].interval.1)
        / symbolica::domains::rational::Rational::from(2);
    let oracle = std::f64::consts::PI * midpoint.to_f64();
    let (x, y, e) = symbol!("qmc_oracle::x", "qmc_oracle::y", "qmc_oracle::e");
    let f = Atom::var(y).pow(5) + Atom::var(y) - Atom::var(x);
    let primitive = f.clone().pow(-Atom::var(e)) / Atom::var(e);
    let expected = f.pow(-Atom::one() - Atom::var(e));
    assert!(
        (primitive.derivative(x) / expected - Atom::one())
            .expand()
            .together()
            .cancel()
            .is_zero()
    );
    let mut rows = Vec::new();
    for backend in ["Eager"] {
        let bytes = std::fs::read(format!("{directory}/family-{backend}.bin")).unwrap();
        let kernels = KernelSet::from_bytes(&bytes).unwrap();
        let problem = KernelResultManifest::integration_problem_from_kernels(
            &kernels,
            &ResultScope::FullIntegral,
            kernels.content_id(),
        )
        .unwrap();
        assert_eq!(problem.orders, [-1, -1, 0, 0, 1, 1]);
        let mut archive =
            ProgramArchiveReader::from_reader(Cursor::new(bytes), Default::default()).unwrap();
        let mut reader = archive.select(ProgramRecipe::ThresholdV1).unwrap();
        let mut contexts = (0..kernels.sectors().len())
            .map(|i| {
                reader
                    .load_sector(i)
                    .unwrap()
                    .evaluation_context(0, Default::default())
                    .unwrap()
            })
            .collect::<Vec<_>>();
        drop(reader);
        drop(kernels);
        let settings = QmcSettings {
            points: 1024,
            shifts: 4,
            package_points: 128,
            seed: 202610114101,
            rule: RuleSource::Kuo,
            periodization: Periodization::Korobov3,
        };
        let mut session = QmcSession::democratic(problem, settings.clone()).unwrap();
        let started = std::time::Instant::now();
        let mut counts = BTreeMap::new();
        let mut replayed = 0u64;
        let mut checks = 0u64;
        let mut rescued = 0u64;
        while let Some(task) = session.next_work().unwrap() {
            let id = task.sector_id();
            let result = session
                .worker_context(id)
                .unwrap()
                .evaluate_weighted_batch(task, 64, |points, weights, output| {
                    let reports =
                        contexts[id as usize].evaluate_weighted_batch(points, weights, output)?;
                    for report in reports {
                        *counts.entry(report.precision.bits).or_insert(0u64) += 1;
                        replayed += u64::from(report.replayed);
                        checks += u64::from(report.weighted_check);
                        rescued += u64::from(report.precision.rescued);
                    }
                    Ok::<_, crate::kernel::BatchEvaluationError>(())
                })
                .unwrap();
            session.submit(result).unwrap();
        }
        let seconds = started.elapsed().as_secs_f64();
        assert!(session.is_complete());
        let estimate = session.estimate().unwrap();
        estimate.validate().unwrap();
        assert!(estimate.production_complete);
        assert_eq!(estimate.covariance_of_mean.len(), 36);
        for i in 0..2 {
            assert!(
                estimate.mean[i].abs() <= 8. * estimate.standard_error[i] + 2e-11,
                "pole {i}: {estimate:?}"
            );
        }
        assert!(
            (estimate.mean[3] - oracle).abs() <= 8. * estimate.standard_error[3] + 2e-10,
            "oracle {oracle} vs {estimate:?}"
        );
        assert!(estimate.standard_error[3] < 0.01);
        assert_eq!(session.snapshot().unwrap().completed_points, 8192);
        assert!(
            contexts
                .iter()
                .all(|c| c.contour_validation_report().is_none())
        );
        let timing = contexts
            .iter()
            .map(|c| c.evaluation_metrics())
            .collect::<Vec<_>>();
        rows.push(serde_json::json!({"backend":backend,"settings":settings,"estimate":estimate,"completed_points":8192,"seconds":seconds,"bits_counts":counts,"rescued":rescued,"replayed":replayed,"weighted_checks":checks,"evaluator_times":timing}));
    }
    println!(
        "{}",
        serde_json::json!({"recipe_version":version,"native_primitive_identity":true,"independent_root_interval":[roots[0].interval.0.to_string(),roots[0].interval.1.to_string()],"finite_imaginary_oracle":oracle,"runs":rows})
    );
}
