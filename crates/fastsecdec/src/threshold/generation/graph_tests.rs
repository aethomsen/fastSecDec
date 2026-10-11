use super::*;
use represented::graph::GraphPoint;

fn graph(float: bool) -> Arc<GraphPoint> {
    let model = Arc::new(
        crate::Model::from_json(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/models/scalar.json"
        )))
        .unwrap(),
    );
    let diagram = Arc::new(
        crate::FeynmanDiagram::from_dot(
            model,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../examples/graphs/bubble.dot"
            )),
        )
        .unwrap(),
    );
    let (x, y, eps) = symbol!("graph_adapter::x", "graph_adapter::y", "graph_adapter::eps");
    let p = feynkit_graph::symbols::external_momentum().call(1);
    let kinematics = crate::Kinematics::in_dimension(&symbolica::parse!("D"))
        .unwrap()
        .with_mass_squared(
            &p,
            if float {
                Atom::num(12.0)
            } else {
                Atom::num(12)
            },
        )
        .unwrap();
    Arc::new(GraphPoint {
        diagram,
        kinematics: Arc::new(kinematics),
        scalar_values: BTreeMap::from([(
            symbol!("UFO::mt"),
            if float {
                Atom::num(1.5)
            } else {
                Atom::num((3, 2))
            },
        )]),
        auxiliary_momenta: vec![],
        powers: BTreeMap::new(),
        measure_multiplier: if float {
            Atom::num(0.5)
        } else {
            Atom::num((1, 2))
        },
        coordinates: vec![x, y],
        regulator: eps,
        dimension: Atom::num(4) - Atom::num(2) * Atom::var(eps),
    })
}
fn options() -> PreparationOptions {
    let mut options = PreparationOptions::new(symbol!("graph_adapter::t"));
    options.represented = Some((
        represented::NumericalMeaning::RepresentedValues,
        Default::default(),
    ));
    options
}
const CAP: u64 = 32 * 1024 * 1024;

#[test]
fn structural_edges_match_native_family_order_and_omit_dummy_payloads() {
    let point = graph(false);
    let slots = crate::input::GraphIntegral::propagator_edge_ids(&point.diagram);
    assert_eq!(slots, vec![crate::EdgeId(2), crate::EdgeId(3)]);
    let native = crate::input::GraphIntegral::new_with_scalar_values(
        point.diagram.clone(),
        &point.kinematics,
        &point.scalar_values,
    )
    .unwrap();
    assert_eq!(slots, native.propagator_edges());
    assert_eq!(slots.len(), native.family().denominators().len());
    let metadata = point
        .diagram
        .map_data(
            |_, v| v.clone(),
            |id, _, e| {
                let mut e = e.clone();
                if id == crate::EdgeId(2) {
                    e.is_dummy = true;
                }
                e
            },
        )
        .unwrap();
    // This is the cheap structural view, not a new validated family.
    assert_eq!(
        crate::input::GraphIntegral::propagator_edge_ids(&metadata),
        vec![crate::EdgeId(3)]
    );
}

#[test]
fn represented_graph_publication_and_original_owner_replay_preserve_vectors_and_identity() {
    let root = tempfile::tempdir().unwrap();
    let original = graph(true);
    let prepared = prepare_graph(original.clone(), options(), root.path(), |_| {
        ControlFlow::Continue(())
    })
    .unwrap();
    let receipt = &prepared.receipt;
    let mut first = compile(root.path(), receipt);
    let mut stages = vec![];
    let restored = resume_graph(
        original,
        Default::default(),
        root.path(),
        receipt,
        "replay",
        &receipt.publication.source_identity,
        &receipt.publication.prepared_identity,
        CAP,
        |p| {
            stages.push(p.stage);
            ControlFlow::Continue(())
        },
    )
    .unwrap();
    assert!(!stages.contains(&Stage::Solve));
    assert!(stages.contains(&Stage::Verify));
    assert_eq!(
        receipt.publication.prepared_identity,
        restored.receipt.publication.prepared_identity
    );
    let mut second = compile(root.path(), &restored.receipt);
    assert_eq!(first.content_id(), second.content_id());
    assert_eq!(first.exact_coefficients(), second.exact_coefficients());
    assert_eq!(first.sectors().len(), second.sectors().len());
    for point in [0.125, 0.43, 0.875] {
        for (a, b) in first.sectors_mut().iter_mut().zip(second.sectors_mut()) {
            let mut av = vec![0.; a.output_count()];
            let mut bv = vec![0.; b.output_count()];
            a.evaluate(&[point], &mut av).unwrap();
            b.evaluate(&[point], &mut bv).unwrap();
            assert_eq!(av, bv);
        }
    }
    assert!(matches!(
        resume(
            root.path(),
            receipt,
            "no-original",
            &receipt.publication.source_identity,
            &receipt.publication.prepared_identity,
            CAP,
            |_| ControlFlow::Continue(())
        ),
        Err(Error::Evidence(gcad::staging::Error::OriginalGraphRequired))
    ));
    assert!(
        resume_graph(
            graph(false),
            Default::default(),
            root.path(),
            receipt,
            "different-original",
            &receipt.publication.source_identity,
            &receipt.publication.prepared_identity,
            CAP,
            |_| ControlFlow::Continue(())
        )
        .is_err()
    );
}

