use super::*;
use crate::{
    kernel::{KernelLoadOptions, KernelSet, ThresholdPublicationPlan},
    parametric::*,
};
use std::io::Cursor;
use symbolica::{
    atom::{Atom, AtomCore},
    domains::float::Complex,
    symbol,
};

#[path = "parameter_tests.rs"]
mod parameter_tests;

#[path = "regular_tests.rs"]
mod regular_tests;

#[path = "graph_tests.rs"]
mod graph_tests;

fn analytic() -> (Arc<ParametricIntegrand>, PreparationOptions) {
    let (x, a, eps, t) = symbol!(
        "preparation_adapter::x",
        "preparation_adapter::a",
        "preparation_adapter::eps",
        "preparation_adapter::t"
    );
    let input = ParametricIntegrand::new(
        vec![x],
        eps,
        ParametricDomain::UnitCube,
        vec![ParametricTerm::new(
            Atom::one(),
            vec![Atom::Zero],
            vec![
                PolynomialFactor::new(
                    Atom::var(x) - Atom::var(a),
                    -Atom::one() - Atom::var(eps),
                    FactorRole::Singularity,
                )
                .with_semantics(FactorSemantics::Causal),
                PolynomialFactor::new(
                    Atom::one() + Atom::i() * Atom::var(a) * Atom::var(x),
                    Atom::one(),
                    FactorRole::Polynomial,
                ),
            ],
        )],
    )
    .unwrap();
    let mut options = PreparationOptions::new(t);
    options.threshold.kinematics = gcad::GcadKinematics {
        runtime_parameters: vec![a],
        strict_positive: vec![Atom::var(a), Atom::one() - Atom::var(a)],
        ..Default::default()
    };
    options.fixed_fiber.insert(a, Rational::from((1, 4)));
    options.generation.max_order = 1;
    (Arc::new(input), options)
}
fn publication(root: &Path, receipt: &PreparationReceipt) -> ThresholdPublicationPlan {
    ThresholdPublicationPlan::from_trusted_preparer(
        &root.join(&receipt.work_directory),
        receipt.publication.clone(),
        &receipt.publication.source_identity,
        &receipt.publication.prepared_identity,
        16 * 1024 * 1024,
    )
    .unwrap()
}
fn compile(root: &Path, receipt: &PreparationReceipt) -> KernelSet {
    let plan = publication(root, receipt);
    let mut archive = plan.archive_writer(Cursor::new(Vec::new())).unwrap();
    for index in (0..plan.job_count()).rev() {
        let mut record = Vec::new();
        let work = plan.work(index).unwrap();
        let receipt = work
            .compile_record(
                &root.join(&receipt.work_directory),
                16 * 1024 * 1024,
                &mut record,
            )
            .unwrap();
        plan.append_work_record(&mut archive, &mut record.as_slice(), receipt)
            .unwrap();
    }
    let (bytes, catalogue) = archive.finish().unwrap();
    let kernels = KernelSet::from_bytes_with_options(
        &bytes.into_inner(),
        KernelLoadOptions { validate: true },
    )
    .unwrap();
    assert_eq!(
        kernels.content_id(),
        catalogue
            .recipe(crate::kernel::ProgramRecipe::ThresholdV1)
            .unwrap()
            .content_id
    );
    assert!(
        kernels
            .threshold_metadata()
            .unwrap()
            .full_original_scope()
            .unwrap()
    );
    kernels
}
#[test]
fn detached_publication_and_resume_reverify_without_solve() {
    let root = tempfile::tempdir().unwrap();
    let (input, options) = analytic();
    let weak = Arc::downgrade(&input);
    let mut stages = Vec::new();
    let prepared = prepare(input, options, root.path(), |p| {
        stages.push(p.stage);
        ControlFlow::Continue(())
    })
    .unwrap();
    assert!(weak.upgrade().is_none());
    assert!(stages.contains(&Stage::Solve));
    assert_eq!(stages.last(), Some(&Stage::Complete));
    let serialized = serde_json::to_vec(&prepared.receipt).unwrap();
    let receipt: PreparationReceipt = serde_json::from_slice(&serialized).unwrap();
    let mut original = compile(root.path(), &receipt);
    let mut resumed_stages = Vec::new();
    let resumed = resume(
        root.path(),
        &receipt,
        "resumed-vectors",
        &receipt.publication.source_identity,
        &receipt.publication.prepared_identity,
        16 * 1024 * 1024,
        |p| {
            resumed_stages.push(p.stage);
            ControlFlow::Continue(())
        },
    )
    .unwrap();
    assert!(!resumed_stages.contains(&Stage::Solve));
    assert!(resumed_stages.contains(&Stage::Verify));
    assert_eq!(
        resumed.receipt.publication.prepared_identity,
        receipt.publication.prepared_identity
    );
    // A fresh trusted verifier response permits structural readmission of the
    // retained old plan; no global proof is reconstructed in this coordinator.
    ThresholdPublicationPlan::from_trusted_preparer(
        &root.path().join(&receipt.work_directory),
        receipt.publication.clone(),
        &resumed.receipt.publication.source_identity,
        &resumed.receipt.publication.prepared_identity,
        16 * 1024 * 1024,
    )
    .unwrap();
    let mut second = compile(root.path(), &resumed.receipt);
    assert_eq!(original.content_id(), second.content_id());
    assert_eq!(original.exact_coefficients(), second.exact_coefficients());
    assert_eq!(original.sectors().len(), second.sectors().len());
    for point in [0.2, 0.43, 0.8] {
        for (a, b) in original.sectors_mut().iter_mut().zip(second.sectors_mut()) {
            let mut av = vec![0.; a.output_count()];
            let mut bv = vec![0.; b.output_count()];
            a.evaluate(&[point], &mut av).unwrap();
            b.evaluate(&[point], &mut bv).unwrap();
            assert_eq!(av, bv);
        }
    }
    assert!(
        resume(
            root.path(),
            &receipt,
            "bad",
            "foreign-source",
            &receipt.publication.prepared_identity,
            16 * 1024 * 1024,
            |_| ControlFlow::Continue(())
        )
        .is_err()
    );
    assert!(
        ThresholdPublicationPlan::from_trusted_preparer(
            &root.path().join(&receipt.work_directory),
            receipt.publication.clone(),
            &receipt.publication.source_identity,
            "foreign-prepared",
            16 * 1024 * 1024
        )
        .is_err()
    );
}
#[test]
fn cancellation_retains_raw_evidence_before_verification() {
    let root = tempfile::tempdir().unwrap();
    let (input, options) = analytic();
    let mut receipt = None;
    let error = prepare(input, options, root.path(), |p| {
        if p.receipts.raw.is_some() {
            receipt = p.checkpoint();
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    })
    .err()
    .unwrap();
    assert!(matches!(error, Error::Cancelled));
    let receipt = receipt.unwrap();
    let mut stages = Vec::new();
    let recovered = resume_evidence(
        root.path(),
        &receipt,
        "recovered",
        &receipt.evidence.request.source_identity,
        16 * 1024 * 1024,
        |p| {
            stages.push(p.stage);
            ControlFlow::Continue(())
        },
    )
    .unwrap();
    assert!(!stages.contains(&Stage::Solve));
    assert!(recovered.receipt.evidence.verification.is_some());
    assert!(
        compile(root.path(), &recovered.receipt)
            .threshold_metadata()
            .is_some()
    );
}
#[test]
fn unsupported_routes_refuse_before_native_solve() {
    for kind in 0..3 {
        let root = tempfile::tempdir().unwrap();
        let (input, mut options) = analytic();
        match kind {
            0 => options.generation.source_sectors = Some(vec![0]),
            1 => options.threshold.threshold_cells = Some(vec![0]),
            _ => options.generation.program_recipe = crate::kernel::ProgramRecipe::FixedV1,
        }
        let mut solve = false;
        assert!(
            prepare(input, options, root.path(), |p| {
                solve |= p.stage == Stage::Solve;
                ControlFlow::Continue(())
            })
            .is_err()
        );
        assert!(!solve);
        assert!(root.path().read_dir().unwrap().next().is_none());
    }
}
#[test]
fn native_graph_projective_publication_uses_ordinary_qmc_and_oneloop() {
    use crate::integration::{Periodization, QmcSession, QmcSettings, RuleSource};
    let (x, y, eps, t) = symbol!(
        "preparation_graph::x",
        "preparation_graph::y",
        "preparation_graph::eps",
        "preparation_graph::t"
    );
    let model = Arc::new(
        crate::Model::from_json(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/models/scalar.json"
        )))
        .unwrap(),
    );
    let kinematics = crate::Kinematics::in_dimension(&symbolica::parse!("D"))
        .unwrap()
        .with_mass_squared(
            &feynkit_graph::symbols::external_momentum().call(1),
            Atom::num(16),
        )
        .unwrap();
    let graph = crate::input::GraphIntegral::from_dot(
        model,
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/graphs/bubble.dot"
        )),
        &kinematics,
    )
    .unwrap()
    .with_scalar_values(&BTreeMap::from([(symbol!("UFO::mt"), Atom::num(3).sqrt())]))
    .unwrap();
    let gamma = symbolica::transcendental::gamma();
    let multiplier = gamma.call(Atom::one() - Atom::num(2) * Atom::var(eps))
        / (gamma.call(Atom::one() + Atom::var(eps))
            * gamma.call(Atom::one() - Atom::var(eps)).pow(2));
    let input = ParametricIntegrand::from_graph(
        &graph.with_measure_multiplier(multiplier),
        vec![x, y],
        eps,
        Atom::num(4) - Atom::num(2) * Atom::var(eps),
    )
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let mut options = PreparationOptions::new(t);
    options.generation.max_order = 0;
    let prepared = prepare(Arc::new(input), options, root.path(), |_| {
        ControlFlow::Continue(())
    })
    .unwrap();
    assert_eq!(prepared.receipt.original_dimensions, 2);
    assert_eq!(prepared.receipt.cells, 3);
    assert_eq!(prepared.receipt.endpoint_charts, 6);
    let mut kernels = compile(root.path(), &prepared.receipt);
    let problem = crate::results::KernelResultManifest::integration_problem_from_kernels(
        &kernels,
        &crate::results::ResultScope::FullIntegral,
        "threshold-public-preparer-graph-bubble",
    )
    .unwrap();
    assert_eq!(kernels.orders(), [-1, -1, 0, 0]);
    let mut session = QmcSession::democratic(
        problem,
        QmcSettings {
            points: 16384,
            shifts: 16,
            seed: 202610106104,
            package_points: 1024,
            periodization: Periodization::Korobov3,
            rule: RuleSource::Kuo,
        },
    )
    .unwrap();
    while let Some(task) = session.next_work().unwrap() {
        let id = task.sector_id();
        let result = session
            .worker_context(id)
            .unwrap()
            .evaluate(task, |p, o| {
                kernels.sectors_mut()[id as usize].evaluate(p, o)
            })
            .unwrap();
        session.submit(result).unwrap();
    }
    let estimate = session.estimate().unwrap();
    let mut reference = [Complex::new(0., 0.); 3];
    oneloop::evaluate_with_backend(
        oneloop::ScalarIntegral::B0,
        &[16., 3., 3., 1.].map(|v| Complex::new(v, 0.)),
        &mut reference,
        oneloop::EvaluationBackend::Expression,
    )
    .unwrap();
    let expected = [
        reference[1].re,
        reference[1].im,
        reference[0].re,
        reference[0].im,
    ];
    for (i, value) in expected.iter().enumerate() {
        assert!(
            (estimate.mean[i] - value).abs()
                <= 6. * estimate.covariance_of_mean[i * 4 + i].sqrt() + 2e-10,
            "mean {:?}, expected {:?}",
            estimate.mean,
            expected
        );
    }
}

