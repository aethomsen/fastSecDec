use super::*;
use crate::{
    parametric::{
        FactorRole, FactorSemantics, ParametricDomain, ParametricIntegrand, ParametricTerm,
        PolynomialFactor,
    },
    threshold::gcad::{GcadKinematics, GcadRequest, SolverOptions},
};
use regular::callback::{Number, attempt};
use std::collections::BTreeMap;
use symbolica::{
    domains::float::{Complex, Float, RealLike},
    evaluate::OptimizationSettings,
    symbol,
};

fn fixture(
    shift: i64,
    polynomial_numerator: bool,
) -> (ParametricIntegrand, Symbol, Symbol, Symbol) {
    let (p, y, eps, u) = symbol!(
        "uniform_template::p",
        "uniform_template::y",
        "uniform_template::eps",
        "uniform_template::u"
    );
    let polynomial = Atom::var(y).pow(5) + Atom::var(y) - Atom::var(p) + Atom::num(shift);
    let mut factors = vec![
        PolynomialFactor::new(
            polynomial,
            -Atom::one() - Atom::var(eps),
            FactorRole::Singularity,
        )
        .with_semantics(FactorSemantics::Causal),
    ];
    if polynomial_numerator {
        factors.push(PolynomialFactor::new(
            Atom::one() + Atom::var(p) * Atom::var(y),
            Atom::one(),
            FactorRole::Polynomial,
        ));
    }
    let prefactor = if polynomial_numerator {
        Atom::one() + Atom::var(p)
    } else {
        Atom::one()
    };
    (
        ParametricIntegrand::new(
            vec![y],
            eps,
            ParametricDomain::UnitCube,
            vec![ParametricTerm::new(prefactor, vec![Atom::zero()], factors)],
        )
        .unwrap(),
        p,
        y,
        u,
    )
}
fn solve(
    input: &ParametricIntegrand,
    p: Symbol,
    shift: i64,
    value: Option<Rational>,
) -> Arc<VerifiedDecomposition> {
    let kinematics = if let Some(v) = value {
        GcadKinematics {
            exact_values: BTreeMap::from([(p, v)]),
            ..Default::default()
        }
    } else {
        GcadKinematics {
            runtime_parameters: vec![p],
            strict_positive: vec![
                Atom::var(p) - Atom::num(Rational::from(shift) + Rational::from((1, 4))),
                Atom::num(Rational::from(shift) + Rational::from((1, 2))) - Atom::var(p),
            ],
            ..Default::default()
        }
    };
    let count = 1 + kinematics.runtime_parameters.len();
    Arc::new(
        GcadRequest::unit_cube(
            input,
            kinematics,
            SolverOptions {
                order: (0..count).map(|i| format!("v{i}")).collect(),
                ..Default::default()
            },
            GcadRequest::default_limits(),
        )
        .unwrap()
        .solve_verified()
        .unwrap(),
    )
}
fn limits() -> Limits {
    Limits {
        unit_margin: Rational::from((1, 8)),
        ..Default::default()
    }
}
fn options() -> GenerationOptions {
    GenerationOptions {
        mode: crate::generation::GenerationMode::Symbolic,
        ..Default::default()
    }
}

#[test]
fn uniform_parameter_quintic_has_complete_source_and_shared_guard() {
    let (input, p, _, u) = fixture(2, false);
    let owner = solve(&input, p, 2, None);
    let family =
        NormalizedFamily::admit_parameter_affine_shifted(owner.clone(), vec![u], limits()).unwrap();
    let chamber = family.parameter_chamber().unwrap();
    assert!(Arc::ptr_eq(chamber.decomposition(), &owner));
    assert_eq!(chamber.cell_indices().collect::<Vec<_>>(), [0, 1]);
    assert_eq!(chamber.runtime_parameters(), [p]);
    assert_eq!(chamber.integration_axes(), [1]);
    assert_eq!(chamber.layout().map_inputs().collect::<Vec<_>>(), [p, u]);
    assert_eq!(chamber.layout().kernel_inputs().collect::<Vec<_>>(), [u, p]);
    for value in [Rational::from((19, 8)), Rational::from((37, 16))] {
        assert_eq!(
            chamber
                .guard()
                .admit(&BTreeMap::from([(p, value.clone())]))
                .unwrap(),
            [value]
        );
    }
    for value in [
        Rational::from((9, 4)),
        Rational::from((5, 2)),
        Rational::zero(),
    ] {
        assert!(
            chamber
                .guard()
                .admit(&BTreeMap::from([(p, value)]))
                .is_err()
        );
    }
    assert!(chamber.guard().admit(&BTreeMap::new()).is_err());
    assert!(NormalizedFamily::admit_affine_shifted(owner, vec![u], limits()).is_err());
    for chart in family.charts() {
        assert_eq!(chart.terms()[0].powers().len(), 1);
        let ChartGeometry::Single(g) = chart.geometry() else {
            panic!()
        };
        assert!(g.width().powers().is_empty());
        assert_eq!(g.section().axis(), 1);
    }
}