#[test]
fn graph_raw_checkpoint_replays_without_solve_and_refuses_policy_configuration_change() {
    let root = tempfile::tempdir().unwrap();
    let mut checkpoint = None;
    let error = prepare_graph(graph(true), options(), root.path(), |p| {
        if let Some(c) = p.checkpoint() {
            checkpoint = Some(c);
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    })
    .err()
    .unwrap();
    assert!(matches!(error, Error::Cancelled));
    let checkpoint = checkpoint.unwrap();
    let mut stages = vec![];
    let recovered = resume_graph_evidence(
        graph(true),
        Default::default(),
        root.path(),
        &checkpoint,
        "recovered",
        &checkpoint.evidence.request.source_identity,
        CAP,
        |p| {
            stages.push(p.stage);
            ControlFlow::Continue(())
        },
    )
    .unwrap();
    assert!(!stages.contains(&Stage::Solve));
    assert!(recovered.receipt.evidence.verification.is_some());
    let mut changed = options();
    changed.represented.as_mut().unwrap().1.converted_literals += 1;
    let foreign = PreparationCheckpoint {
        configuration: configuration::write(root.path(), &changed, regular::Route::LegacyFiber)
            .unwrap(),
        ..checkpoint.clone()
    };
    assert!(matches!(
        resume_graph_evidence(
            graph(true),
            Default::default(),
            root.path(),
            &foreign,
            "foreign",
            &foreign.evidence.request.source_identity,
            CAP,
            |_| ControlFlow::Continue(())
        ),
        Err(Error::Association("graph conversion policy/configuration"))
    ));
    let mut payload_options = options();
    payload_options.graph_payloads =
        Some(represented::graph::GraphPayloadPolicy::PrimitiveFragmentsV1);
    let payload_foreign = PreparationCheckpoint {
        configuration: configuration::write(
            root.path(),
            &payload_options,
            regular::Route::LegacyFiber,
        )
        .unwrap(),
        ..checkpoint.clone()
    };
    assert!(matches!(
        resume_graph_evidence(
            graph(true),
            Default::default(),
            root.path(),
            &payload_foreign,
            "foreign-payload",
            &payload_foreign.evidence.request.source_identity,
            CAP,
            |_| ControlFlow::Continue(())
        ),
        Err(Error::Association("graph conversion policy/configuration"))
    ));
    let cap = represented::Limits {
        converted_literals: 0,
        ..Default::default()
    };
    assert!(
        resume_graph_evidence(
            graph(true),
            cap,
            root.path(),
            &checkpoint,
            "low-cap",
            &checkpoint.evidence.request.source_identity,
            CAP,
            |_| ControlFlow::Continue(())
        )
        .is_err()
    );
}

#[test]
fn graph_conversion_cancellation_and_unsupported_options_do_not_solve() {
    for phase in 0..2 {
        let root = tempfile::tempdir().unwrap();
        let error = prepare_graph(graph(true), options(), root.path(), |p| {
            assert_eq!(p.stage, Stage::Input);
            if phase == 0 || p.completed > 0 {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })
        .err()
        .unwrap();
        assert!(matches!(error, Error::Cancelled));
        assert_eq!(root.path().read_dir().unwrap().count(), 0);
    }
    for variant in 0..3 {
        let root = tempfile::tempdir().unwrap();
        let mut o = options();
        match variant {
            0 => o.represented = None,
            1 => {
                o.fixed_fiber
                    .insert(symbol!("graph_adapter::a"), Rational::one());
            }
            _ => o
                .threshold
                .kinematics
                .runtime_parameters
                .push(symbol!("graph_adapter::a")),
        }
        assert!(matches!(
            prepare_graph(graph(true), o, root.path(), |_| panic!(
                "unsupported before work"
            )),
            Err(Error::Unsupported(_))
        ));
        assert_eq!(root.path().read_dir().unwrap().count(), 0);
    }
}
