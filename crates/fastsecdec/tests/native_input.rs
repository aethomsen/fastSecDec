use idenso::tensor::SymbolicTensor;
use std::{collections::BTreeMap, sync::Arc};

use fastsecdec::{
    Atom, AtomCore, EdgeId, Error, FeynmanDiagram, IntegralFamily, Kinematics, Model,
    input::{GraphIntegral, default_algebra_settings, prepare_family_input},
    parametric::ScalarParametricIntegral,
};
use feynkit_graph::symbols;
use symbolica::{parse, symbol};

const MODEL: &str = include_str!("../../../examples/models/massless_phi3.json");
const BUBBLE: &str = include_str!("../../../examples/graphs/bubble.dot");

fn model() -> Arc<Model> {
    Arc::new(Model::from_json(MODEL).unwrap())
}

fn kinematics() -> Kinematics {
    Kinematics::in_dimension(&parse!("D"))
        .unwrap()
        // Native routing chooses P(0) as the dependent external momentum.
        .with_mass_squared(&symbols::external_momentum().call(1), parse!("s"))
        .unwrap()
}

#[test]
fn native_dot_and_linnet_remain_the_graph_owner() {
    let diagram = Arc::new(FeynmanDiagram::from_dot(model(), BUBBLE).unwrap());
    let integral = GraphIntegral::new(Arc::clone(&diagram), &kinematics()).unwrap();
    assert!(Arc::ptr_eq(&diagram, integral.diagram()));
    let graph = integral.diagram().underlying();
    assert_eq!(graph.n_edges(), 4);
    assert_eq!(graph.n_nodes(), 2);
    assert_eq!(integral.diagram().loop_count(), 1);
    assert_eq!(integral.propagator_edges(), &[EdgeId(2), EdgeId(3)]);
    assert_eq!(integral.powers(), &[1, 1]);

    let exported = integral.diagram().to_dot().unwrap();
    let restored = GraphIntegral::from_dot(model(), &exported, &kinematics()).unwrap();
    assert_eq!(restored.diagram().to_dot().unwrap(), exported);
    assert_eq!(
        restored.family().denominators(),
        integral.family().denominators()
    );
}

#[test]
fn hepkit_symanzik_and_measure_have_the_expected_signs() {
    let integral = GraphIntegral::from_dot(model(), BUBBLE, &kinematics()).unwrap();
    let parametric = ScalarParametricIntegral::from_graph(
        &integral,
        vec![parse!("x"), parse!("y")],
        parse!("4-2*eps"),
    )
    .unwrap();
    assert_eq!(parametric.u(), &parse!("x+y"));
    assert!(
        (parametric.f() + parse!("s*x*y")).expand().is_zero(),
        "F = {}",
        parametric.f()
    );
    assert_eq!(parametric.u_exponent().expand(), parse!("-2+2*eps"));
    assert_eq!(parametric.f_exponent().expand(), parse!("-eps"));
    assert_eq!(parametric.prefactor(), &parse!("gamma(eps)"));
    let laurent = parametric
        .prefactor()
        .series(symbol!("eps"), 0, 0)
        .unwrap()
        .to_atom();
    assert_eq!(laurent, parse!("1/eps-euler_gamma"));
}

#[test]
fn raised_powers_use_stable_edges_and_gamma_normalization() {
    let integral = GraphIntegral::from_dot(model(), BUBBLE, &kinematics())
        .unwrap()
        .with_powers(&BTreeMap::from([(EdgeId(2), 2)]))
        .unwrap()
        .with_measure_multiplier(parse!("normalization"));
    let parametric = ScalarParametricIntegral::from_graph(
        &integral,
        vec![parse!("x"), parse!("y")],
        Atom::num(4),
    )
    .unwrap();
    assert_eq!(parametric.powers(), [2, 1]);
    assert_eq!(parametric.prefactor(), &parse!("-normalization"));
    assert_eq!(parametric.u_exponent(), &Atom::num(-1));
    assert_eq!(parametric.f_exponent(), &Atom::num(-1));
    assert!(matches!(
        integral
            .clone()
            .with_powers(&BTreeMap::from([(EdgeId(0), 2)])),
        Err(Error::UnknownPropagator(EdgeId(0)))
    ));
    assert!(matches!(
        integral.with_powers(&BTreeMap::from([(EdgeId(2), 0)])),
        Err(Error::InvalidPower { .. })
    ));
}

