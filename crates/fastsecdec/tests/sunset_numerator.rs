//! Coupled two-loop Gaussian numerator: native routing and independent density.
use idenso::tensor::SymbolicTensor;
use std::{collections::BTreeMap, sync::Arc};

use fastsecdec::{
    Atom, AtomCore, Kinematics, Model,
    input::{GraphIntegral, default_algebra_settings},
    parametric::{ParametricIntegrand, ParametricTerm},
};
use feynkit_graph::symbols;
use serde::Deserialize;
use symbolica::{
    id::Pattern, parse, parser::ParseSettings, symbol, transcendental::TranscendentalFunctions,
};

#[path = "support/sunset_integration.rs"]
mod integration;

const DOT: &str = include_str!("../../../examples/graphs/sunset_2loop_numerator.dot");

fn graph(scale: i64) -> GraphIntegral {
    let model =
        Arc::new(Model::from_json(include_str!("../../../examples/models/scalar.json")).unwrap());
    let p = symbols::external_momentum().call(0);
    let kinematics = Kinematics::in_dimension(&parse!("sunset::D"))
        .unwrap()
        .with_mass_squared(&p, Atom::num(-scale))
        .unwrap();
    GraphIntegral::from_dot(model, DOT, &kinematics)
        .unwrap()
        .with_scalar_values(&BTreeMap::from([(symbol!("UFO::mt"), Atom::Zero)]))
        .unwrap()
}

fn parametric(graph: &GraphIntegral, dimension: Atom) -> ParametricIntegrand {
    ParametricIntegrand::from_graph(
        graph,
        (0..3)
            .map(|index| symbol!(format!("sunset::x{index}")))
            .collect(),
        symbol!("sunset::eps"),
        dimension,
    )
    .unwrap()
}

fn expression(text: &str) -> Atom {
    Atom::parse(text.replace("**", "^"), "sunset", ParseSettings::default()).unwrap()
}

fn at(expression: &Atom, sample: &Sample) -> f64 {
    let mut value = expression.clone();
    for (index, coordinate) in sample.coordinates.iter().enumerate() {
        value = value
            .replace(Pattern::Literal(parse!(format!("sunset::x{index}"))))
            .with(Pattern::Literal(self::expression(coordinate)));
    }
    value = value
        .replace(Pattern::Literal(parse!("sunset::eps")))
        .with(Pattern::Literal(self::expression(&sample.epsilon)));
    f64::try_from(&value).unwrap()
}

#[derive(Deserialize)]
struct Oracle {
    u: String,
    f: String,
    gamma_factor: String,
    samples: Vec<Sample>,
}

#[derive(Deserialize)]
struct Sample {
    coordinates: [String; 3],
    epsilon: String,
    gamma_stripped_density: String,
    full_density: Option<String>,
}

#[test]
fn native_sunset_preserves_coupled_propagators_and_numerator() {
    let graph = graph(1);
    let family = graph.family();
    assert_eq!(graph.diagram().loop_count(), 2);
    assert_eq!(graph.powers(), [1, 1, 1]);
    let k = family.loop_momenta();
    let p = symbols::external_momentum().call(0);
    let dot = |left: &Atom, right: &Atom| family.kinematics().scalar_product(left, right).unwrap();
    let third = &k[0] + &k[1] + &p;
    let expected = [dot(&k[0], &k[0]), dot(&k[1], &k[1]), dot(&third, &third)];
    for (actual, expected) in family.denominators().iter().zip(expected) {
        assert!((actual - expected).expand().is_zero(), "{actual}");
    }
    let expected: Atom = dot(&k[0], &k[1]) + 2 * dot(&k[0], &p);
    assert!(
        (graph.scalar_numerator(&default_algebra_settings()).unwrap() - expected)
            .expand()
            .is_zero()
    );
    let stable = graph.diagram().to_dot().unwrap();
    let reloaded =
        GraphIntegral::from_dot(graph.diagram().model_arc(), &stable, family.kinematics())
            .unwrap()
            .with_scalar_values(&BTreeMap::from([(symbol!("UFO::mt"), Atom::Zero)]))
            .unwrap();
    assert_eq!(reloaded.diagram().to_dot().unwrap(), stable);
    assert_eq!(reloaded.family().denominators(), family.denominators());
    assert_eq!(
        reloaded
            .scalar_numerator(&default_algebra_settings())
            .unwrap(),
        graph.scalar_numerator(&default_algebra_settings()).unwrap(),
    );
    assert_eq!(
        reloaded.diagram().overall_factor(),
        graph.diagram().overall_factor()
    );
    assert_eq!(reloaded.diagram().projector(), graph.diagram().projector());
    // The numerator lives in a native vertex fragment, so the native setter
    // produces the scalar control without parsing or modifying DOT text.
    let scalar = graph
        .diagram()
        .as_ref()
        .clone()
        .with_numerator(SymbolicTensor::infer(Atom::one()).unwrap())
        .unwrap();
    let restored = GraphIntegral::new(Arc::new(scalar), family.kinematics())
        .unwrap()
        .with_scalar_values(&BTreeMap::from([(symbol!("UFO::mt"), Atom::Zero)]))
        .unwrap();
    assert_eq!(restored.family().denominators(), family.denominators());
    assert_eq!(
        restored
            .scalar_numerator(&default_algebra_settings())
            .unwrap(),
        Atom::one()
    );
}