#[test]
fn preparer_child() {
    let Ok(path) = std::env::var("FSD_THRESHOLD_PREPARER_DIRECTORY") else {
        return;
    };
    let (input, options) = analytic();
    let prepared = prepare(input, options, Path::new(&path), |_| {
        ControlFlow::Continue(())
    })
    .unwrap();
    std::fs::write(
        Path::new(&path).join("receipt.json"),
        serde_json::to_vec(&prepared.receipt).unwrap(),
    )
    .unwrap();
}
#[test]
fn fresh_preparer_child_returns_only_detached_native_work() {
    let root = tempfile::tempdir().unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "threshold::generation::tests::preparer_child",
            "--test-threads=1",
        ])
        .env("FSD_THRESHOLD_PREPARER_DIRECTORY", root.path())
        .status()
        .unwrap();
    assert!(status.success());
    let receipt: PreparationReceipt =
        serde_json::from_slice(&std::fs::read(root.path().join("receipt.json")).unwrap()).unwrap();
    let kernels = compile(root.path(), &receipt);
    assert!(kernels.threshold_metadata().is_some());
    assert!(
        ThresholdPublicationPlan::from_trusted_preparer(
            &root.path().join(&receipt.work_directory),
            receipt.publication.clone(),
            &receipt.publication.source_identity,
            &receipt.publication.prepared_identity,
            0
        )
        .is_err()
    );
}
