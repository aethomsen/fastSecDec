use super::*;
use crate::parametric::{ParametricDomain, ParametricTerm, PolynomialFactor};
use symbolica::{atom::Atom, symbol};

fn source(positive: i64) -> Arc<ParametricIntegrand> {
    let (x, y, eps) = symbol!(
        "positive_request::x",
        "positive_request::y",
        "positive_request::eps"
    );
    Arc::new(
        ParametricIntegrand::new(
            vec![x, y],
            eps,
            ParametricDomain::ProjectiveSimplex,
            vec![ParametricTerm::new(
                Atom::one(),
                vec![Atom::zero(); 2],
                vec![
                    PolynomialFactor::new(
                        Atom::num(positive) * (Atom::var(x) + Atom::var(y)),
                        Atom::one(),
                        FactorRole::Singularity,
                    )
                    .with_semantics(FactorSemantics::Positive),
                    PolynomialFactor::new(
                        Atom::var(x) - Atom::var(y),
                        Atom::num(-3),
                        FactorRole::Singularity,
                    )
                    .with_semantics(FactorSemantics::Causal),
                ],
            )],
        )
        .unwrap(),
    )
}

fn limits() -> Limits {
    Limits {
        workers: 1,
        memory_mib: 1024,
        wall_time_secs: 10.0,
        ..Default::default()
    }
}

#[test]
fn original_request_issues_both_signs_and_rejects_relabelled_evidence() {
    let input = source(1);
    let request =
        PositiveProjectiveRequest::exact(input.clone(), 1, Default::default(), limits()).unwrap();
    assert!(Arc::ptr_eq(request.preparation().input(), &input));
    assert_eq!(request.problem().variables, ["x0", "x1"]);
    assert_eq!(request.problem().effective_order(), ["x1", "x0"]);
    assert_eq!(request.problem().constraints.len(), 2);
    assert_eq!(request.problem().split.len(), 1);
    let raw = Arc::new(request.solve().unwrap());
    let owner = request.verify(raw.clone()).unwrap();
    assert!(Arc::ptr_eq(owner.original_result(), &raw));
    assert!(Arc::ptr_eq(owner.preparation(), request.preparation()));
    assert_eq!(owner.cell_count(), 2);
    assert_eq!(
        owner
            .native_result()
            .cells
            .iter()
            .map(|c| c.signs[0])
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([-1, 1])
    );
    assert!(
        request
            .verify(Arc::new(owner.native_result().clone()))
            .is_err()
    );
    let mut changed = (*raw).clone();
    changed.problem.limits.wall_time_secs = 9.0;
    assert!(matches!(
        request.verify(Arc::new(changed)),
        Err(GcadError::RequestMismatch)
    ));
    let mut changed = (*raw).clone();
    changed
        .homogeneous
        .as_mut()
        .unwrap()
        .reduced
        .cells
        .remove(0);
    assert!(request.verify(Arc::new(changed)).is_err());
}

#[test]
fn source_positive_factor_is_verified_without_pruning_the_domain() {
    let request =
        PositiveProjectiveRequest::exact(source(-1), 0, Default::default(), limits()).unwrap();
    assert_eq!(request.problem().constraints, ["x0>0", "x1>0"]);
    let raw = Arc::new(request.solve().unwrap());
    assert!(matches!(
        request.verify(raw),
        Err(GcadError::Unsupported(_))
    ));
}

#[test]
fn request_rejects_implicit_workers_ordering_and_partial_sign_coverage() {
    let mut l = limits();
    l.workers = 2;
    assert!(PositiveProjectiveRequest::exact(source(1), 0, Default::default(), l).is_err());
    for solver in [
        SolverOptions {
            order: vec!["x1".into(), "x0".into()],
            ..Default::default()
        },
        SolverOptions {
            order: vec!["x0".into(), "x0".into()],
            ..Default::default()
        },
        SolverOptions {
            monotone_negative_target: true,
            ..Default::default()
        },
        SolverOptions {
            homogeneous_anchor: Some("x1".into()),
            ..Default::default()
        },
        SolverOptions {
            homogeneous_block: vec!["x0".into()],
            ..Default::default()
        },
        SolverOptions {
            ordering: "search".into(),
            ..Default::default()
        },
    ] {
        assert!(PositiveProjectiveRequest::exact(source(1), 0, solver, limits()).is_err());
    }
}