#[test]
fn graph_weights_are_applied_once_without_automorphism_division() {
    let dot = r#"digraph weighted_bubble {
        graph [num="3", overall_factor="5", projector="11"];
        ext [style=invis];
        a [num="2"];
        ext -> a [particle="phi"];
        b -> ext [particle="phi"];
        a -> b [particle="phi", num="7", lmb_id=0];
        b -> a [particle="phi"];
    }"#;
    let diagram = FeynmanDiagram::from_dot(model(), dot).unwrap();
    let stable_dot = diagram
        .to_dot()
        .unwrap()
        .replace("symmetry_factor=1,", "symmetry_factor=13,");
    let integral = GraphIntegral::from_dot(model(), &stable_dot, &kinematics()).unwrap();
    assert_eq!(integral.diagram().symmetry_factor(), 13);
    assert_eq!(
        integral
            .scalar_numerator(&default_algebra_settings())
            .unwrap(),
        Atom::num(2310)
    );
}

#[test]
fn tensor_contraction_reuses_native_momentum_and_dimension() {
    let diagram = FeynmanDiagram::from_dot(model(), BUBBLE)
        .unwrap()
        .with_numerator(
            SymbolicTensor::infer(parse!(
                "gammalooprs::K(0,spenso::mink(4,mu))*gammalooprs::K(0,spenso::mink(4,mu))"
            ))
            .unwrap(),
        )
        .unwrap();
    let integral = GraphIntegral::new(Arc::new(diagram), &kinematics()).unwrap();
    let numerator = integral
        .scalar_numerator(&default_algebra_settings())
        .unwrap();
    let k = symbols::loop_momentum().call(0);
    assert_eq!(
        numerator,
        integral
            .family()
            .kinematics()
            .scalar_product(&k, &k)
            .unwrap()
    );
    assert!(matches!(
        ScalarParametricIntegral::from_graph(
            &integral,
            vec![parse!("x"), parse!("y")],
            parse!("4-2*eps")
        ),
        Err(Error::LoopNumerator)
    ));
}

#[test]
fn residual_free_indices_are_rejected() {
    let diagram = FeynmanDiagram::from_dot(model(), BUBBLE)
        .unwrap()
        .with_numerator(
            SymbolicTensor::infer(parse!("gammalooprs::K(0,spenso::mink(4,mu))")).unwrap(),
        )
        .unwrap();
    let integral = GraphIntegral::new(Arc::new(diagram), &kinematics()).unwrap();
    assert!(matches!(
        integral.scalar_numerator(&default_algebra_settings()),
        Err(Error::FreeTensorIndices)
    ));
}

#[test]
fn concrete_dimension_is_not_reinterpreted_as_a_symbolic_regulator() {
    let integral = GraphIntegral::from_dot(model(), BUBBLE, &Kinematics::new()).unwrap();
    assert!(matches!(
        ScalarParametricIntegral::from_graph(
            &integral,
            vec![parse!("x"), parse!("y")],
            parse!("4-2*eps")
        ),
        Err(Error::ConcreteDimensionMismatch)
    ));
}

fn modified_model(modify: impl FnOnce(&mut serde_json::Value)) -> Arc<Model> {
    let mut json: serde_json::Value = serde_json::from_str(MODEL).unwrap();
    modify(&mut json);
    Arc::new(Model::from_json(&json.to_string()).unwrap())
}

