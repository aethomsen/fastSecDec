//! Native contraction of indexed sums must reach Gaussian scalar products.
use idenso::tensor::SymbolicTensor;
use std::sync::Arc;

use fastsecdec::{
    Atom, AtomCore, FeynmanDiagram, Kinematics, Model,
    input::{GraphIntegral, NumeratorContraction},
    parametric::ParametricIntegrand,
};
use feynkit_graph::symbols;
use spenso::structure::representation::{Minkowski, RepName};
use symbolica::{parse, symbol};

#[test]
fn dots_and_full_contract_connected_sums_and_preserve_graph_weights() {
    let model = Arc::new(
        Model::from_json(include_str!("../../../examples/models/massless_phi3.json")).unwrap(),
    );
    let diagram =
        FeynmanDiagram::from_dot(model, include_str!("../../../examples/graphs/bubble.dot"))
            .unwrap();
    let k = symbols::loop_momentum().call(0);
    let p = symbols::external_momentum().call(1);
    let kin = Kinematics::in_dimension(&parse!("D"))
        .unwrap()
        .with_mass_squared(&p, parse!("s"))
        .unwrap();
    // Concrete indices are promoted to the input's native dimension on admission.
    let rep = Minkowski {}.new_rep(4);
    let kmu = rep.vector(k.as_view(), [Atom::num(1)]);
    let pmu = rep.vector(p.as_view(), [Atom::num(1)]);
    let numerator = (&kmu + &pmu) * (Atom::num(2) * &kmu - Atom::num(3) * &pmu);
    let diagram = diagram
        .with_numerator(SymbolicTensor::infer(numerator).unwrap())
        .unwrap()
        .with_projector(SymbolicTensor::infer(Atom::num(5)).unwrap())
        .with_numerator_prefactor(SymbolicTensor::infer(Atom::num(13)).unwrap())
        .with_overall_factor(Atom::num(7));
    let original = diagram.to_json().unwrap();
    let graph = GraphIntegral::new(Arc::new(diagram), &kin)
        .unwrap()
        .with_measure_multiplier(Atom::num(11));
    let expected = Atom::num(455)
        * (Atom::num(2) * kin.scalar_product(&k, &k).unwrap()
            - kin.scalar_product(&k, &p).unwrap()
            - Atom::num(3) * parse!("s"));
    let mut densities = Vec::new();
    for mode in [NumeratorContraction::Dots, NumeratorContraction::Full] {
        let scalar = graph.scalar_numerator(&mode.algebra_settings()).unwrap();
        assert!(
            (&scalar - &expected).expand().is_zero(),
            "{mode:?}: {scalar}"
        );
        let integrand = ParametricIntegrand::from_family(
            graph.family(),
            graph.powers(),
            scalar * graph.measure_multiplier(),
            vec![symbol!("x"), symbol!("y")],
            symbol!("eps"),
            parse!("4-2*eps"),
        )
        .unwrap();
        densities.push(integrand.density());
    }
    assert!((densities[0].clone() - &densities[1]).expand().is_zero());
    assert_eq!(graph.diagram().to_json().unwrap(), original);
}
