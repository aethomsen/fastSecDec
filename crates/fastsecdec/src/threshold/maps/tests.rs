use super::super::gcad::{GcadKinematics, GcadRequest, PreparedDomain, SolverOptions};
use super::*;
use crate::parametric::{
    FactorRole, FactorSemantics, ParametricDomain, ParametricIntegrand, ParametricTerm,
    PolynomialFactor,
};
use std::collections::BTreeMap;
use symbolica::{
    atom::{Atom, AtomCore},
    domains::float::Complex,
    evaluate::{ExpressionEvaluator, OptimizationSettings},
    prelude::Rational,
    symbol,
};

fn source(
    parameters: Vec<Symbol>,
    polynomial: Atom,
    domain: ParametricDomain,
) -> ParametricIntegrand {
    ParametricIntegrand::new(
        parameters.clone(),
        symbol!("linear_cell_test::eps"),
        domain,
        vec![ParametricTerm::new(
            Atom::one() + Atom::i(),
            vec![Atom::zero(); parameters.len()],
            vec![
                PolynomialFactor::new(polynomial, Atom::num(-1), FactorRole::Singularity)
                    .with_semantics(FactorSemantics::Causal),
            ],
        )],
    )
    .unwrap()
}
fn solve(input: &ParametricIntegrand, kinematics: GcadKinematics) -> Arc<VerifiedDecomposition> {
    Arc::new(
        GcadRequest::unit_cube(
            input,
            kinematics,
            SolverOptions::default(),
            GcadRequest::default_limits(),
        )
        .unwrap()
        .solve_verified()
        .unwrap(),
    )
}
fn settings() -> OptimizationSettings {
    OptimizationSettings::default()
        .cpe_iterations(Some(0))
        .cores(1)
}
fn assert_program(admission: &ParameterAdmission, unit: &[Rational]) {
    let expected = admission.map_rational(unit).unwrap();
    let program = admission.compile(settings()).unwrap();
    assert_eq!(
        program.program().get_input_len(),
        unit.len() + program.parameter_count()
    );
    let bytes = bincode::encode_to_vec(program.program(), bincode::config::standard()).unwrap();
    let (restored, used): (ExpressionEvaluator<Complex<Rational>>, usize) =
        bincode::decode_from_slice(&bytes, bincode::config::standard()).unwrap();
    assert_eq!(used, bytes.len());
    let mut evaluator = restored.map_coeff(&|v| Complex::new(v.re.to_f64(), v.im.to_f64()));
    let point = program
        .admitted_parameter_values()
        .chain(unit.iter())
        .map(|v| Complex::new(v.to_f64(), 0.))
        .collect::<Vec<_>>();
    let mut actual = vec![Complex::new(0., 0.); program.measure_output() + 1];
    evaluator.evaluate(&point, &mut actual);
    let expected = expected
        .prepared_coordinates
        .into_iter()
        .chain(expected.original_coordinates)
        .chain([expected.measure])
        .collect::<Vec<_>>();
    for (a, b) in actual.iter().zip(&expected) {
        assert!((a.re - b.to_f64()).abs() < 1e-13);
        assert_eq!(a.im, 0.);
    }
}

#[test]
fn finite_linear_cells_compose_in_original_order_and_retain_owners() {
    let (x, y, t, u) = symbol!(
        "linear_cell_test::x",
        "linear_cell_test::y",
        "linear_cell_test::t",
        "linear_cell_test::u"
    );
    let input = source(
        vec![x, y],
        Atom::var(x) - Atom::var(y),
        ParametricDomain::UnitCube,
    );
    let owner = solve(&input, GcadKinematics::default());
    assert_eq!(owner.cells().len(), 2);
    let maps = (0..owner.cells().len())
        .map(|index| CellMap::new(owner.clone(), index, vec![t, u]).unwrap())
        .collect::<Vec<_>>();
    assert!(Arc::ptr_eq(
        maps[0].decomposition(),
        maps[1].decomposition()
    ));
    drop(owner);
    drop(input);
    for map in maps {
        let admission = map
            .linear()
            .unwrap()
            .admit_parameters(&BTreeMap::new())
            .unwrap();
        for a in [Rational::from((1, 4)), Rational::from((3, 4))] {
            for b in [Rational::from((1, 3)), Rational::from((2, 3))] {
                let point = admission.map_rational(&[a.clone(), b.clone()]).unwrap();
                let difference = &point.prepared_coordinates[0] - &point.prepared_coordinates[1];
                let expected_sign = map.owner.native_result().cells[map.cell_index].signs[0];
                assert_eq!(
                    if difference > Rational::zero() { 1 } else { -1 },
                    expected_sign
                );
                assert!(point.measure > Rational::zero());
                assert_program(&admission, &[a.clone(), b]);
            }
        }
    }
}

