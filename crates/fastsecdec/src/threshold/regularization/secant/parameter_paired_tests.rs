use super::*;
use crate::{
    input::GraphIntegral,
    parametric::{
        FactorRole, FactorSemantics, ParametricDomain, ParametricIntegrand, ParametricTerm,
        PolynomialFactor,
    },
    threshold::{
        gcad::{GcadKinematics, GcadRequest, SolverOptions},
        projective::AffineProjectivePreparation,
    },
};
use regular::{
    BracketProposal,
    callback::{Number, attempt},
};
use std::collections::BTreeMap;
use symbolica::{
    domains::float::{Complex, Float, RealLike},
    evaluate::OptimizationSettings,
    symbol,
};

fn graph() -> (AffineProjectivePreparation, Symbol, Symbol) {
    let (s, x, y, e, u) = symbol!(
        "uniform_bubble::s",
        "uniform_bubble::x",
        "uniform_bubble::y",
        "uniform_bubble::eps",
        "uniform_bubble::u"
    );
    let model = Arc::new(
        crate::Model::from_json(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/models/scalar.json"
        )))
        .unwrap(),
    );
    let kin = crate::Kinematics::in_dimension(&symbolica::parse!("D"))
        .unwrap()
        .with_mass_squared(
            &feynkit_graph::symbols::external_momentum().call(1),
            Atom::var(s),
        )
        .unwrap();
    let graph = GraphIntegral::from_dot(
        model,
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/graphs/bubble.dot"
        )),
        &kin,
    )
    .unwrap()
    .with_scalar_values(&BTreeMap::from([(
        symbol!("UFO::mt"),
        Atom::num(3).pow(Atom::num((1, 2))),
    )]))
    .unwrap();
    let input = ParametricIntegrand::from_graph(
        &graph,
        vec![x, y],
        e,
        Atom::num(4) - Atom::num(2) * Atom::var(e),
    )
    .unwrap();
    let preparation = AffineProjectivePreparation::last_coordinate(&input).unwrap();
    assert_eq!(
        preparation.images(),
        [Atom::var(x), Atom::one() - Atom::var(x)]
    );
    assert_eq!(preparation.measure_jacobian(), &Atom::one());
    let f = preparation.terms()[0]
        .factors()
        .iter()
        .find(|f| f.semantics() == FactorSemantics::Causal)
        .unwrap();
    assert!(
        (f.polynomial()
            - (Atom::num(3) - Atom::var(s) * Atom::var(x) * (Atom::one() - Atom::var(x))))
        .expand()
        .is_zero()
    );
    (preparation, s, u)
}
fn proposals() -> [BracketProposal; 2] {
    [
        BracketProposal {
            lower: Rational::from((1, 8)),
            upper: Rational::from((3, 8)),
            derivative_margin: Rational::one(),
        },
        BracketProposal {
            lower: Rational::from((5, 8)),
            upper: Rational::from((7, 8)),
            derivative_margin: Rational::one(),
        },
    ]
}
fn limits() -> Limits {
    Limits {
        unit_margin: Rational::from((1, 8)),
        ..Default::default()
    }
}
fn owner(
    preparation: &AffineProjectivePreparation,
    s: Symbol,
    lower: i64,
) -> Arc<VerifiedDecomposition> {
    Arc::new(
        GcadRequest::projective(
            Arc::new(preparation.clone()),
            GcadKinematics {
                runtime_parameters: vec![s],
                strict_positive: vec![
                    Atom::var(s) - Atom::num(lower),
                    Atom::num(18) - Atom::var(s),
                ],
                ..Default::default()
            },
            SolverOptions {
                order: vec!["v0".into(), "v1".into()],
                ..Default::default()
            },
            GcadRequest::default_limits(),
        )
        .unwrap()
        .solve_verified()
        .unwrap(),
    )
}
fn family(preparation: &AffineProjectivePreparation, s: Symbol, u: Symbol) -> NormalizedFamily {
    NormalizedFamily::admit_parameter_pair_affine_shifted(
        owner(preparation, s, 15),
        vec![u],
        proposals(),
        limits(),
    )
    .unwrap()
}
fn options() -> GenerationOptions {
    GenerationOptions {
        mode: crate::generation::GenerationMode::Symbolic,
        ..Default::default()
    }
}