#[test]
fn uniform_parameter_exponents_and_nonpolynomial_prefactors_are_refused() {
    let (input, p, y, u) = fixture(0, false);
    let eps = input.regulator();
    for (prefactor, exponent) in [
        (Atom::one(), -Atom::var(p) - Atom::var(eps)),
        (Atom::var(p).pow(-1), -Atom::one() - Atom::var(eps)),
    ] {
        let input = ParametricIntegrand::new(
            vec![y],
            eps,
            ParametricDomain::UnitCube,
            vec![ParametricTerm::new(
                prefactor,
                vec![Atom::zero()],
                vec![
                    PolynomialFactor::new(
                        Atom::var(y).pow(5) + Atom::var(y) - Atom::var(p),
                        exponent,
                        FactorRole::Singularity,
                    )
                    .with_semantics(FactorSemantics::Causal),
                ],
            )],
        )
        .unwrap();
        let error = match NormalizedFamily::admit_parameter_affine_shifted(
            solve(&input, p, 0, None),
            vec![u],
            limits(),
        ) {
            Ok(_) => panic!("unsupported parameter dependence was admitted"),
            Err(error) => error,
        };
        assert!(matches!(error, Error::Unsupported(_)), "{error}");
    }
}

#[test]
fn uniform_parameter_full_laurent_body_matches_separate_fixed_fibers() {
    for (shift, numerator) in [(0, false), (2, true)] {
        let (input, p, _, u) = fixture(shift, numerator);
        let family = NormalizedFamily::admit_parameter_affine_shifted(
            solve(&input, p, shift, None),
            vec![u],
            limits(),
        )
        .unwrap();
        let continued = family.continue_symbolically(&options()).unwrap();
        let vector = continued.expand_vector(1).unwrap();
        assert!(
            vector
                .get(&-1)
                .is_none_or(|v| v.clone().into_inner().together().is_zero())
        );
        let expressions = vec![
            vector
                .get(&-1)
                .map_or_else(Atom::zero, |v| v.clone().into_inner()),
            vector
                .get(&0)
                .map_or_else(Atom::zero, |v| v.clone().into_inner()),
            vector
                .get(&1)
                .map_or_else(Atom::zero, |v| v.clone().into_inner()),
            family.root().clone(),
            family.root().derivative(p),
            family.root().derivative(p).derivative(p),
        ];
        let exact = Atom::evaluator_multiple(&expressions, &[Atom::var(u), Atom::var(p)])
            .function_map(continued.functions().clone())
            .optimization_settings(OptimizationSettings::default().cores(1))
            .build()
            .unwrap();
        let saved = bincode::serde::encode_to_vec(&exact, bincode::config::standard()).unwrap();
        for bits in [128, 256] {
            let map = |q: &Complex<Rational>| {
                Complex::new(
                    Float::rational_at(&q.re, bits),
                    Float::rational_at(&q.im, bits),
                )
            };
            let mut evaluator = family
                .callback_scope()
                .enter(bits, || exact.clone().map_coeff_with_prec(&map, bits));
            for offset in [
                Rational::from((3, 8)),
                Rational::from((5, 16)),
                Rational::from((3, 8)),
            ] {
                let value = Rational::from(shift) + &offset;
                family
                    .parameter_chamber()
                    .unwrap()
                    .guard()
                    .admit(&BTreeMap::from([(p, value.clone())]))
                    .unwrap();
                let fixed = NormalizedFamily::admit_affine_shifted(
                    solve(&input, p, shift, Some(value.clone())),
                    vec![u],
                    limits(),
                )
                .unwrap();
                let fixed_continued = fixed.continue_symbolically(&options()).unwrap();
                let reference = fixed_continued.expand_vector(1).unwrap();
                let fixed_exact = Atom::evaluator_multiple(
                    &[-1, 0, 1].map(|order| {
                        reference
                            .get(&order)
                            .map_or_else(Atom::zero, |v| v.clone().into_inner())
                    }),
                    &[Atom::var(u)],
                )
                .function_map(fixed_continued.functions().clone())
                .optimization_settings(OptimizationSettings::default().cores(1))
                .build()
                .unwrap();
                let mut fixed_evaluator = fixed
                    .callback_scope()
                    .enter(bits, || fixed_exact.map_coeff_with_prec(&map, bits));
                let section = &family.charts()[0].geometry().sections()[0];
                let algebra =
                    symgcad::algebra::Algebra::new(&family.owner().native_result().order).unwrap();
                let fiber = algebra
                    .specialize_univariate(section.polynomial(), 1, std::slice::from_ref(&value))
                    .unwrap();
                let mut isolated = symgcad::roots::isolate_union(vec![(0, fiber)]).unwrap();
                assert_eq!(isolated.len(), 1);
                for _ in 0..bits + 64 {
                    symgcad::roots::refine_once(&mut isolated[0]);
                }
                let exact_root =
                    (&isolated[0].interval.0 + &isolated[0].interval.1) / Rational::from(2);
                for unit in [Rational::from((1, 8)), Rational::from((7, 8))] {
                    let zero =
                        || Complex::new(Float::exact_zero_at(bits), Float::exact_zero_at(bits));
                    let at = |q: &Rational| {
                        Complex::new(Float::rational_at(q, bits), Float::exact_zero_at(bits))
                    };
                    let mut actual = vec![zero(); 6];
                    let mut expected = vec![zero(); 3];
                    attempt(|| evaluator.evaluate(&[at(&unit), at(&value)], &mut actual)).unwrap();
                    attempt(|| fixed_evaluator.evaluate(&[at(&unit)], &mut expected)).unwrap();
                    let tolerance = Rational::from((1, 2)).pow(u64::from(bits - 24));
                    for (a, b) in actual[..3].iter().zip(&expected) {
                        assert!((a.re.to_rational() - b.re.to_rational()).abs() < tolerance);
                        assert!((a.im.to_rational() - b.im.to_rational()).abs() < tolerance);
                    }
                    let r = actual[3].re.to_rational();
                    assert!(
                        r >= &isolated[0].interval.0 - &tolerance
                            && r <= &isolated[0].interval.1 + &tolerance
                    );
                    let py = Rational::one() + Rational::from(5) * r.pow(4);
                    assert!((actual[4].re.to_rational() - Rational::one() / &py).abs() < tolerance);
                    assert!(
                        (actual[5].re.to_rational() + Rational::from(20) * r.pow(3) / py.pow(3))
                            .abs()
                            < tolerance
                    );
                    let scale = if numerator {
                        (Rational::one() + &value) * (Rational::one() + &value * &exact_root)
                    } else {
                        Rational::one()
                    };
                    let oracle = std::f64::consts::PI
                        * (scale / (Rational::one() + Rational::from(5) * exact_root.pow(4)))
                            .to_f64();
                    assert!((actual[1].im.to_f64() - oracle).abs() < 1e-10);
                }
            }
        }
        assert_eq!(
            saved,
            bincode::serde::encode_to_vec(&exact, bincode::config::standard()).unwrap()
        );
    }
}