#[test]
fn parameter_chambers_are_exact_guards_not_unit_dimensions() {
    let (a, x, t) = symbol!(
        "linear_cell_test::a",
        "linear_cell_test::x",
        "linear_cell_test::t"
    );
    let input = source(
        vec![x],
        Atom::var(x) - Atom::var(a),
        ParametricDomain::UnitCube,
    );
    let owner = solve(
        &input,
        GcadKinematics {
            runtime_parameters: vec![a],
            strict_positive: vec![Atom::var(a), Atom::one() - Atom::var(a)],
            ..GcadKinematics::default()
        },
    );
    for index in 0..owner.cells().len() {
        let map = CellMap::new(owner.clone(), index, vec![t])
            .unwrap()
            .linear()
            .unwrap();
        assert_eq!(map.source.unit_coordinates.len(), 1);
        assert_eq!(map.source.axes[0].role, AliasRole::RuntimeParameter);
        for value in [Rational::from((1, 4)), Rational::from((3, 4))] {
            let admitted = map.admit_parameters(&BTreeMap::from([(a, value)])).unwrap();
            assert_program(&admitted, &[Rational::from((1, 3))]);
        }
        for bad in [Rational::zero(), Rational::one(), Rational::from(-1)] {
            assert!(map.admit_parameters(&BTreeMap::from([(a, bad)])).is_err());
        }
        assert!(map.admit_parameters(&BTreeMap::new()).is_err());
        let first = map
            .admit_parameters(&BTreeMap::from([(a, Rational::from((1, 4)))]))
            .unwrap()
            .compile(settings())
            .unwrap();
        let rebound = first
            .rebind(&BTreeMap::from([(a, Rational::from((3, 4)))]))
            .unwrap();
        assert!(std::ptr::eq(first.program(), rebound.program()));
        assert_eq!(rebound.inputs(), &[a, t]);
        assert!(
            first
                .rebind(&BTreeMap::from([(a, Rational::one())]))
                .is_err()
        );
        let unit = Rational::from((1, 3));
        let expected = rebound
            .admission()
            .map_rational(std::slice::from_ref(&unit))
            .unwrap();
        let mut evaluator = rebound
            .program()
            .clone()
            .map_coeff(&|v| Complex::new(v.re.to_f64(), v.im.to_f64()));
        let point = rebound
            .admitted_parameter_values()
            .chain(std::iter::once(&unit))
            .map(|v| Complex::new(v.to_f64(), 0.))
            .collect::<Vec<_>>();
        let mut actual = vec![Complex::new(0., 0.); rebound.measure_output() + 1];
        evaluator.evaluate(&point, &mut actual);
        assert!((actual[0].re - expected.prepared_coordinates[0].to_f64()).abs() < 1e-13);
        assert!((actual[rebound.measure_output()].re - expected.measure.to_f64()).abs() < 1e-13);
    }
}