#[test]
fn actual_projective_bubble_keeps_complete_uniform_pair_and_original_measure() {
    let (preparation, s, u) = graph();
    let source = owner(&preparation, s, 15);
    let family = NormalizedFamily::admit_parameter_pair_affine_shifted(
        source.clone(),
        vec![u],
        proposals(),
        limits(),
    )
    .unwrap();
    assert_eq!(
        family.owner().request().projective_preparation(),
        Some(&preparation)
    );
    assert_eq!(family.owner().request().input(), preparation.input());
    assert_eq!(family.charts().len(), 4);
    assert_eq!(family.roots().len(), 2);
    let chamber = family.parameter_chamber().unwrap();
    assert!(Arc::ptr_eq(chamber.decomposition(), &source));
    assert_eq!(chamber.cell_indices().collect::<Vec<_>>(), [0, 1, 2]);
    assert_eq!(chamber.integration_axes(), [1]);
    assert_eq!(chamber.layout().kernel_inputs().collect::<Vec<_>>(), [u, s]);
    for value in [16, 17] {
        chamber
            .guard()
            .admit(&BTreeMap::from([(s, Rational::from(value))]))
            .unwrap();
    }
    for value in [0, 12, 15, 18] {
        assert!(
            chamber
                .guard()
                .admit(&BTreeMap::from([(s, Rational::from(value))]))
                .is_err()
        );
    }
    for chart in family.charts() {
        let ChartGeometry::Paired(pair) = chart.geometry() else {
            panic!()
        };
        assert_eq!(pair.pair().sections()[0].axis(), 1);
        assert_eq!(pair.pair().sections()[1].axis(), 1);
        assert_ne!(
            pair.pair().sections()[0].bound(),
            pair.pair().sections()[1].bound()
        );
        assert_eq!(
            chart.terms()[0].powers(),
            [-Atom::var(preparation.input().regulator())]
        );
        let (_, _, magnitude) = pair.normalized(family.roots());
        assert!(magnitude.contains_symbol(s)); // Keep the positive second secant, not a monicized source.
    }
    let continued = family.continue_symbolically(&options()).unwrap();
    assert!(
        continued
            .face_reductions()
            .iter()
            .all(|r| r.quotient.is_zero())
    );
    assert!(
        NormalizedFamily::admit_pair_affine_shifted(source, vec![u], proposals(), limits())
            .is_err()
    );
    assert!(
        NormalizedFamily::admit_parameter_pair_affine_shifted(
            owner(&preparation, s, 12),
            vec![u],
            proposals(),
            limits()
        )
        .is_err()
    );
}

// This reference uses the exact already-proved one-axis gauge density only in a
// test. The delivered family above retains the original projective owner.
fn fixed_reference(
    preparation: &AffineProjectivePreparation,
    s: Symbol,
    u: Symbol,
    value: &Rational,
) -> NormalizedFamily {
    let input = ParametricIntegrand::new(
        preparation.coordinates().to_vec(),
        preparation.input().regulator(),
        ParametricDomain::UnitCube,
        preparation.terms().to_vec(),
    )
    .unwrap();
    let source = Arc::new(
        GcadRequest::unit_cube(
            &input,
            GcadKinematics {
                exact_values: BTreeMap::from([(s, value.clone())]),
                ..Default::default()
            },
            SolverOptions::default(),
            GcadRequest::default_limits(),
        )
        .unwrap()
        .solve_verified()
        .unwrap(),
    );
    NormalizedFamily::admit_pair_affine_shifted(source, vec![u], proposals(), limits()).unwrap()
}

// Test-only independent substitution into the original physical F. The fixed
// native decomposition at s=16 has linear bounds, not an algebraic paired owner.
fn rational_reference(preparation: &AffineProjectivePreparation, u: Symbol) -> Vec<Atom> {
    let e = Atom::var(preparation.input().regulator());
    let t = Atom::var(u);
    let f = |x: &Atom| (Atom::num(3) - Atom::num(16) * x * (Atom::one() - x)).expand();
    for root in [Atom::num((1, 4)), Atom::num((3, 4))] {
        assert!(f(&root).is_zero());
    }
    let mut expression = Atom::zero();
    for (index, image) in [
        (Atom::one() - &t) / Atom::num(4),
        (Atom::one() + &t) / Atom::num(4),
        (Atom::num(3) - &t) / Atom::num(4),
        (Atom::num(3) + &t) / Atom::num(4),
    ]
    .into_iter()
    .enumerate()
    {
        let negative = index == 1 || index == 2;
        let magnitude = if negative { -f(&image) } else { f(&image) };
        let phase = if negative {
            (Atom::i() * Atom::var(Symbol::PI) * &e).exp()
        } else {
            Atom::one()
        };
        expression +=
            preparation.terms()[0].prefactor() * magnitude.pow(-&e) * phase / Atom::num(4);
    }
    let vector = crate::generation::threshold_expand_vector(
        &expression,
        &[u],
        preparation.input().regulator(),
        1,
    )
    .unwrap();
    [-1, 0, 1]
        .map(|order| {
            vector
                .get(&order)
                .map_or_else(Atom::zero, |v| v.clone().into_inner())
        })
        .to_vec()
}