#[test]
fn widths_are_rejected_before_the_quadratic_family_can_omit_them() {
    let model = modified_model(|json| {
        json["parameters"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "name": "width", "lhablock": null, "lhacode": null,
                "nature": "external", "parameter_type": "real",
                "value": [0.1, 0.0], "expression": null
            }));
        json["particles"][0]["width"] = serde_json::json!("width");
    });
    assert!(matches!(
        GraphIntegral::from_dot(model, BUBBLE, &kinematics()),
        Err(Error::UnsupportedWidth {
            edge: EdgeId(2),
            ..
        })
    ));
}

#[test]
fn atomic_scalar_point_admits_zero_builtin_width_and_preserves_native_family() {
    let native_model = Arc::new(Model::standard_model());
    let original_model = native_model.to_json().unwrap();
    let diagram = Arc::new(
        FeynmanDiagram::from_dot(
            Arc::clone(&native_model),
            r#"
        digraph top_bubble {
            ext [style=invis];
            ext -> a [particle="g"];
            b -> ext [particle="g"];
            a -> b [particle="t", lmb_id=0];
            b -> a [particle="t"];
        }
    "#,
        )
        .unwrap()
        .with_numerator(SymbolicTensor::infer(Atom::one()).unwrap())
        .unwrap(),
    );
    let basis = diagram.loop_momentum_basis();
    let independent = basis
        .external_edges
        .iter()
        .position(|edge| !basis.dependent_externals.contains(edge))
        .unwrap();
    let kin = Kinematics::in_dimension(&parse!("D"))
        .unwrap()
        .with_mass_squared(&symbols::external_momentum().call(independent), parse!("s"))
        .unwrap();
    assert!(matches!(
        GraphIntegral::new(Arc::clone(&diagram), &kin),
        Err(Error::UnsupportedWidth { .. })
    ));
    for width in [Atom::num(1), parse!("unresolved_width")] {
        assert!(matches!(
            GraphIntegral::new_with_scalar_values(
                Arc::clone(&diagram),
                &kin,
                &BTreeMap::from([(symbol!("UFO::WT"), width)])
            ),
            Err(Error::UnsupportedWidth { .. })
        ));
    }
    let values = BTreeMap::from([
        (symbol!("UFO::WT"), Atom::Zero),
        (symbol!("UFO::MT"), Atom::num((345, 2))),
        (symbol!("s"), Atom::num(-1)),
    ]);
    let integral =
        GraphIntegral::new_with_scalar_values(Arc::clone(&diagram), &kin, &values).unwrap();
    assert!(Arc::ptr_eq(integral.diagram(), &diagram));
    assert_eq!(native_model.to_json().unwrap(), original_model);
    assert_eq!(
        integral.family().kinematics().dimension().to_symbolic(),
        parse!("D")
    );
    let parametric = ScalarParametricIntegral::from_graph(
        &integral,
        vec![parse!("x"), parse!("y")],
        parse!("2-2*eps"),
    )
    .unwrap();
    assert!(
        (parametric.f() - parse!("(345/2)^2*(x+y)^2+x*y"))
            .expand()
            .is_zero(),
        "F={}, denominators={:?}, externals={:?}",
        parametric.f(),
        integral.family().denominators(),
        integral.family().external_momenta(),
    );
    assert_eq!(parametric.u_exponent().expand(), parse!("2*eps"));
    assert_eq!(parametric.f_exponent().expand(), parse!("-1-eps"));
    let mut invalid = values.clone();
    invalid.insert(symbol!("UFO::MT"), Atom::num(1) + Atom::i());
    assert!(matches!(
        GraphIntegral::new_with_scalar_values(Arc::clone(&diagram), &kin, &invalid),
        Err(Error::UnsupportedMass { .. })
    ));
    invalid = values.clone();
    invalid.insert(symbol!("D"), Atom::num(4));
    assert!(matches!(
        GraphIntegral::new_with_scalar_values(Arc::clone(&diagram), &kin, &invalid),
        Err(Error::ScalarBindings(_))
    ));
    invalid = values;
    invalid.insert(symbols::loop_momentum(), Atom::num(1));
    assert!(matches!(
        GraphIntegral::new_with_scalar_values(diagram, &kin, &invalid),
        Err(Error::ScalarBindings(_))
    ));
}

