//! Public native-family entry: exact projection, Gaussian moments and weights.
use idenso::tensor::SymbolicTensor;
use std::sync::Arc;

use fastsecdec::{
    Atom, AtomCore, Error, IntegralFamily, Kinematics, Model,
    input::{GraphIntegral, default_algebra_settings},
    parametric::{ParametricIntegrand, ScalarParametricIntegral},
};
use feynkit_graph::symbols;
use symbolica::{id::Pattern, parse, symbol};

fn tadpole(mass_squared: Atom, duplicate: bool) -> (IntegralFamily, Atom) {
    let k = parse!("family_api::k");
    let kinematics = Kinematics::in_dimension(&parse!("family_api::D"))
        .unwrap()
        .with_momenta([k.clone()])
        .unwrap();
    let denominator = kinematics.scalar_product(&k, &k).unwrap() - mass_squared;
    let denominators = if duplicate {
        vec![denominator.clone(), denominator.clone()]
    } else {
        vec![denominator.clone()]
    };
    (
        IntegralFamily::new(vec![k], vec![], denominators, &kinematics).unwrap(),
        denominator,
    )
}

fn finite(expression: Atom) -> Atom {
    expression
        .series(symbol!("family_api::eps"), 0, 0)
        .unwrap()
        .to_atom()
        .cancel()
}

