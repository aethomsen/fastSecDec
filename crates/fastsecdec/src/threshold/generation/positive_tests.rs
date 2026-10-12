use super::*;
use crate::parametric::{
    FactorRole, FactorSemantics, ParametricDomain, ParametricTerm, PolynomialFactor,
};
use std::{fs, time::Duration};
use symbolica::{atom::Atom, parse, symbol};
const CAP: u64 = 64 * 1024 * 1024;
fn options() -> PreparationOptions {
    let mut o = PreparationOptions::new(symbol!("positive_generation::t"));
    o.enable_automatic_endpoint_routes();
    o.threshold.solver.homogeneous_anchor = Some("x2".into());
    o.threshold.solver.ordering = "given".into();
    o.threshold.solver.order = vec!["x2".into(), "x0".into(), "x1".into()];
    o.threshold.gcad_limits.workers = 1;
    o.threshold.gcad_limits.wall_time_secs = 30.;
    o.threshold.gcad_limits.memory_mib = 1024;
    o.generation.max_order = 0;
    o.generation.subtraction = crate::generation::SubtractionStrategy::Taylor;
    o.compilation.backend = crate::kernel::EvaluatorBackend::Eager;
    let s = o.regular_family.as_mut().unwrap();
    s.discovery.max_dyadic_level = 4;
    s.discovery.wall_time = Duration::from_secs(120);
    s.limits.geometry.wall_time = Duration::from_secs(120);
    s.limits.geometry.max_terms = 100_000;
    s.limits.geometry.certificates.bernstein_coefficients = 262144;
    s.limits.geometry.certificates.work_limit = 1_000_000_000;
    s.limits.density.source_nodes = 32768;
    o
}
fn exact() -> Arc<ParametricIntegrand> {
    let (x, y, z, e) = symbol!(
        "positive_generation::x",
        "positive_generation::y",
        "positive_generation::z",
        "positive_generation::eps"
    );
    Arc::new(
        ParametricIntegrand::new(
            vec![x, y, z],
            e,
            ParametricDomain::ProjectiveSimplex,
            vec![ParametricTerm::new(
                Atom::one(),
                vec![Atom::zero(); 3],
                vec![
                    PolynomialFactor::new(
                        Atom::var(x) + Atom::var(y) + Atom::var(z),
                        Atom::num(-3),
                        FactorRole::Singularity,
                    )
                    .with_semantics(FactorSemantics::Positive),
                ],
            )],
        )
        .unwrap(),
    )
}
fn graph_point(measure: i64) -> Arc<represented::graph::GraphPoint> {
    use crate::{FeynmanDiagram, Kinematics, Model};
    let model = Arc::new(
        Model::from_json(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/models/scalar.json"
        )))
        .unwrap(),
    );
    let diagram = Arc::new(
        FeynmanDiagram::from_dot(
            model,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../examples/no_deformation/benchmarks/overview/sunrise.dot"
            )),
        )
        .unwrap(),
    );
    let p = feynkit_graph::symbols::external_momentum().call(0);
    let e = symbol!("positive_generation::graph_eps");
    let scalar_values = diagram
        .model()
        .scalar_bindings(None, &BTreeMap::from([(symbol!("UFO::mt"), Atom::one())]))
        .unwrap();
    Arc::new(represented::graph::GraphPoint {
        diagram,
        kinematics: Arc::new(
            Kinematics::in_dimension(&parse!("feynkit_graph::D"))
                .unwrap()
                .with_mass_squared(&p, Atom::num(16))
                .unwrap(),
        ),
        scalar_values,
        auxiliary_momenta: vec![],
        powers: BTreeMap::new(),
        measure_multiplier: Atom::num(measure),
        coordinates: vec![
            symbol!("positive_generation::gx"),
            symbol!("positive_generation::gy"),
            symbol!("positive_generation::gz"),
        ],
        regulator: e,
        dimension: Atom::num(4) - Atom::num(2) * Atom::var(e),
    })
}
fn check_records(root: &Path, c: &PreparationCheckpoint) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    [
        &c.configuration,
        &c.evidence.request.record,
        &c.evidence.record,
    ]
    .iter()
    .map(|r| {
        let p = r.resolve(root).unwrap();
        let b = fs::read(&p).unwrap();
        (p, b)
    })
    .collect()
}
#[test]
fn positive_exact_fresh_completed_recovery_and_archive_restoration() {
    let dir = tempfile::tempdir().unwrap();
    let receipt = prepare(
        exact(),
        options(),
        dir.path(),
        |_| ControlFlow::Continue(()),
    )
    .unwrap()
    .receipt;
    assert_eq!(receipt.cells, 1);
    assert_eq!(receipt.original_dimensions, 3);
    assert_eq!(receipt.integration_dimensions, 2);
    let (mut saved, route) = configuration::read(dir.path(), &receipt.configuration, CAP).unwrap();
    assert_eq!(route, regular::Route::PositiveProjective);
    assert!(saved.threshold.solver.homogeneous_anchor.is_none());
    let staged = gcad::staging::StagedPositiveRequest::read(
        dir.path(),
        receipt.evidence.request.as_ref().unwrap(),
        CAP,
    )
    .unwrap();
    assert_eq!(
        staged.request().problem().solver.order,
        vec!["x2", "x0", "x1"]
    );
    let mut stages = vec![];
    let replay = resume(
        dir.path(),
        &receipt,
        "replay",
        &receipt.publication.source_identity,
        &receipt.publication.prepared_identity,
        CAP,
        |p| {
            stages.push(p.stage);
            ControlFlow::Continue(())
        },
    )
    .unwrap()
    .receipt;
    assert!(!stages.contains(&Stage::Solve));
    assert!(stages.contains(&Stage::Verify));
    assert_eq!(
        replay.publication.prepared_identity,
        receipt.publication.prepared_identity
    );
    let first = tests::compile(dir.path(), &receipt);
    let second = tests::compile(dir.path(), &replay);
    assert_eq!(first.content_id(), second.content_id());
    // An endpoint route record cannot relabel the authentic staged owner.
    saved.threshold.solver.homogeneous_anchor = None;
    let wrong =
        configuration::write(dir.path(), &saved, regular::Route::ResolvedProjective).unwrap();
    let mut checkpoint = receipt.checkpoint().unwrap();
    checkpoint.configuration = wrong;
    assert!(
        resume_evidence(
            dir.path(),
            &checkpoint,
            "wrong-route",
            &receipt.publication.source_identity,
            CAP,
            |_| ControlFlow::Continue(())
        )
        .is_err()
    );
    assert!(
        resume(
            dir.path(),
            &receipt,
            "wrong-id",
            &receipt.publication.source_identity,
            "wrong",
            CAP,
            |_| ControlFlow::Continue(())
        )
        .is_err()
    );
}
#[test]
fn positive_resource_retry_preserves_original_and_resumes_new_attempt() {
    let base = tempfile::tempdir().unwrap();
    let old = base.path().join("old");
    fs::create_dir(&old).unwrap();
    let mut checkpoint = None;
    let result = prepare(exact(), options(), &old, |p| {
        if let Some(c) = p.checkpoint() {
            checkpoint = Some(c);
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    });
    assert!(matches!(result, Err(Error::Cancelled)));
    let checkpoint = checkpoint.unwrap();
    let before = check_records(&old, &checkpoint);
    let overlay = EndpointPreparationResources {
        max_charts: Some(20000),
        geometry: GeometryResources {
            wall_time_secs: Some(240.),
            ..Default::default()
        },
        ..Default::default()
    };
    let new = base.path().join("retry");
    let mut retry_checkpoint = None;
    let result = retry_endpoint_evidence(
        &old,
        &checkpoint,
        &checkpoint.evidence.request.source_identity,
        &overlay,
        &new,
        CAP,
        |p| {
            if let Some(c) = p.checkpoint.as_ref() {
                retry_checkpoint = Some(c.clone());
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        },
    );
    assert!(matches!(result, Err(Error::Cancelled)));
    let retry_checkpoint = retry_checkpoint.unwrap();
    let mut stages = vec![];
    let done = resume_endpoint_retry(
        &new,
        &retry_checkpoint,
        "resumed",
        &checkpoint.evidence.request.source_identity,
        None,
        CAP,
        |p| {
            stages.push(p.preparation.stage);
            ControlFlow::Continue(())
        },
    )
    .unwrap();
    assert!(!stages.contains(&Stage::Solve));
    assert_eq!(done.prepared.receipt.cells, 1);
    for (p, b) in before {
        assert_eq!(fs::read(p).unwrap(), b);
    }
    let (saved, route) =
        configuration::read(&new, &done.checkpoint.preparation.configuration, CAP).unwrap();
    assert_eq!(route, regular::Route::PositiveProjective);
    assert_eq!(saved.regularization.max_charts, 20000);
    let again = base.path().join("completed-retry");
    let completed = retry_endpoint_evidence(
        &new,
        &done.prepared.receipt.checkpoint().unwrap(),
        &checkpoint.evidence.request.source_identity,
        &overlay,
        &again,
        CAP,
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    assert_eq!(completed.prepared.receipt.cells, 1);
}
#[test]
fn actual_graph_interruption_authentication_and_full_algebraic_publication() {
    let dir = tempfile::tempdir().unwrap();
    let original = graph_point(1);
    let mut o = options();
    o.represented = Some((
        represented::NumericalMeaning::RepresentedValues,
        Default::default(),
    ));
    let fresh_options = o.clone();
    let mut checkpoint = None;
    let initial = prepare_graph(original.clone(), o, dir.path(), |p| {
        if let Some(c) = p.checkpoint() {
            checkpoint = Some(c);
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    });
    assert!(
        matches!(initial, Err(Error::Cancelled)),
        "{:?}",
        initial.err()
    );
    let c = checkpoint.unwrap();
    assert_eq!(c.evidence.cells, 7);
    let source = &c.evidence.request.source_identity;
    assert!(
        resume_evidence(dir.path(), &c, "missing-graph", source, CAP, |_| {
            ControlFlow::Continue(())
        })
        .is_err()
    );
    assert!(
        resume_graph_evidence(
            graph_point(2),
            Default::default(),
            dir.path(),
            &c,
            "wrong-graph",
            source,
            CAP,
            |_| ControlFlow::Continue(())
        )
        .is_err()
    );
    let cap = represented::Limits {
        nodes: 1,
        ..Default::default()
    };
    assert!(
        resume_graph_evidence(
            original.clone(),
            cap,
            dir.path(),
            &c,
            "small-cap",
            source,
            CAP,
            |_| ControlFlow::Continue(())
        )
        .is_err()
    );
    let mut stages = vec![];
    let receipt = resume_graph_evidence(
        original.clone(),
        Default::default(),
        dir.path(),
        &c,
        "actual",
        source,
        CAP,
        |p| {
            stages.push(p.stage);
            ControlFlow::Continue(())
        },
    )
    .unwrap()
    .receipt;
    assert!(!stages.contains(&Stage::Solve));
    assert_eq!(receipt.cells, 7);
    assert_eq!(receipt.endpoint_charts, 62);
    let fresh_directory = tempfile::tempdir().unwrap();
    let fresh = prepare_graph(original, fresh_options, fresh_directory.path(), |_| {
        ControlFlow::Continue(())
    })
    .unwrap()
    .receipt;
    assert_eq!(fresh.endpoint_charts, receipt.endpoint_charts);
    assert_eq!(
        fresh.publication.prepared_identity,
        receipt.publication.prepared_identity
    );
    let kernels = tests::compile(dir.path(), &receipt);
    assert_eq!(kernels.sectors().len(), 62);
    assert_eq!(
        kernels
            .orders()
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>(),
        [-2, -1, 0].into()
    );
}
#[test]
fn positive_selection_wrong_order_policy_and_cancellation_are_typed() {
    for mutate in 0..4 {
        let dir = tempfile::tempdir().unwrap();
        let mut o = options();
        match mutate {
            0 => o.threshold.solver.order.clear(),
            1 => o.threshold.solver.homogeneous_anchor = Some("x9".into()),
            2 => o.threshold.solver.homogeneous_block = vec!["x0".into()],
            _ => {
                o.represented = Some((
                    represented::NumericalMeaning::RepresentedValues,
                    Default::default(),
                ))
            }
        }
        assert!(prepare(exact(), o, dir.path(), |_| ControlFlow::Continue(())).is_err());
    }
    assert!(family::admission(&options()).is_err());
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(
        prepare(exact(), options(), dir.path(), |_| ControlFlow::Break(())),
        Err(Error::Cancelled)
    ));
}