#[test]
fn one_uniform_bubble_program_refreshes_full_vector_at_two_physical_points() {
    let (preparation, s, u) = graph();
    let family = family(&preparation, s, u);
    let continued = family.continue_symbolically(&options()).unwrap();
    let vector = continued.expand_vector(1).unwrap();
    let mut expressions = [-1, 0, 1]
        .map(|order| {
            vector
                .get(&order)
                .map_or_else(Atom::zero, |v| v.clone().into_inner())
        })
        .to_vec();
    expressions.extend(family.roots().iter().cloned());
    let exact = Atom::evaluator_multiple(&expressions, &[Atom::var(u), Atom::var(s)])
        .function_map(continued.functions().clone())
        .optimization_settings(OptimizationSettings::default().cores(1))
        .build()
        .unwrap();
    let saved = bincode::serde::encode_to_vec(&exact, bincode::config::standard()).unwrap();
    for bits in [128, 256] {
        let convert = |q: &Complex<Rational>| {
            Complex::new(
                Float::rational_at(&q.re, bits),
                Float::rational_at(&q.im, bits),
            )
        };
        let mut evaluator = family
            .callback_scope()
            .enter(bits, || exact.clone().map_coeff_with_prec(&convert, bits));
        for value in [Rational::from(16), Rational::from(17), Rational::from(16)] {
            family
                .parameter_chamber()
                .unwrap()
                .guard()
                .admit(&BTreeMap::from([(s, value.clone())]))
                .unwrap();
            let fixed = (value != 16).then(|| fixed_reference(&preparation, s, u, &value));
            let (reference_expressions, functions) = if let Some(fixed) = &fixed {
                let fc = fixed.continue_symbolically(&options()).unwrap();
                let fv = fc.expand_vector(1).unwrap();
                (
                    [-1, 0, 1]
                        .map(|o| {
                            fv.get(&o)
                                .map_or_else(Atom::zero, |v| v.clone().into_inner())
                        })
                        .to_vec(),
                    fc.functions().clone(),
                )
            } else {
                (rational_reference(&preparation, u), FunctionMap::new())
            };
            let fx = Atom::evaluator_multiple(&reference_expressions, &[Atom::var(u)])
                .function_map(functions)
                .optimization_settings(OptimizationSettings::default().cores(1))
                .build()
                .unwrap();
            let mut reference = if let Some(fixed) = &fixed {
                fixed
                    .callback_scope()
                    .enter(bits, || fx.map_coeff_with_prec(&convert, bits))
            } else {
                fx.map_coeff_with_prec(&convert, bits)
            };
            let section = &family.charts()[0].geometry().sections()[0];
            let algebra =
                symgcad::algebra::Algebra::new(&family.owner().native_result().order).unwrap();
            let fiber = algebra
                .specialize_univariate(section.polynomial(), 1, std::slice::from_ref(&value))
                .unwrap();
            let mut roots = symgcad::roots::isolate_union(vec![(0, fiber)]).unwrap();
            assert_eq!(roots.len(), 2);
            for r in &mut roots {
                for _ in 0..bits + 64 {
                    symgcad::roots::refine_once(r);
                }
            }
            for unit in [Rational::from((1, 8)), Rational::from((7, 8))] {
                let zero = || Complex::new(Float::exact_zero_at(bits), Float::exact_zero_at(bits));
                let at = |q: &Rational| {
                    Complex::new(Float::rational_at(q, bits), Float::exact_zero_at(bits))
                };
                let mut actual = vec![zero(); 5];
                let mut expected = vec![zero(); 3];
                attempt(|| evaluator.evaluate(&[at(&unit), at(&value)], &mut actual)).unwrap();
                attempt(|| reference.evaluate(&[at(&unit)], &mut expected)).unwrap();
                let tolerance = Rational::from((1, 2)).pow(u64::from(bits - 24));
                for (a, b) in actual[..3].iter().zip(&expected) {
                    assert!((a.re.to_rational() - b.re.to_rational()).abs() < tolerance);
                    assert!((a.im.to_rational() - b.im.to_rational()).abs() < tolerance);
                }
                assert!((actual[0].re.to_rational() - Rational::one()).abs() < tolerance);
                assert!(actual[0].im.to_rational().abs() < tolerance);
                for (a, r) in actual[3..].iter().zip(&roots) {
                    let v = a.re.to_rational();
                    assert!(v >= &r.interval.0 - &tolerance && v <= &r.interval.1 + &tolerance);
                }
                let beta = (1.0 - 12.0 / value.to_f64()).sqrt();
                assert!((actual[1].im.to_f64() - std::f64::consts::PI * beta).abs() < 1e-11);
            }
        }
    }
    assert_eq!(
        saved,
        bincode::serde::encode_to_vec(&exact, bincode::config::standard()).unwrap()
    );
}