#[test]
fn coupled_gaussian_density_matches_independent_pysecdec_parameterization() {
    let graph = graph(1);
    let input = parametric(&graph, parse!("4-2*sunset::eps"));
    let oracle: Oracle = serde_json::from_str(include_str!(
        "../../../examples/references/sunset_2loop_numerator_density.json"
    ))
    .unwrap();
    let (u, f) = graph
        .family()
        .symanzik(
            &input
                .parameters()
                .iter()
                .map(|p| Atom::var(*p))
                .collect::<Vec<_>>(),
        )
        .unwrap();
    assert!((u - expression(&oracle.u)).expand().is_zero());
    assert!((f - expression(&oracle.f)).expand().is_zero());
    let density = input.density();
    // Cancel the common Gamma factor while it is still factored in each
    // prefactor, before evaluating the removable eps=0 pole.
    let gamma = expression(&oracle.gamma_factor);
    let gamma_stripped = ParametricIntegrand::new(
        input.parameters().to_vec(),
        input.regulator(),
        input.domain(),
        input
            .terms()
            .iter()
            .map(|term| {
                ParametricTerm::new(
                    term.prefactor() / &gamma,
                    term.monomial_powers().to_vec(),
                    term.factors().to_vec(),
                )
            })
            .collect(),
    )
    .unwrap()
    .density();
    assert_eq!(oracle.samples.len(), 9);
    for sample in &oracle.samples {
        let expected: f64 = sample.gamma_stripped_density.parse().unwrap();
        let actual = at(&gamma_stripped, sample);
        assert!((actual - expected).abs() < 2e-12 * expected.abs().max(1.0));
        if let Some(expected) = &sample.full_density {
            let expected: f64 = expected.parse().unwrap();
            let actual = at(&density, sample);
            assert!((actual - expected).abs() < 2e-12 * expected.abs().max(1.0));
        }
    }
}

#[test]
fn scalar_sunset_in_its_convergence_domain_checks_the_measure_sign() {
    let graph = graph(1);
    let scalar = graph
        .diagram()
        .as_ref()
        .clone()
        .with_numerator(SymbolicTensor::infer(Atom::one()).unwrap())
        .unwrap();
    let scalar = GraphIntegral::new(Arc::new(scalar), graph.family().kinematics())
        .unwrap()
        .with_scalar_values(&BTreeMap::from([(symbol!("UFO::mt"), Atom::Zero)]))
        .unwrap();
    let input = parametric(&scalar, parse!("5/2"));
    // Grozin Eq.(4.3), composed twice; change all three negative propagators
    // to our positive q^2 convention. At 2<D<3 this is also a convergent Wick
    // integral, so its negative sign is checked before Laurent continuation.
    let reference =
        -Atom::num((1, 2)).gamma() * Atom::num((1, 4)).gamma().pow(3) / Atom::num((3, 4)).gamma();
    let expected = f64::try_from(&reference).unwrap();
    assert!(expected < 0.0);
    let actual = integration::integrate(&input, 0, 41821, 65536);
    assert_eq!(actual.orders, [0]);
    integration::compare(0, actual.mean[0], actual.standard_error[0], expected);
}

#[test]
fn coupled_sunset_complete_laurent_vector_matches_the_scalar_identity() {
    let eps = symbol!("sunset::eps");
    for scale in [1, 2] {
        let input = parametric(&graph(scale), parse!("4-2*sunset::eps"));
        // q1+q2+q3=-p. Permutation symmetry and scaleless pinches imply
        // integral(q1.q2)=p^2*I/6 and integral(q1.p)=-p^2*I/3.
        // Thus integral(q1.q2+2q1.p)=-p^2*I/2. This is a test identity,
        // not a new multiloop reduction implementation.
        let reference = Atom::num(scale).pow(parse!("2-2*sunset::eps"))
            * parse!("-gamma(-1+2*sunset::eps)*gamma(1-sunset::eps)^3/(2*gamma(3-3*sunset::eps))");
        let series = reference.series(eps, 0, 1).unwrap();
        let pole = f64::try_from(&series.coefficient((-1).into()).unwrap()).unwrap();
        assert!((pole - (scale * scale) as f64 / 8.0).abs() < 1e-14);
        let actual = integration::integrate(&input, 1, 41822 + scale as u64, 4096);
        assert_eq!(actual.orders.last(), Some(&1));
        assert!(actual.orders.contains(&-1));
        for (index, order) in actual.orders.iter().enumerate() {
            let coefficient = series.coefficient(i64::from(*order).into()).unwrap();
            let expected = f64::try_from(&coefficient).unwrap();
            integration::compare(
                *order,
                actual.mean[index],
                actual.standard_error[index],
                expected,
            );
        }
    }
}