#[test]
fn nonlinear_parameter_prefix_retains_complete_source_and_roles() {
    let (p, y, u) = symbol!(
        "parameter_prefix_test::p",
        "parameter_prefix_test::y",
        "parameter_prefix_test::u"
    );
    // The parameter lies outside the unit interval; only y is integrated.
    let input = source(
        vec![y],
        Atom::var(y).pow(5) + Atom::var(y) - Atom::var(p) + Atom::num(2),
        ParametricDomain::UnitCube,
    );
    let kinematics = GcadKinematics {
        runtime_parameters: vec![p],
        strict_positive: vec![
            Atom::var(p) - Atom::num((9, 4)),
            Atom::num((5, 2)) - Atom::var(p),
        ],
        ..GcadKinematics::default()
    };
    let owner = solve(&input, kinematics.clone());
    assert_eq!(owner.cells().len(), 2);
    let maps = (0..owner.cells().len())
        .map(|i| CellMap::new(owner.clone(), i, vec![u]).unwrap())
        .collect::<Vec<_>>();
    for map in &maps {
        let prefix = map.parameter_prefix().unwrap();
        assert!(Arc::ptr_eq(prefix.source(), map));
        assert_eq!(prefix.runtime_parameters(), &[p]);
        assert_eq!(prefix.raw_parameters().collect::<Vec<_>>(), [p]);
        assert_eq!(prefix.axes().len(), 1);
        assert_eq!(
            prefix
                .admit(&BTreeMap::from([(p, Rational::from((19, 8)))]))
                .unwrap()
                .raw_values(),
            &[Rational::from((19, 8))]
        );
        assert!(
            prefix
                .admit(&BTreeMap::from([(p, Rational::from((9, 4)))]))
                .is_err()
        );
        assert!(prefix.admit(&BTreeMap::new()).is_err());
        assert_eq!(prefix.axes()[0].lower(), Some(&Atom::num((9, 4))));
        assert_eq!(prefix.axes()[0].upper(), Some(&Atom::num((5, 2))));
        assert!(matches!(map.linear(), Err(GcadError::Unsupported(_))));
    }
    let chamber = ParameterChamber::prepare(maps.iter().rev().cloned().collect()).unwrap();
    assert!(Arc::ptr_eq(chamber.decomposition(), &owner));
    assert_eq!(chamber.prefixes().len(), 2);
    assert!(Arc::ptr_eq(
        chamber.prefixes()[0].guard().unwrap(),
        chamber.prefixes()[1].guard().unwrap()
    ));
    assert_eq!(chamber.prefixes()[0].source().cell_index(), 0);
    assert_eq!(chamber.prefixes()[1].source().cell_index(), 1);
    let layout = chamber.layout();
    assert_eq!(layout.raw_to_runtime(), &[Some(0), None]);
    assert_eq!(layout.raw_to_unit(), &[None, Some(0)]);
    assert_eq!(layout.map_inputs().collect::<Vec<_>>(), [p, u]);
    assert_eq!(layout.kernel_inputs().collect::<Vec<_>>(), [u, p]);
    assert_eq!(layout.map_from_kernel().collect::<Vec<_>>(), [1, 0]);
    assert!(matches!(
        ParameterChamber::prepare(vec![maps[0].clone()]),
        Err(GcadError::Invalid(_))
    ));
    assert!(matches!(
        ParameterChamber::prepare(vec![maps[0].clone(), maps[0].clone()]),
        Err(GcadError::Invalid(_))
    ));
    let foreign = solve(&input, kinematics);
    assert!(matches!(
        ParameterChamber::prepare(vec![
            maps[0].clone(),
            CellMap::new(foreign, 1, vec![u]).unwrap()
        ]),
        Err(GcadError::Invalid(_))
    ));
}

#[test]
fn exact_nonlinear_and_unbounded_cells_remain_inspectable_but_do_not_lower() {
    let (x, t) = symbol!("linear_cell_test::x", "linear_cell_test::t");
    let input = source(
        vec![x],
        Atom::var(x).pow(2) - Atom::num((1, 2)),
        ParametricDomain::UnitCube,
    );
    let owner = solve(&input, GcadKinematics::default());
    let map = CellMap::new(owner.clone(), 0, vec![t]).unwrap();
    assert!(map.native_axes()[0].upper.is_some());
    assert!(matches!(map.linear(), Err(GcadError::Unsupported(_))));
    assert!(CellMap::new(owner, 0, vec![x]).is_err());
    let input = source(
        vec![x],
        Atom::var(x) - Atom::one(),
        ParametricDomain::PositiveOrthant,
    );
    let request = GcadRequest::prepared(
        &input,
        PreparedDomain::explicit(
            vec![x],
            vec![Atom::var(x)],
            "test positive source domain".into(),
        )
        .unwrap(),
        GcadKinematics::default(),
        SolverOptions::default(),
        GcadRequest::default_limits(),
    )
    .unwrap();
    let owner = Arc::new(request.solve_verified().unwrap());
    let last = owner.cells().len() - 1;
    let map = CellMap::new(owner, last, vec![t]).unwrap();
    assert!(map.native_axes()[0].upper.is_none());
    assert!(matches!(map.linear(), Err(GcadError::Unsupported(_))));
}