#[test]
fn parameter_pair_keeps_higher_degree_face_division_and_optional_zero_refused() {
    let (preparation, s, u) = graph();
    let x = preparation.coordinates()[0];
    let e = preparation.input().regulator();
    // Force an endpoint finite-face call with a quadratic numerator. Its
    // nonconstant leading unit still requires a separate proof producer.
    let mut factors = preparation.terms()[0].factors().to_vec();
    factors.push(PolynomialFactor::new(
        Atom::var(x).pow(2),
        Atom::one(),
        FactorRole::Polynomial,
    ));
    for f in &mut factors {
        if f.semantics() == FactorSemantics::Causal {
            *f = PolynomialFactor::new(
                f.polynomial().clone(),
                -Atom::one() - Atom::var(e),
                FactorRole::Singularity,
            )
            .with_semantics(FactorSemantics::Causal);
        }
    }
    let input = ParametricIntegrand::new(
        vec![x],
        e,
        ParametricDomain::UnitCube,
        vec![ParametricTerm::new(
            Atom::one(),
            vec![Atom::zero()],
            factors,
        )],
    )
    .unwrap();
    let make_source = |input: &ParametricIntegrand| {
        Arc::new(
            GcadRequest::unit_cube(
                input,
                GcadKinematics {
                    runtime_parameters: vec![s],
                    strict_positive: vec![
                        Atom::var(s) - Atom::num(15),
                        Atom::num(18) - Atom::var(s),
                    ],
                    ..Default::default()
                },
                SolverOptions {
                    order: vec!["v0".into(), "v1".into()],
                    ..Default::default()
                },
                GcadRequest::default_limits(),
            )
            .unwrap()
            .solve_verified()
            .unwrap(),
        )
    };
    let source = make_source(&input);
    let quadratic = NormalizedFamily::admit_parameter_pair_affine_shifted(
        source,
        vec![u],
        proposals(),
        limits(),
    )
    .unwrap();
    assert!(
        matches!(quadratic.continue_symbolically(&options()),Err(Error::Unsupported(message)) if message.contains("rational leading coefficient"))
    );
    let scalar_factors = input.terms()[0]
        .factors()
        .iter()
        .filter(|f| f.semantics() == FactorSemantics::Causal)
        .cloned()
        .collect();
    let scalar_pole = ParametricIntegrand::new(
        vec![x],
        e,
        ParametricDomain::UnitCube,
        vec![ParametricTerm::new(
            Atom::one(),
            vec![Atom::zero()],
            scalar_factors,
        )],
    )
    .unwrap();
    let pole_family = NormalizedFamily::admit_parameter_pair_affine_shifted(
        make_source(&scalar_pole),
        vec![u],
        proposals(),
        limits(),
    )
    .unwrap();
    let pole_continued = pole_family.continue_symbolically(&options()).unwrap();
    assert!(!pole_continued.face_reductions().is_empty());
    assert!(
        pole_continued
            .face_reductions()
            .iter()
            .all(|r| r.quotient.is_zero())
    );
    let scalar = family(&preparation, s, u);
    let continued = scalar.continue_symbolically(&options()).unwrap();
    let coefficient = symbolica::atom::AliasedAtom::from(Atom::one());
    assert!(
        matches!(zeros::discover(&continued,0,0,&coefficient),Err(Error::Unsupported(message)) if message.contains("role/unit receipt"))
    );
    assert!(
        zeros::prove(&continued, 0, 0, &coefficient)
            .unwrap()
            .is_none()
    );
}