#[test]
fn uniform_width_factors_only_sampled_prefix_axes() {
    let (p, x, y, e, s, t) = symbol!(
        "uniform_width::p",
        "uniform_width::x",
        "uniform_width::y",
        "uniform_width::eps",
        "uniform_width::s",
        "uniform_width::t"
    );
    let input = ParametricIntegrand::new(
        vec![x, y],
        e,
        ParametricDomain::UnitCube,
        vec![ParametricTerm::new(
            Atom::one(),
            vec![Atom::zero(); 2],
            vec![
                PolynomialFactor::new(
                    Atom::var(y).pow(5) + Atom::var(y) - Atom::var(p) * Atom::var(x),
                    -Atom::one() - Atom::var(e),
                    FactorRole::Singularity,
                )
                .with_semantics(FactorSemantics::Causal),
            ],
        )],
    )
    .unwrap();
    let solve = |value: Option<Rational>| {
        let kin = if let Some(v) = value {
            GcadKinematics {
                exact_values: BTreeMap::from([(p, v)]),
                ..Default::default()
            }
        } else {
            GcadKinematics {
                runtime_parameters: vec![p],
                strict_positive: vec![
                    Atom::var(p) - Atom::num((1, 4)),
                    Atom::num((1, 2)) - Atom::var(p),
                ],
                ..Default::default()
            }
        };
        let count = 2 + kin.runtime_parameters.len();
        Arc::new(
            GcadRequest::unit_cube(
                &input,
                kin,
                SolverOptions {
                    order: (0..count).map(|i| format!("v{i}")).collect(),
                    ..Default::default()
                },
                GcadRequest::default_limits(),
            )
            .unwrap()
            .solve_verified()
            .unwrap(),
        )
    };
    let family =
        NormalizedFamily::admit_parameter_affine_shifted(solve(None), vec![s, t], limits())
            .unwrap();
    assert_eq!(
        family.parameter_chamber().unwrap().integration_axes(),
        [1, 2]
    );
    let ChartGeometry::Single(lower) = family.charts()[0].geometry() else {
        panic!()
    };
    assert!(lower.lower_cell());
    assert_eq!(lower.width().powers(), [1]);
    assert_eq!(
        lower.atom(lower.width().residual(), family.root()),
        Atom::var(p)
    );
    let continued = family.continue_symbolically(&options()).unwrap();
    let vector = continued.expand_vector(1).unwrap();
    let rows = [-1, 0, 1].map(|order| {
        vector
            .get(&order)
            .map_or_else(Atom::zero, |v| v.clone().into_inner())
    });
    let exact = Atom::evaluator_multiple(&rows, &[Atom::var(s), Atom::var(t), Atom::var(p)])
        .function_map(continued.functions().clone())
        .optimization_settings(OptimizationSettings::default().cores(1))
        .build()
        .unwrap();
    let bits = 128;
    let convert = |q: &Complex<Rational>| {
        Complex::new(
            Float::rational_at(&q.re, bits),
            Float::rational_at(&q.im, bits),
        )
    };
    let mut eval = family
        .callback_scope()
        .enter(bits, || exact.map_coeff_with_prec(&convert, bits));
    for value in [Rational::from((3, 8)), Rational::from((5, 16))] {
        let fixed = NormalizedFamily::admit_affine_shifted(
            solve(Some(value.clone())),
            vec![s, t],
            limits(),
        )
        .unwrap();
        let fc = fixed.continue_symbolically(&options()).unwrap();
        let fv = fc.expand_vector(1).unwrap();
        let fx = Atom::evaluator_multiple(
            &[-1, 0, 1].map(|o| {
                fv.get(&o)
                    .map_or_else(Atom::zero, |v| v.clone().into_inner())
            }),
            &[Atom::var(s), Atom::var(t)],
        )
        .function_map(fc.functions().clone())
        .optimization_settings(OptimizationSettings::default().cores(1))
        .build()
        .unwrap();
        let mut fe = fixed
            .callback_scope()
            .enter(bits, || fx.map_coeff_with_prec(&convert, bits));
        for sx in [Rational::from((1, 8)), Rational::from((7, 8))] {
            let at = |q: &Rational| {
                Complex::new(Float::rational_at(q, bits), Float::exact_zero_at(bits))
            };
            let zero = Complex::new(Float::exact_zero_at(bits), Float::exact_zero_at(bits));
            let mut a = vec![zero.clone(); 3];
            let mut b = vec![zero; 3];
            let ty = Rational::from((3, 8));
            attempt(|| eval.evaluate(&[at(&sx), at(&ty), at(&value)], &mut a)).unwrap();
            attempt(|| fe.evaluate(&[at(&sx), at(&ty)], &mut b)).unwrap();
            let tol = Rational::from((1, 2)).pow(96);
            for (a, b) in a.iter().zip(&b) {
                assert!((a.re.to_rational() - b.re.to_rational()).abs() < tol);
                assert!((a.im.to_rational() - b.im.to_rational()).abs() < tol);
            }
        }
    }
}