#[test]
fn projective_source_images_and_delta_measure_compose_once() {
    let (x, y, z, t, u) = symbol!(
        "linear_cell_test::x",
        "linear_cell_test::y",
        "linear_cell_test::z",
        "linear_cell_test::t",
        "linear_cell_test::u"
    );
    let input = ParametricIntegrand::new(
        vec![x, y, z],
        symbol!("linear_cell_test::eps"),
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
    .unwrap();
    let preparation =
        super::super::projective::AffineProjectivePreparation::last_coordinate(&input).unwrap();
    let owner = Arc::new(
        GcadRequest::projective(
            preparation,
            GcadKinematics::default(),
            SolverOptions::default(),
            GcadRequest::default_limits(),
        )
        .unwrap()
        .solve_verified()
        .unwrap(),
    );
    assert_eq!(owner.cells().len(), 1);
    let map = CellMap::new(owner, 0, vec![t, u])
        .unwrap()
        .linear()
        .unwrap();
    let admission = map.admit_parameters(&BTreeMap::new()).unwrap();
    let unit = [Rational::from((1, 3)), Rational::from((1, 2))];
    let point = admission.map_rational(&unit).unwrap();
    assert_eq!(
        point.original_coordinates.iter().sum::<Rational>(),
        Rational::one()
    );
    assert_eq!(point.measure, Rational::from((2, 3)));
    let program = admission.compile(settings()).unwrap();
    assert_eq!(program.prepared_dimension(), 2);
    assert_eq!(program.original_dimension(), 3);
    assert_program(&admission, &unit);
}

#[test]
fn native_full_jacobian_and_reversed_lifting_order_match_width_measure() {
    use symbolica::{
        domains::atom::AtomField,
        id::{Pattern, Replacement},
        tensors::matrix::Matrix,
    };
    let (x, y, t, u) = symbol!(
        "linear_cell_test::x",
        "linear_cell_test::y",
        "linear_cell_test::t",
        "linear_cell_test::u"
    );
    let input = source(
        vec![x, y],
        Atom::var(x) - Atom::var(y),
        ParametricDomain::UnitCube,
    );
    let options = SolverOptions {
        order: vec!["v1".into(), "v0".into()],
        ..SolverOptions::default()
    };
    let owner = Arc::new(
        GcadRequest::unit_cube(
            &input,
            GcadKinematics::default(),
            options,
            GcadRequest::default_limits(),
        )
        .unwrap()
        .solve_verified()
        .unwrap(),
    );
    assert_eq!(owner.native_result().order, vec!["v1", "v0"]);
    for index in 0..owner.cells().len() {
        let map = CellMap::new(owner.clone(), index, vec![t, u])
            .unwrap()
            .linear()
            .unwrap();
        let mut images = BTreeMap::<Symbol, Atom>::new();
        let mut jacobian = Atom::one();
        for (axis, bounds) in map.source.axes.iter().zip(&map.axes) {
            let replace = |expression: &Atom| {
                expression.replace_multiple(images.iter().map(|(symbol, image)| {
                    Replacement::new(
                        Pattern::Literal(Atom::var(*symbol)),
                        Pattern::Literal(image.clone()),
                    )
                }))
            };
            let lower = replace(&bounds.lower.as_ref().unwrap().expression);
            let upper = replace(&bounds.upper.as_ref().unwrap().expression);
            let width = upper - &lower;
            let image =
                lower + &width * Atom::var(map.source.unit_coordinates[axis.unit_index.unwrap()]);
            jacobian *= width;
            images.insert(axis.symbol, image);
        }
        let full = [images[&x].clone(), images[&y].clone()];
        let entries = full
            .iter()
            .flat_map(|image| {
                [t, u]
                    .iter()
                    .map(|axis| image.derivative(*axis))
                    .collect::<Vec<_>>()
            })
            .collect();
        let determinant = Matrix::from_linear(
            entries,
            2,
            2,
            AtomField {
                statistical_zero_test: false,
                ..AtomField::new()
            },
        )
        .unwrap()
        .det()
        .unwrap();
        assert!(
            (determinant.pow(2) - jacobian.pow(2))
                .together()
                .expand()
                .is_zero()
        );
        let admission = map.admit_parameters(&BTreeMap::new()).unwrap();
        assert_program(
            &admission,
            &[Rational::from((1, 3)), Rational::from((2, 3))],
        );
        assert!(
            admission
                .map_rational(&[Rational::zero(), Rational::from((1, 2))])
                .is_err()
        );
    }
}

#[test]
fn actual_linear_selector_domain_and_degree_drop_checks_are_exact() {
    use symgcad::{algebra::Algebra, output::RootIndexDomain};
    let algebra = Algebra::new(&["p".into(), "x".into()]).unwrap();
    let zero = algebra.parse("0").unwrap();
    let one = algebra.parse("1").unwrap();
    for selector in [
        RootIndexDomain::Real,
        RootIndexDomain::RealDescending,
        RootIndexDomain::Positive,
    ] {
        let root = super::linear::LinearRoot {
            polynomial: 0,
            selector,
            numerator: zero.clone(),
            denominator: one.clone(),
            expression: Atom::zero(),
        };
        let value = root.evaluate(&[Rational::from(1), Rational::zero()]);
        if selector == RootIndexDomain::Positive {
            assert!(value.is_err());
        } else {
            assert_eq!(value.unwrap(), Rational::zero());
        }
    }
    let root = super::linear::LinearRoot {
        polynomial: 1,
        selector: RootIndexDomain::Real,
        numerator: one,
        denominator: algebra.parse("p").unwrap(),
        expression: Atom::one() / Atom::var(symbol!("linear_cell_test::p")),
    };
    assert!(
        root.evaluate(&[Rational::zero(), Rational::zero()])
            .is_err()
    );
    assert_eq!(
        root.evaluate(&[Rational::from(2), Rational::zero()])
            .unwrap(),
        Rational::from((1, 2))
    );
}

#[test]
fn unit_symbols_cannot_capture_auxiliary_or_numerator_only_symbols() {
    let (x, eps, eta, n, rho, t) = symbol!(
        "linear_cell_test::x",
        "linear_cell_test::eps",
        "linear_cell_test::eta",
        "linear_cell_test::numerator_parameter",
        "linear_cell_test::prefactor_parameter",
        "linear_cell_test::t"
    );
    let input = ParametricIntegrand::new(
        vec![x],
        eps,
        ParametricDomain::UnitCube,
        vec![ParametricTerm::new(
            Atom::var(rho),
            vec![Atom::var(eps)],
            vec![
                PolynomialFactor::new(
                    Atom::var(x) - Atom::num((1, 2)),
                    Atom::num(-1) + Atom::var(eta),
                    FactorRole::Singularity,
                )
                .with_semantics(FactorSemantics::Causal),
                PolynomialFactor::new(
                    Atom::var(n) + Atom::var(x),
                    Atom::num(2),
                    FactorRole::Polynomial,
                ),
            ],
        )],
    )
    .unwrap();
    let owner = solve(&input, GcadKinematics::default());
    assert!(
        owner
            .request()
            .aliases()
            .iter()
            .all(|a| ![eta, n, rho].contains(&a.symbol))
    );
    for collision in [x, eps, eta, n, rho] {
        assert!(CellMap::new(owner.clone(), 0, vec![collision]).is_err());
    }
    assert!(CellMap::new(owner, 0, vec![t]).is_ok());
}
