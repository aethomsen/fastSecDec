//! Gaussian helicity sources checked against HEPKit's transverse projector.
use idenso::tensor::SymbolicTensor;
use std::{collections::BTreeMap, sync::Arc};

use fastsecdec::{
    Atom, AtomCore, Error, Kinematics, Model,
    input::{GraphIntegral, default_algebra_settings},
    parametric::ParametricIntegrand,
};
use feynkit_graph::{IntegralFamilyError, symbols};
use feynkit_tensor::TensorReducer;
use symbolica::{parse, symbol};

fn input(e2_squared: Atom, mutual: Atom) -> (GraphIntegral, [Atom; 2]) {
    let e1 = Atom::var(spenso::vector_symbol!("auxiliary_test::e1"));
    let e2 = Atom::var(spenso::vector_symbol!("auxiliary_test::e2"));
    let p = symbols::external_momentum().call(1);
    let k = symbols::loop_momentum().call(0);
    let kin = Kinematics::in_dimension(&parse!("auxiliary_test::D"))
        .unwrap()
        .with_mass_squared(&p, Atom::num(-1))
        .unwrap()
        .with_mass_squared(&e1, Atom::num(-1))
        .unwrap()
        .with_mass_squared(&e2, e2_squared)
        .unwrap()
        .with_scalar_product(&e1, &e2, mutual)
        .unwrap()
        .with_scalar_product(&p, &e1, Atom::Zero)
        .unwrap()
        .with_scalar_product(&p, &e2, Atom::Zero)
        .unwrap();
    let model = Arc::new(
        Model::from_json(include_str!("../../../examples/models/massless_phi3.json")).unwrap(),
    );
    let graph = GraphIntegral::from_dot(
        model,
        include_str!("../../../examples/graphs/bubble.dot"),
        &kin,
    )
    .unwrap();
    let numerator = kin.scalar_product(&k, &e1).unwrap() * kin.scalar_product(&k, &e2).unwrap();
    let diagram = graph
        .diagram()
        .as_ref()
        .clone()
        .with_numerator(SymbolicTensor::infer(numerator).unwrap())
        .unwrap();
    (
        GraphIntegral::new(Arc::new(diagram), &kin).unwrap(),
        [e1, e2],
    )
}

fn density(graph: &GraphIntegral) -> Atom {
    ParametricIntegrand::from_graph(
        graph,
        vec![symbol!("auxiliary_test::x"), symbol!("auxiliary_test::y")],
        symbol!("auxiliary_test::eps"),
        parse!("4-2*auxiliary_test::eps"),
    )
    .unwrap()
    .density()
}

#[test]
fn transverse_helicity_sources_match_native_reducer_with_degenerate_gram() {
    for (norm, mutual) in [(Atom::num(-1), Atom::num(-1)), (Atom::one(), Atom::i())] {
        let (original, auxiliaries) = input(norm, mutual);
        let graph = original
            .clone()
            .with_auxiliary_external_momenta(&auxiliaries)
            .unwrap();
        assert!(Arc::ptr_eq(original.diagram(), graph.diagram()));
        assert_eq!(
            original.family().denominators(),
            graph.family().denominators()
        );
        assert_eq!(original.powers(), graph.powers());

        let reduced = TensorReducer::new(parse!("auxiliary_test::D"))
            .with_integrated_head(symbols::loop_momentum())
            .with_external_vector(parse!("gammalooprs::P(1,spenso::mink(auxiliary_test::D))"))
            .reduce(original.diagram().numerator().expression().as_view())
            .unwrap();
        assert!(reduced.is_fully_contracted());
        let numerator = original.family().kinematics().apply(&reduced.expression());
        let diagram = original
            .diagram()
            .as_ref()
            .clone()
            .with_numerator(SymbolicTensor::infer(numerator).unwrap())
            .unwrap();
        let control =
            GraphIntegral::new(Arc::new(diagram), original.family().kinematics()).unwrap();

        // Compare pole, finite and next coefficient without a custom tensor
        // average or Gram solve. Only this small scalar identity is expanded.
        let difference = (density(&graph) - density(&control))
            .series(symbol!("auxiliary_test::eps"), 0, 1)
            .unwrap()
            .to_atom()
            .expand()
            .together()
            .cancel();
        assert!(difference.is_zero(), "{difference}");
    }
}

#[test]
fn auxiliary_bindings_commute_and_native_label_validation_is_preserved() {
    let value = symbol!("auxiliary_test::helicity_dot");
    let (graph, auxiliaries) = input(Atom::num(-1), Atom::var(value));
    let bindings = BTreeMap::from([(value, Atom::num(-1))]);
    let early = graph
        .clone()
        .with_scalar_values(&bindings)
        .unwrap()
        .with_auxiliary_external_momenta(&auxiliaries)
        .unwrap();
    let late = graph
        .clone()
        .with_auxiliary_external_momenta(&auxiliaries)
        .unwrap()
        .with_scalar_values(&bindings)
        .unwrap();
    assert_eq!(
        early
            .family()
            .kinematics()
            .scalar_product(&auxiliaries[0], &auxiliaries[1])
            .unwrap(),
        Atom::num(-1)
    );
    assert_eq!(
        early.scalar_numerator(&default_algebra_settings()).unwrap(),
        late.scalar_numerator(&default_algebra_settings()).unwrap()
    );
    assert!(matches!(
        graph
            .clone()
            .with_auxiliary_external_momenta(&[auxiliaries[0].clone(), auxiliaries[0].clone()]),
        Err(Error::IntegralFamily(
            IntegralFamilyError::RepeatedMomentum(_)
        ))
    ));
    let k = graph.family().loop_momenta()[0].clone();
    assert!(matches!(
        graph.with_auxiliary_external_momenta(&[k]),
        Err(Error::IntegralFamily(
            IntegralFamilyError::RepeatedMomentum(_)
        ))
    ));
}