#[test]
fn complex_masses_are_rejected_before_the_quadratic_family_can_omit_them() {
    let model = modified_model(|json| {
        json["parameters"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "name": "mass", "lhablock": null, "lhacode": null,
                "nature": "external", "parameter_type": "complex",
                "value": [1.0, 0.1], "expression": null
            }));
        json["particles"][0]["mass"] = serde_json::json!("mass");
    });
    assert!(matches!(
        GraphIntegral::from_dot(model, BUBBLE, &kinematics()),
        Err(Error::UnsupportedMass {
            edge: EdgeId(2),
            ..
        })
    ));
}

#[test]
fn custom_denominators_are_rejected_before_the_quadratic_family_can_omit_them() {
    let model = modified_model(|json| {
        json["propagators"][0]["denominator"] = serde_json::json!("(UFO::P(UFO::idx(1,1)))^4");
    });
    let diagram = Arc::new(FeynmanDiagram::from_dot(Arc::clone(&model), BUBBLE).unwrap());
    assert!(matches!(
        GraphIntegral::from_dot(model, BUBBLE, &kinematics()),
        Err(Error::UnsupportedDenominator {
            edge: EdgeId(2),
            ..
        })
    ));
    assert!(matches!(
        GraphIntegral::new_with_scalar_values(
            diagram,
            &kinematics(),
            &BTreeMap::from([(symbol!("UFO::ZERO"), Atom::Zero),])
        ),
        Err(Error::UnsupportedDenominator { .. })
    ));
}

#[test]
fn parameter_names_cannot_capture_external_weights_or_the_regulator() {
    let integral = GraphIntegral::from_dot(model(), BUBBLE, &kinematics())
        .unwrap()
        .with_measure_multiplier(parse!("x"));
    assert!(matches!(
        ScalarParametricIntegral::from_graph(
            &integral,
            vec![parse!("x"), parse!("y")],
            parse!("4-2*eps")
        ),
        Err(Error::ParameterCollision(_))
    ));
    let integral = GraphIntegral::from_dot(model(), BUBBLE, &kinematics()).unwrap();
    assert!(matches!(
        ScalarParametricIntegral::from_graph(
            &integral,
            vec![parse!("eps"), parse!("y")],
            parse!("4-2*eps")
        ),
        Err(Error::ParameterCollision(_))
    ));
}

#[test]
fn parameter_binding_precedes_native_scalelessness_and_updates_kinematics() {
    let graph = GraphIntegral::from_dot(model(), BUBBLE, &kinematics())
        .unwrap()
        .with_measure_multiplier(parse!("s"))
        .with_scalar_values(&BTreeMap::from([(symbol!("s"), Atom::num(-1))]))
        .unwrap();
    let (u, f) = graph
        .family()
        .symanzik(&[parse!("x"), parse!("y")])
        .unwrap();
    assert_eq!(u, parse!("x+y"));
    assert_eq!(f, parse!("x*y"));
    assert_eq!(graph.measure_multiplier(), &Atom::num(-1));
    let massless = GraphIntegral::from_dot(model(), BUBBLE, &kinematics())
        .unwrap()
        .with_scalar_values(&BTreeMap::from([(symbol!("s"), Atom::Zero)]))
        .unwrap();
    let integral = fastsecdec::parametric::ParametricIntegrand::from_graph(
        &massless,
        vec![symbol!("x"), symbol!("y")],
        symbol!("eps"),
        parse!("4-2*eps"),
    )
    .unwrap();
    assert!(integral.terms().is_empty());
}

