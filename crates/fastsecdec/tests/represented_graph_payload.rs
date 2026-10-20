#![cfg(feature = "threshold-decomposition")]
use fastsecdec::{
    Atom, AtomCore, FeynmanDiagram, Kinematics, Model,
    integration::{Periodization, QmcSession, QmcSettings, RuleSource},
    kernel::{KernelLoadOptions, KernelSet, ThresholdPublicationPlan},
    results::{KernelResultManifest, ResultScope},
    threshold::{
        gcad::staging::StagedRequest,
        generation::{self, PreparationOptions, PreparationReceipt},
        represented::{
            self, Limits, NumericalMeaning,
            graph::{GraphLocation, GraphPayloadPolicy, GraphPoint},
        },
    },
};
use idenso::tensor::SymbolicTensor;
use spenso::structure::representation::{Minkowski, RepName};
use std::{collections::BTreeMap, io::Cursor, ops::ControlFlow, path::Path, sync::Arc};
use symbolica::{domains::float::Float, parse, symbol};
const CAP: u64 = 64 * 1024 * 1024;
fn point(bits: u32, tensor: bool, payload: bool) -> Arc<GraphPoint> {
    let model = Arc::new(
        Model::from_json(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/models/scalar.json"
        )))
        .unwrap(),
    );
    let mut diagram = FeynmanDiagram::from_dot(
        model,
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/graphs/bubble.dot"
        )),
    )
    .unwrap();
    let (x, y, e) = symbol!("graph_payload::x", "graph_payload::y", "graph_payload::eps");
    let p = feynkit_graph::symbols::external_momentum().call(1);
    if payload {
        let vertex = diagram.vertices().next().unwrap().0;
        let edge = diagram
            .edges()
            .find(|(_, _, e)| e.external.is_none())
            .unwrap()
            .0;
        let [a, b, c, d, f] = ["0.1", "0.2", "0.3", "0.4", "0.7"]
            .map(|s| Atom::num(Float::parse(s, Some(bits)).unwrap()));
        let vector = Minkowski {}.new_rep(4).vector(p.as_view(), [Atom::num(1)]);
        let numerator = if tensor { a * &vector } else { a };
        let projector = if tensor { c * &vector } else { c };
        diagram = diagram
            .with_numerator_fragments(
                [(vertex, SymbolicTensor::infer(numerator).unwrap())],
                [(edge, SymbolicTensor::infer(b).unwrap())],
            )
            .unwrap()
            .with_projector(SymbolicTensor::infer(projector).unwrap())
            .with_numerator_prefactor(SymbolicTensor::infer(d).unwrap())
            .with_overall_factor(f);
    }
    let gamma = symbolica::transcendental::gamma();
    let measure = gamma.call(Atom::one() - Atom::num(2) * Atom::var(e))
        / (gamma.call(Atom::one() + Atom::var(e)) * gamma.call(Atom::one() - Atom::var(e)).pow(2));
    Arc::new(GraphPoint {
        diagram: Arc::new(diagram),
        kinematics: Arc::new(
            Kinematics::in_dimension(&parse!("D"))
                .unwrap()
                .with_mass_squared(&p, Atom::num(12.0))
                .unwrap(),
        ),
        scalar_values: BTreeMap::from([(symbol!("UFO::mt"), Atom::num(1.5))]),
        auxiliary_momenta: vec![],
        powers: BTreeMap::new(),
        measure_multiplier: measure,
        coordinates: vec![x, y],
        regulator: e,
        dimension: Atom::num(4) - Atom::num(2) * Atom::var(e),
    })
}
fn options(payload: bool) -> PreparationOptions {
    let mut o = PreparationOptions::new(symbol!("graph_payload::t"));
    o.represented = Some((NumericalMeaning::RepresentedValues, Limits::default()));
    if payload {
        o.graph_payloads = Some(GraphPayloadPolicy::PrimitiveFragmentsV1)
    };
    o
}
fn prepared(source: Arc<GraphPoint>, root: &Path) -> PreparationReceipt {
    generation::prepare_graph(source, options(true), root, |_| ControlFlow::Continue(()))
        .unwrap()
        .receipt
}
fn kernels(root: &Path, r: &PreparationReceipt) -> KernelSet {
    let path = root.join(&r.work_directory);
    let plan = ThresholdPublicationPlan::from_trusted_preparer(
        &path,
        r.publication.clone(),
        &r.publication.source_identity,
        &r.publication.prepared_identity,
        CAP,
    )
    .unwrap();
    let mut writer = plan.archive_writer(Cursor::new(Vec::new())).unwrap();
    for i in (0..plan.job_count()).rev() {
        let work = plan.work(i).unwrap();
        let mut bytes = vec![];
        let receipt = work.compile_record(&path, CAP, &mut bytes).unwrap();
        plan.append_work_record(&mut writer, &mut bytes.as_slice(), receipt)
            .unwrap();
    }
    let (bytes, _) = writer.finish().unwrap();
    KernelSet::from_bytes_with_options(&bytes.into_inner(), KernelLoadOptions { validate: true })
        .unwrap()
}
fn converted(
    source: Arc<GraphPoint>,
    root: &Path,
) -> represented::graph::ExactRepresentedGraphInput {
    let mut receipt = None;
    let error = generation::prepare_graph(source.clone(), options(true), root, |p| {
        if p.stage == generation::Stage::Solve {
            receipt = p.receipts.request.clone();
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    })
    .err()
    .unwrap();
    assert!(matches!(error, generation::Error::Cancelled));
    let staged = StagedRequest::read_with_original_graph(
        root,
        &receipt.unwrap(),
        CAP,
        source,
        Limits::default(),
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    staged
        .request()
        .preparametric_graph_input()
        .unwrap()
        .clone()
}
#[test]
fn primitive_policy_retains_original_rounding_precision_and_native_tensor_roles() {
    for bits in [53, 192] {
        for tensor in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let source = point(bits, tensor, true);
            let original = source.diagram.to_json().unwrap();
            let owner = converted(source.clone(), dir.path());
            let exact = owner.exact_diagram();
            assert_eq!(
                owner.payload_policy(),
                Some(GraphPayloadPolicy::PrimitiveFragmentsV1)
            );
            assert_eq!(source.diagram.to_json().unwrap(), original);
            assert_eq!(exact.id(), source.diagram.id());
            assert_eq!(
                exact.loop_momentum_basis(),
                source.diagram.loop_momentum_basis()
            );
            assert!(Arc::ptr_eq(&exact.model_arc(), &source.diagram.model_arc()));
            assert_eq!(exact.cuts(), source.diagram.cuts());
            assert_eq!(exact.symmetry_factor(), source.diagram.symmetry_factor());
            exact.validate().unwrap();
            assert!(owner.conversions().iter().any(|r| matches!(
                r.location,
                GraphLocation::OriginalAggregate {}
            ) && r.precision_bits[0] == bits));
            assert_eq!(
                owner
                    .conversions()
                    .iter()
                    .filter(|r| matches!(r.location, GraphLocation::Payload { .. }))
                    .count(),
                5
            );
            let aggregate = owner
                .conversions()
                .iter()
                .find(|r| matches!(r.location, GraphLocation::OriginalAggregate {}))
                .unwrap();
            let primitive=owner.conversions().iter().filter(|r|matches!(&r.location,GraphLocation::Payload{role} if role.starts_with("vertex_")||role.starts_with("edge_"))).map(|r|r.exact.clone()).product::<Atom>();
            assert!(!(&aggregate.exact - &primitive).expand().is_zero());
            // The known native external-vector contraction is scalar s, never a new
            // tensor implementation. Input validation runs actual Idenso contraction.
            let plain = converted(
                point(bits, false, true),
                tempfile::tempdir().unwrap().path(),
            );
            if tensor {
                assert_ne!(owner.exact(), plain.exact());
            }
        }
    }
}
#[test]
fn policy_refuses_structural_float_and_unprojected_tensor_before_solve() {
    let base = point(53, false, true);
    let momentum = feynkit_graph::symbols::external_momentum().call(Atom::num(1.0));
    let index = Minkowski {}.new_rep(4).vector(
        feynkit_graph::symbols::external_momentum()
            .call(1)
            .as_view(),
        [Atom::num(1.0)],
    );
    let label = Minkowski {}
        .new_rep(4)
        .vector(momentum.as_view(), [Atom::num(1)]);
    for bad in [
        symbol!("graph_payload::f").call(Atom::num(0.1)),
        index,
        label,
    ] {
        let mut p = (*base).clone();
        p.diagram = Arc::new(
            p.diagram
                .as_ref()
                .clone()
                .with_projector(SymbolicTensor::infer(bad).unwrap()),
        );
        let before = p.diagram.to_json().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let err = generation::prepare_graph(Arc::new(p.clone()), options(true), dir.path(), |p| {
            assert_eq!(p.stage, generation::Stage::Input);
            ControlFlow::Continue(())
        })
        .err()
        .unwrap();
        assert!(err.to_string().contains("Float under a function"), "{err}");
        assert_eq!(before, p.diagram.to_json().unwrap());
        assert_eq!(dir.path().read_dir().unwrap().count(), 0);
    }
    let mut p = (*point(53, true, true)).clone();
    p.diagram = Arc::new(
        p.diagram
            .as_ref()
            .clone()
            .with_projector(SymbolicTensor::infer(Atom::one()).unwrap()),
    );
    let dir = tempfile::tempdir().unwrap();
    assert!(
        generation::prepare_graph(Arc::new(p), options(true), dir.path(), |p| {
            assert_eq!(p.stage, generation::Stage::Input);
            ControlFlow::Continue(())
        })
        .is_err()
    );
    for change in 0..3 {
        let dir = tempfile::tempdir().unwrap();
        let mut o = options(true);
        match change {
            0 => o.represented = None,
            1 => o.represented.as_mut().unwrap().1.converted_literals = 0,
            _ => {}
        };
        assert!(
            generation::prepare_graph(base.clone(), o, dir.path(), |_| if change == 2 {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            })
            .is_err()
        );
    }
    let dir = tempfile::tempdir().unwrap();
    assert!(
        generation::prepare_graph(base, options(false), dir.path(), |_| ControlFlow::Continue(
            ()
        ))
        .is_err()
    );
}
#[test]
fn legacy_policy_omission_keeps_native_identity_and_new_policy_is_explicit() {
    let source = point(53, false, false);
    let old = represented::graph::ExactRepresentedGraphInput::prepare(
        source.clone(),
        NumericalMeaning::RepresentedValues,
        Limits::default(),
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    assert!(old.payload_policy().is_none());
    assert!(Arc::ptr_eq(old.exact_diagram(), &source.diagram));
    let dir = tempfile::tempdir().unwrap();
    let explicit = converted(source, dir.path());
    assert_eq!(old.exact(), explicit.exact());
    assert_ne!(old.source_identity(), explicit.source_identity());
}
fn sample(k: KernelSet) -> (Vec<f64>, Vec<f64>) {
    let problem = KernelResultManifest::integration_problem_from_kernels(
        &k,
        &ResultScope::FullIntegral,
        k.content_id(),
    )
    .unwrap();
    assert_eq!(problem.orders, [-1, -1, 0, 0]);
    let mut contexts = (0..k.sectors().len())
        .map(|i| k.evaluation_context(i, Default::default()).unwrap())
        .collect::<Vec<_>>();
    let mut session = QmcSession::democratic(
        problem,
        QmcSettings {
            points: 1024,
            shifts: 8,
            package_points: 128,
            seed: 202610114991,
            rule: RuleSource::Kuo,
            periodization: Periodization::None,
        },
    )
    .unwrap();
    while let Some(task) = session.next_work().unwrap() {
        let id = task.sector_id();
        let r = session
            .worker_context(id)
            .unwrap()
            .evaluate_weighted_batch(task, 64, |p, w, o| {
                contexts[id as usize].evaluate_weighted_batch(p, w, o)?;
                Ok::<_, fastsecdec::kernel::BatchEvaluationError>(())
            })
            .unwrap();
        session.submit(r).unwrap();
    }
    let e = session.estimate().unwrap();
    e.validate().unwrap();
    assert!(e.production_complete);
    assert_eq!(e.covariance_of_mean.len(), 16);
    println!("{}", serde_json::to_string(&e).unwrap());
    (e.mean.clone(), e.covariance_of_mean.clone())
}
#[test]
fn full_tensor_laurent_covariance_saved_reload_and_fresh_original_replay() {
    let dir = tempfile::tempdir().unwrap();
    let original = point(53, true, true);
    let receipt = prepared(original.clone(), dir.path());
    let first = kernels(dir.path(), &receipt);
    let bytes = first.to_bytes().unwrap();
    let first_id = first.content_id().to_owned();
    std::fs::write(
        dir.path().join("receipt.json"),
        serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();
    std::fs::write(dir.path().join("kernel.bin"), &bytes).unwrap();
    drop(original);
    drop(first);
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "cold_payload_replay_child",
            "--ignored",
            "--nocapture",
        ])
        .env("FASTSECDEC_PAYLOAD_CHILD", dir.path())
        .output()
        .unwrap();
    assert!(
        child.status.success(),
        "{}{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
    let restored =
        KernelSet::from_bytes_with_options(&bytes, KernelLoadOptions { validate: true }).unwrap();
    assert_eq!(restored.content_id(), first_id);
    let (mean, cov) = sample(restored);
    let weight = 0.1 * 0.2 * 0.3 * 0.4 * 0.7 * 12.;
    let expected = [
        weight,
        0.,
        weight * (2. - (2.25f64).ln() - 0.5 * (3f64).ln()),
        weight * std::f64::consts::PI / 2.,
    ];
    for i in 0..4 {
        assert!(
            (mean[i] - expected[i]).abs() <= 6. * cov[i * 4 + i].sqrt() + 2e-10,
            "component{i}: {mean:?} {expected:?}"
        );
    }
    let baseline = tempfile::tempdir().unwrap();
    let ordinary = prepared(point(53, false, false), baseline.path());
    let (m, c) = sample(kernels(baseline.path(), &ordinary));
    for i in 0..4 {
        assert!((mean[i] - weight * m[i]).abs() < 2e-10);
        for j in 0..4 {
            let expected_cov = weight * weight * c[i * 4 + j];
            assert!((cov[i * 4 + j] - expected_cov).abs() <= 1e-6 * expected_cov.abs() + 1e-27);
        }
    }
}
#[test]
#[ignore = "invoked as fresh-process original-owner replay"]
fn cold_payload_replay_child() {
    let root = std::path::PathBuf::from(std::env::var_os("FASTSECDEC_PAYLOAD_CHILD").unwrap());
    let receipt: PreparationReceipt =
        serde_json::from_slice(&std::fs::read(root.join("receipt.json")).unwrap()).unwrap();
    let mut stages = vec![];
    let next = generation::resume_graph(
        point(53, true, true),
        Limits::default(),
        &root,
        &receipt,
        "replayed",
        &receipt.publication.source_identity,
        &receipt.publication.prepared_identity,
        CAP,
        |p| {
            stages.push(p.stage);
            ControlFlow::Continue(())
        },
    )
    .unwrap();
    assert!(!stages.contains(&generation::Stage::Solve));
    assert_eq!(
        next.receipt.publication.prepared_identity,
        receipt.publication.prepared_identity
    );
    let first = KernelSet::from_bytes(&std::fs::read(root.join("kernel.bin")).unwrap()).unwrap();
    let second = kernels(&root, &next.receipt);
    assert_eq!(first.content_id(), second.content_id());
    for p in [
        point(192, true, true),
        point(53, false, true),
        point(53, true, false),
    ] {
        assert!(
            generation::resume_graph(
                p,
                Limits::default(),
                &root,
                &receipt,
                "rejected",
                &receipt.publication.source_identity,
                &receipt.publication.prepared_identity,
                CAP,
                |_| ControlFlow::Continue(())
            )
            .is_err()
        );
    }
}