#[test]
fn complete_massive_sunrise_request_matches_native_homogeneous_source() {
    let (x, y, z, eps) = symbol!(
        "positive_request::sx",
        "positive_request::sy",
        "positive_request::sz",
        "positive_request::seps"
    );
    let (x, y, z, e) = (Atom::var(x), Atom::var(y), Atom::var(z), Atom::var(eps));
    let u = &x * &y + &x * &z + &y * &z;
    let f = (&x + &y + &z) * &u - Atom::num(16) * &x * &y * &z;
    let input = Arc::new(
        ParametricIntegrand::new(
            vec![
                symbol!("positive_request::sx"),
                symbol!("positive_request::sy"),
                symbol!("positive_request::sz"),
            ],
            eps,
            ParametricDomain::ProjectiveSimplex,
            vec![ParametricTerm::new(
                Atom::one(),
                vec![Atom::zero(); 3],
                vec![
                    PolynomialFactor::new(
                        u,
                        Atom::num(-3) + Atom::num(3) * &e,
                        FactorRole::Singularity,
                    )
                    .with_semantics(FactorSemantics::Positive),
                    PolynomialFactor::new(
                        f,
                        Atom::one() - Atom::num(2) * &e,
                        FactorRole::Singularity,
                    )
                    .with_semantics(FactorSemantics::Causal),
                ],
            )],
        )
        .unwrap(),
    );
    let request = PositiveProjectiveRequest::exact(input, 2, Default::default(), limits()).unwrap();
    let owner = request.verify(Arc::new(request.solve().unwrap())).unwrap();
    assert_eq!(owner.cell_count(), 7);
    assert_eq!(owner.original_result().order, ["x2", "x0", "x1"]);
    assert_eq!(owner.native_result().order, ["x0", "x1"]);
    assert!(owner.preparation().measure_jacobian().is_one());
}

#[test]
fn original_hepkit_graph_and_represented_point_survive_request_verification() {
    use crate::threshold::represented::{
        NumericalMeaning,
        graph::{ExactRepresentedGraphInput, GraphPoint},
    };
    use crate::{FeynmanDiagram, Kinematics, Model};
    use std::{collections::BTreeMap, ops::ControlFlow};
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
                "/../../examples/graphs/bubble.dot"
            )),
        )
        .unwrap(),
    );
    let (x, y, eps) = symbol!(
        "positive_request::gx",
        "positive_request::gy",
        "positive_request::geps"
    );
    let p = feynkit_graph::symbols::external_momentum().call(1);
    let kinematics = Kinematics::in_dimension(&symbolica::parse!("D"))
        .unwrap()
        .with_mass_squared(&p, Atom::num(12.0))
        .unwrap();
    let point = Arc::new(GraphPoint {
        diagram,
        kinematics: Arc::new(kinematics),
        scalar_values: BTreeMap::from([(symbol!("UFO::mt"), Atom::num(1.5))]),
        auxiliary_momenta: vec![],
        powers: BTreeMap::new(),
        measure_multiplier: Atom::num(0.5),
        coordinates: vec![x, y],
        regulator: eps,
        dimension: Atom::num(4) - Atom::num(2) * Atom::var(eps),
    });
    let graph = Arc::new(
        ExactRepresentedGraphInput::prepare(
            point.clone(),
            NumericalMeaning::RepresentedValues,
            Default::default(),
            |_| ControlFlow::Continue(()),
        )
        .unwrap(),
    );
    let request =
        PositiveProjectiveRequest::graph(graph.clone(), 1, Default::default(), limits()).unwrap();
    assert_eq!(request.source_identity().unwrap(), graph.source_identity());
    assert!(Arc::ptr_eq(request.preparation().input(), graph.exact()));
    let owner = request.verify(Arc::new(request.solve().unwrap())).unwrap();
    assert_eq!(owner.cell_count(), 3);
    let SourceProvenance::PreparametricGraph(retained) = owner.provenance() else {
        panic!("original graph provenance lost");
    };
    assert!(Arc::ptr_eq(retained, &graph));
    assert!(Arc::ptr_eq(retained.original(), &point));
    assert!(Arc::ptr_eq(owner.preparation().input(), graph.exact()));
}