#[test]
fn scalar_overrides_cannot_bypass_mass_and_width_admission() {
    let model = modified_model(|json| {
        for name in ["mass", "width"] {
            json["parameters"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!({
                    "name":name,"lhablock":null,"lhacode":null,"nature":"external",
                    "parameter_type":"real","value":[0.0,0.0],"expression":null
                }));
        }
        json["particles"][0]["mass"] = serde_json::json!("mass");
        json["particles"][0]["width"] = serde_json::json!("width");
        json["propagators"][0]["denominator"] =
            serde_json::json!("(UFO::P(UFO::idx(1,1)))^2-UFO::mass^2");
    });
    let integral = GraphIntegral::from_dot(model, BUBBLE, &kinematics()).unwrap();
    assert!(matches!(
        integral
            .clone()
            .with_scalar_values(&BTreeMap::from([(symbol!("UFO::width"), Atom::num(1))])),
        Err(Error::UnsupportedWidth { .. })
    ));
    let complex = Atom::num(symbolica::domains::float::Complex::new(
        symbolica::domains::rational::Rational::from(1),
        symbolica::domains::rational::Rational::from((1, 1000)),
    ));
    assert!(matches!(
        integral
            .clone()
            .with_scalar_values(&BTreeMap::from([(symbol!("UFO::mass"), complex)])),
        Err(Error::UnsupportedMass { .. })
    ));
    assert!(
        integral
            .with_scalar_values(&BTreeMap::from([
                (symbol!("UFO::mass"), Atom::num(2)),
                (symbol!("UFO::width"), Atom::Zero),
            ]))
            .is_ok()
    );
}

#[test]
fn scalar_bindings_and_dimension_names_ending_in_underscore_are_literal() {
    let dimension = parse!("native_dimension_");
    let kin = Kinematics::in_dimension(&dimension)
        .unwrap()
        .with_mass_squared(
            &symbols::external_momentum().call(1),
            parse!("native_invariant_"),
        )
        .unwrap();
    let graph = GraphIntegral::from_dot(model(), BUBBLE, &kin)
        .unwrap()
        .with_measure_multiplier(parse!("native_weight_*(native_dimension_+untouched)"))
        .with_scalar_values(&BTreeMap::from([
            (symbol!("native_invariant_"), Atom::num(-1)),
            (symbol!("native_weight_"), Atom::num(3)),
        ]))
        .unwrap();
    assert_eq!(
        graph.measure_multiplier(),
        &parse!("3*(native_dimension_+untouched)")
    );
    let scalar = ScalarParametricIntegral::from_graph(
        &graph,
        vec![parse!("x"), parse!("y")],
        parse!("4-2*eps"),
    )
    .unwrap();
    assert_eq!(scalar.f(), &parse!("x*y"));
    assert_eq!(
        scalar.prefactor(),
        &parse!("3*(4-2*eps+untouched)*gamma(eps)")
    );
    let general = fastsecdec::parametric::ParametricIntegrand::from_graph(
        &graph,
        vec![symbol!("x"), symbol!("y")],
        symbol!("eps"),
        parse!("4-2*eps"),
    )
    .unwrap();
    assert_eq!(general.terms()[0].prefactor(), scalar.prefactor());
}