#[test]
fn native_partial_fraction_projection_preserves_a_raised_power_gaussian_moment() {
    let (family, denominator) = tadpole(Atom::one(), true);
    let original_powers = [2, 3];
    let terms = family.partial_fraction(&original_powers, 32).unwrap();
    assert_eq!(terms.len(), 1);
    let (coefficient, powers) = &terms[0];
    let original = denominator.pow(-2) * denominator.pow(-3);
    let reconstructed = coefficient
        * family
            .denominators()
            .iter()
            .zip(powers)
            .map(|(denominator, power)| denominator.pow(-*power))
            .product::<Atom>();
    assert_eq!(original, reconstructed);
    let projected = family.sector(powers).unwrap();
    let positive = powers
        .iter()
        .copied()
        .filter(|power| *power > 0)
        .map(|power| u32::try_from(power).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(positive, [5]);
    assert_eq!(projected.loop_momenta(), family.loop_momenta());

    let x = symbol!("family_api::x");
    let y = symbol!("family_api::y");
    let eps = symbol!("family_api::eps");
    let dimension = parse!("2-2*family_api::eps");
    // The numerator cancels one denominator: at D=2 the normalized massive
    // tadpole is 6*Gamma(4-D/2)/Gamma(4) = 2. Gaussian differentiation must
    // produce the same answer without implementing a numerator cancellation.
    let weighted_numerator = Atom::num(6) * &denominator;
    let reduced = ParametricIntegrand::from_family(
        &projected,
        &positive,
        coefficient * &weighted_numerator,
        vec![x],
        eps,
        dimension.clone(),
    )
    .unwrap();
    let value = finite(
        reduced
            .density()
            .replace(Pattern::Literal(Atom::var(x)))
            .with(Atom::one()),
    );
    assert_eq!(value, Atom::num(2));

    let original = ParametricIntegrand::from_family(
        &family,
        &[2, 3],
        weighted_numerator,
        vec![x, y],
        eps,
        dimension,
    )
    .unwrap();
    let density = finite(
        original
            .density()
            .replace(Pattern::Literal(Atom::var(y)))
            .with(Atom::one() - Atom::var(x)),
    );
    // The original two-parameter simplex reduces to an exact polynomial.
    // Symbolica owns its antiderivative; no quadrature or alternate CAS here.
    let primitive = density
        .to_polynomial_in_vars::<u32>(x)
        .integrate(0)
        .flatten(false);
    let original_value = primitive.replace(Pattern::Literal(Atom::var(x))).with(1)
        - primitive.replace(Pattern::Literal(Atom::var(x))).with(0);
    assert_eq!(original_value, value);
}

#[test]
fn graph_and_family_apply_native_weights_and_measure_exactly_once() {
    let model = Arc::new(
        Model::from_json(include_str!("../../../examples/models/massless_phi3.json")).unwrap(),
    );
    let kinematics = Kinematics::in_dimension(&parse!("family_api::D"))
        .unwrap()
        .with_mass_squared(&symbols::external_momentum().call(1), Atom::num(-1))
        .unwrap();
    let base = GraphIntegral::from_dot(
        model,
        include_str!("../../../examples/graphs/bubble.dot"),
        &kinematics,
    )
    .unwrap();
    let diagram = base
        .diagram()
        .as_ref()
        .clone()
        .with_numerator(SymbolicTensor::infer(Atom::num(5)).unwrap())
        .unwrap()
        .with_overall_factor(Atom::num(2))
        .with_projector(SymbolicTensor::infer(Atom::num(3)).unwrap());
    let graph = GraphIntegral::new(Arc::new(diagram), &kinematics)
        .unwrap()
        .with_measure_multiplier(Atom::num(7));
    let parameters = vec![symbol!("family_api::x_"), symbol!("family_api::y_")];
    let eps = symbol!("family_api::eps");
    let dimension = parse!("4-2*family_api::eps");
    let weighted =
        graph.scalar_numerator(&default_algebra_settings()).unwrap() * graph.measure_multiplier();
    assert_eq!(weighted, Atom::num(210));
    let native = ParametricIntegrand::from_family(
        graph.family(),
        graph.powers(),
        weighted,
        parameters.clone(),
        eps,
        dimension.clone(),
    )
    .unwrap();
    let delegated =
        ParametricIntegrand::from_graph(&graph, parameters.clone(), eps, dimension.clone())
            .unwrap();
    let scalar = ScalarParametricIntegral::from_graph(
        &base,
        parameters.iter().map(|p| Atom::var(*p)).collect(),
        dimension,
    )
    .unwrap();
    assert_eq!(native.density(), delegated.density());
    assert_eq!(scalar.powers(), [1, 1]);
    // The legacy scalar route retains an affine exponent such as 2-(4-2*eps),
    // while the Gaussian route normalizes that exponent. Normalize only these
    // small exponents for the exact identity, leaving the density factored.
    let scalar_density = scalar.prefactor()
        * scalar.u().pow(scalar.u_exponent().expand())
        * scalar.f().pow(scalar.f_exponent().expand());
    assert!(
        (native.density() - Atom::num(210) * scalar_density)
            .cancel()
            .is_zero()
    );
    assert_eq!(graph.family().denominators(), base.family().denominators());
}

#[test]
fn family_admission_rejects_zero_powers_and_parameter_aliases_before_algebra() {
    let (family, _) = tadpole(Atom::one(), false);
    let x = symbol!("family_api::x");
    let eps = symbol!("family_api::eps");
    let dimension = parse!("2-2*family_api::eps");
    for powers in [vec![], vec![0], vec![1, 1]] {
        assert!(
            ParametricIntegrand::from_family(
                &family,
                &powers,
                Atom::Zero,
                vec![x],
                eps,
                dimension.clone(),
            )
            .is_err()
        );
    }
    for (numerator, parameter, dim) in [
        (Atom::var(x), x, dimension.clone()),
        (Atom::one(), x, dimension.clone() + Atom::var(x)),
        (Atom::one(), symbol!("family_api::D"), dimension.clone()),
    ] {
        assert!(matches!(
            ParametricIntegrand::from_family(&family, &[4], numerator, vec![parameter], eps, dim),
            Err(Error::ParameterCollision(_))
        ));
    }
    assert!(
        ParametricIntegrand::from_family(
            &family,
            &[4],
            Atom::one(),
            vec![eps],
            eps,
            dimension.clone()
        )
        .is_err()
    );
    let (repeated, _) = tadpole(Atom::one(), true);
    assert!(
        ParametricIntegrand::from_family(
            &repeated,
            &[2, 2],
            Atom::one(),
            vec![x, x],
            eps,
            dimension.clone(),
        )
        .is_err()
    );
    // Physical mass x must not silently become the integration coordinate.
    // The native Symanzik label validator, not a new FastSecDec parser, rejects it.
    let (collision, _) = tadpole(Atom::var(x), false);
    assert!(matches!(
        ParametricIntegrand::from_family(&collision, &[4], Atom::one(), vec![x], eps, dimension),
        Err(Error::IntegralFamily(
            feynkit_graph::IntegralFamilyError::InvalidLabels
        ))
    ));
}

#[test]
fn native_dimension_in_physical_coefficients_requires_explicit_specialization() {
    let native_dimension = parse!("family_api::D");
    let (family, _) = tadpole(native_dimension.clone(), false);
    let parameters = vec![symbol!("family_api::x")];
    let eps = symbol!("family_api::eps");
    let changed = ParametricIntegrand::from_family(
        &family,
        &[4],
        Atom::one(),
        parameters.clone(),
        eps,
        parse!("2-2*family_api::eps"),
    );
    assert!(
        changed
            .unwrap_err()
            .to_string()
            .contains("specialize dimension-dependent")
    );
    // Retaining the same formal dimension is unambiguous, and an unrelated
    // physical mass symbol is not mistaken for the tensor dimension.
    assert!(
        ParametricIntegrand::from_family(
            &family,
            &[4],
            Atom::one(),
            parameters.clone(),
            eps,
            native_dimension,
        )
        .is_ok()
    );
    let (physical_mass, _) = tadpole(parse!("family_api::M"), false);
    let result = ParametricIntegrand::from_family(
        &physical_mass,
        &[4],
        Atom::one(),
        parameters,
        eps,
        parse!("2-2*family_api::eps"),
    )
    .unwrap();
    assert!(result.density().contains(parse!("family_api::M").as_view()));
    assert!(!result.density().contains(parse!("family_api::D").as_view()));
}