#[test]
fn signed_family_projection_preserves_native_numerator_and_specialization() {
    let k = parse!("family_input_k");
    let p = parse!("family_input_p");
    let e = parse!("family_input_e");
    let kin = Kinematics::in_dimension(&parse!("family_input_D"))
        .unwrap()
        .with_scalar_product(&p, &p, parse!("family_input_s"))
        .unwrap();
    let kk = kin.scalar_product(&k, &k).unwrap();
    let kp = kin.scalar_product(&k, &p).unwrap();
    let family = IntegralFamily::new(
        vec![k.clone()],
        vec![p.clone()],
        vec![&kk - parse!("family_input_m2"), kp.clone(), &kk + &kp],
        &kin,
    )
    .unwrap();
    let extended = kin
        .clone()
        .with_scalar_product(&p, &p, Atom::num(-1))
        .unwrap()
        .with_scalar_product(&e, &e, Atom::num(-2))
        .unwrap()
        .with_scalar_product(&p, &e, Atom::num(3))
        .unwrap();
    let bindings = BTreeMap::from([
        (symbol!("family_input_s"), Atom::num(-1)),
        (symbol!("family_input_m2"), Atom::num(2)),
        (symbol!("family_input_weight"), Atom::num(5)),
    ]);
    let ke = extended.scalar_product(&k, &e).unwrap();
    let (projected, powers, numerator) = prepare_family_input(
        &family,
        &[3, -2, 0],
        parse!("family_input_weight") * &ke,
        Some(&extended),
        &bindings,
        std::slice::from_ref(&e),
    )
    .unwrap();
    assert_eq!(projected.denominators(), &[&kk - 2]);
    assert_eq!(powers, [3]);
    assert_eq!(numerator, Atom::num(5) * ke * kp.pow(Atom::num(2)));
    assert_eq!(projected.external_momenta(), &[p.clone(), e.clone()]);
    assert_eq!(
        projected.kinematics().scalar_product(&p, &p).unwrap(),
        Atom::num(-1)
    );
    assert_eq!(
        projected.kinematics().scalar_product(&e, &e).unwrap(),
        Atom::num(-2)
    );
    // The input remains the native symbolic owner, with no changed assumptions.
    assert_eq!(
        family.kinematics().scalar_product(&p, &p).unwrap(),
        parse!("family_input_s")
    );
    assert_eq!(family.denominators()[0], &kk - parse!("family_input_m2"));
}

#[test]
fn family_input_rejects_stale_kinematics_empty_support_and_momentum_bindings() {
    let k = parse!("family_guard_k");
    let p = parse!("family_guard_p");
    let kin = Kinematics::in_dimension(&parse!("family_guard_D"))
        .unwrap()
        .with_scalar_product(&p, &p, Atom::num(-1))
        .unwrap();
    let family = IntegralFamily::new(
        vec![k.clone()],
        vec![p.clone()],
        vec![kin.scalar_product(&k, &k).unwrap() - parse!("family_guard_m2")],
        &kin,
    )
    .unwrap();
    let conflicting = kin.with_scalar_product(&p, &p, Atom::num(-2)).unwrap();
    assert!(matches!(
        prepare_family_input(
            &family,
            &[2],
            Atom::one(),
            Some(&conflicting),
            &BTreeMap::new(),
            &[],
        ),
        Err(Error::ScalarBindings(_))
    ));
    let kk = family.kinematics().scalar_product(&k, &k).unwrap();
    let unresolved_pp = Kinematics::in_dimension(&parse!("family_guard_D"))
        .unwrap()
        .scalar_product(&p, &p)
        .unwrap();
    for bindings in [
        BTreeMap::from([(symbol!("family_guard_k"), Atom::num(1))]),
        BTreeMap::from([(symbol!("family_guard_p"), Atom::num(1))]),
        BTreeMap::from([(symbol!("family_guard_D"), Atom::num(4))]),
        BTreeMap::from([(symbol!("family_guard_s"), k)]),
        BTreeMap::from([(symbol!("family_guard_s"), p)]),
        // Native dot products use momentum function heads, not bare variables.
        // k²/2 would otherwise silently produce a different valid quadratic
        // denominator, rather than necessarily failing later with U=0.
        BTreeMap::from([(symbol!("family_guard_m2"), &kk / 2)]),
        BTreeMap::from([(symbol!("family_guard_m2"), kk)]),
        BTreeMap::from([(symbol!("family_guard_m2"), unresolved_pp)]),
    ] {
        assert!(matches!(
            prepare_family_input(&family, &[2], Atom::one(), None, &bindings, &[],),
            Err(Error::ScalarBindings(_))
        ));
    }
    for powers in [vec![], vec![0], vec![-1], vec![i32::MIN]] {
        assert!(
            prepare_family_input(&family, &powers, Atom::one(), None, &BTreeMap::new(), &[],)
                .is_err()
        );
    }
}
