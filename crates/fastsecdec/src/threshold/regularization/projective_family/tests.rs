use super::*;
use crate::{
    parametric::{
        FactorRole, FactorSemantics, ParametricDomain, ParametricIntegrand, ParametricTerm,
        PolynomialFactor,
    },
    threshold::gcad::{GcadKinematics, GcadRequest, SolverOptions},
};
use symbolica::{
    domains::{
        atom::AtomField,
        float::{Complex, Float},
    },
    symbol,
    tensors::matrix::Matrix,
};

fn triangle(scale: i64) -> Arc<VerifiedDecomposition> {
    triangle_numerator(scale, false)
}
fn triangle_numerator(scale: i64, complex_numerator: bool) -> Arc<VerifiedDecomposition> {
    let (x, y, z, e) = symbol!(
        "projective_family_test::x",
        "projective_family_test::y",
        "projective_family_test::z",
        "projective_family_test::eps"
    );
    let u = Atom::var(x) + Atom::var(y) + Atom::var(z);
    let f = &u * &u - Atom::num(scale) * Atom::var(x) * Atom::var(z);
    let mut factors = vec![
        PolynomialFactor::new(
            u,
            -Atom::num(if complex_numerator { 2 } else { 1 }) + Atom::num(2) * Atom::var(e),
            FactorRole::Singularity,
        )
        .with_semantics(FactorSemantics::Positive),
        PolynomialFactor::new(f, -Atom::one() - Atom::var(e), FactorRole::Singularity)
            .with_semantics(FactorSemantics::Causal),
    ];
    if complex_numerator {
        factors.push(PolynomialFactor::new(
            (Atom::one() + Atom::num(2) * Atom::i()) * (Atom::var(x) + Atom::num(2) * Atom::var(z)),
            Atom::one(),
            FactorRole::Polynomial,
        ));
    }
    let input = ParametricIntegrand::new(
        vec![x, y, z],
        e,
        ParametricDomain::ProjectiveSimplex,
        vec![ParametricTerm::new(
            -Atom::one(),
            vec![Atom::Zero; 3],
            factors,
        )],
    )
    .unwrap();
    Arc::new(
        GcadRequest::projective(
            crate::threshold::projective::AffineProjectivePreparation::eliminate(&input, 1)
                .unwrap(),
            GcadKinematics::default(),
            SolverOptions {
                order: vec!["v1".into(), "v0".into()],
                ..Default::default()
            },
            symgcad::Limits {
                workers: 1,
                wall_time_secs: 10.,
                memory_mib: 1024,
                ..Default::default()
            },
        )
        .unwrap()
        .solve_verified()
        .unwrap(),
    )
}
#[test]
fn complete_massive_triangle_fans_keep_source_phases_and_native_jacobians() {
    let owner = triangle(5);
    let (a, b) = symbol!("projective_family_test::a", "projective_family_test::b");
    let family =
        ResolvedProjectiveFamily::admit(owner.clone(), vec![a, b], Default::default(), |_| {
            ControlFlow::Continue(())
        })
        .unwrap();
    assert!(Arc::ptr_eq(family.owner(), &owner));
    assert_eq!(family.charts().len(), 18);
    assert_eq!(family.convergence_strip().upper(), Some(&Rational::zero()));
    let mut cells = BTreeSet::new();
    for chart in family.charts() {
        cells.insert(chart.cell_index());
        assert_eq!(chart.terms().len(), 1);
        for factor in chart.factors() {
            assert!(factor.numerator().margin() > &Rational::zero());
            assert!(factor.denominator().margin() > &Rational::zero());
        }
        check_signed_measure(&family, chart);
        let negative = chart.factors().iter().any(|f| {
            matches!(f.origin(), FactorOrigin::Factor { factor: 1, .. }) && f.sign() == -1
        });
        assert_eq!(negative, chart.cell_index() == 2);
    }
    assert_eq!(cells, BTreeSet::from([0, 1, 2, 3]));
    let continued = family
        .continue_symbolically(
            &crate::generation::GenerationOptions {
                max_order: 0,
                ..Default::default()
            },
            |_| ControlFlow::Continue(()),
        )
        .unwrap();
    assert_eq!(continued.charts().len(), 18);
    assert_eq!(continued.profiles().len(), 18);
}
#[test]
fn complete_owner_refuses_coordinate_capture_and_cancellation() {
    let owner = triangle(5);
    let x = owner.request().domain().coordinates()[0];
    let b = symbol!("projective_family_test::b");
    assert!(
        ResolvedProjectiveFamily::admit(owner.clone(), vec![x, b], Default::default(), |_| {
            ControlFlow::Continue(())
        })
        .is_err()
    );
    let a = symbol!("projective_family_test::a");
    assert!(matches!(
        ResolvedProjectiveFamily::admit(
            owner.clone(),
            vec![symbolica::transcendental::root_var(), b],
            Default::default(),
            |_| ControlFlow::Continue(())
        ),
        Err(Error::Invalid(message)) if message.contains("native root polynomial variable")
    ));
    assert!(matches!(
        ResolvedProjectiveFamily::admit(owner.clone(), vec![a, b], Default::default(), |_| {
            ControlFlow::Break(())
        }),
        Err(Error::Cancelled)
    ));
    assert!(
        ResolvedProjectiveFamily::admit(
            owner,
            vec![a, b],
            Limits {
                max_charts: 1,
                ..Default::default()
            },
            |_| ControlFlow::Continue(())
        )
        .is_err()
    );
}
use std::collections::BTreeSet;

#[test]
fn full_complex_density_matches_original_source_at_common_epsilon() {
    let owner = triangle_numerator(5, true);
    let (a, b) = symbol!(
        "projective_family_test::complex_a",
        "projective_family_test::complex_b"
    );
    let family =
        ResolvedProjectiveFamily::admit(owner.clone(), vec![a, b], Default::default(), |_| {
            ControlFlow::Continue(())
        })
        .unwrap();
    assert_eq!(family.charts().len(), 18);
    check_full_density(
        &owner,
        &family,
        Rational::from((-1, 3)),
        &[
            vec![Rational::from((1, 3)), Rational::from((2, 5))],
            vec![Rational::from((3, 4)), Rational::from((1, 2))],
        ],
    );
}

#[test]
fn rational_massless_sunrise_field_keeps_all_faces_and_regulator_pole() {
    let (x, y, z, eps, a, b) = symbol!(
        "projective_rational_test::x",
        "projective_rational_test::y",
        "projective_rational_test::z",
        "projective_rational_test::eps",
        "projective_rational_test::a",
        "projective_rational_test::b"
    );
    let e = Atom::var(eps);
    let u = Atom::var(x) * Atom::var(y) + Atom::var(x) * Atom::var(z) + Atom::var(y) * Atom::var(z);
    let f = -Atom::var(x) * Atom::var(y) * Atom::var(z);
    let prefactor = -symbolica::function!(
        symbolica::transcendental::gamma(),
        -Atom::one() + Atom::num(2) * &e
    );
    let input = ParametricIntegrand::new(
        vec![x, y, z],
        eps,
        ParametricDomain::ProjectiveSimplex,
        vec![ParametricTerm::new(
            prefactor.clone(),
            vec![Atom::Zero; 3],
            vec![
                PolynomialFactor::new(
                    u,
                    -Atom::num(3) + Atom::num(3) * &e,
                    FactorRole::Singularity,
                )
                .with_semantics(FactorSemantics::Positive),
                PolynomialFactor::new(f, Atom::one() - Atom::num(2) * &e, FactorRole::Singularity)
                    .with_semantics(FactorSemantics::Causal),
            ],
        )],
    )
    .unwrap();
    let owner = Arc::new(
        GcadRequest::projective(
            crate::threshold::projective::AffineProjectivePreparation::eliminate(&input, 2)
                .unwrap(),
            GcadKinematics::default(),
            SolverOptions::default(),
            symgcad::Limits {
                workers: 1,
                wall_time_secs: 10.,
                memory_mib: 1024,
                ..Default::default()
            },
        )
        .unwrap()
        .solve_verified()
        .unwrap(),
    );
    assert_eq!(owner.cells().len(), 1);
    let family =
        ResolvedProjectiveFamily::admit(owner.clone(), vec![a, b], Default::default(), |_| {
            ControlFlow::Continue(())
        })
        .unwrap();
    assert!(Arc::ptr_eq(family.owner(), &owner));
    let halves = family
        .charts()
        .iter()
        .map(|c| c.endpoint_path()[0])
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(halves, (0..4).collect::<std::collections::BTreeSet<_>>());
    for chart in family.charts() {
        assert!(chart.geometry().constant_field().is_trivial());
        assert!(chart.geometry().sections().is_empty());
        assert_eq!(chart.cell_index(), 0);
        assert_eq!(chart.terms().len(), 1);
        assert_eq!(
            chart.terms()[0].prefactor(),
            &(-prefactor.clone() * (Atom::num(2) * Atom::i() * Atom::var(Symbol::PI) * &e).exp())
        );
    }
    let continued = family
        .continue_symbolically(&Default::default(), |_| ControlFlow::Continue(()))
        .unwrap();
    assert_eq!(continued.charts().len(), family.charts().len());
}

fn check_signed_measure(family: &ResolvedProjectiveFamily, chart: &ResolvedChart) {
    let field = AtomField {
        statistical_zero_test: false,
        ..AtomField::new()
    };
    let matrix = Matrix::from_linear(
        chart
            .images()
            .iter()
            .flat_map(|a| {
                family
                    .coordinates()
                    .iter()
                    .map(|s| a.derivative(*s))
                    .collect::<Vec<_>>()
            })
            .collect(),
        u32::try_from(family.coordinates().len()).unwrap(),
        u32::try_from(family.coordinates().len()).unwrap(),
        field,
    )
    .unwrap();
    let determinant = matrix.det().unwrap();
    let mut context = chart.geometry().algebra.context().clone();
    let vars = Arc::new(
        family
            .coordinates()
            .iter()
            .copied()
            .map(symbolica::poly::PolyVariable::from)
            .collect::<Vec<_>>(),
    );
    let ratio = context
        .to_rational_polynomial::<u32>(
            (determinant - Atom::num(i64::from(chart.orientation())) * chart.positive_measure())
                .as_view(),
            vars,
        )
        .unwrap();
    assert!(ratio.numerator.is_zero());
}

fn check_full_density(
    owner: &Arc<VerifiedDecomposition>,
    family: &ResolvedProjectiveFamily,
    epsilon: Rational,
    points: &[Vec<Rational>],
) {
    let request = owner.request();
    let source = &request.prepared_terms()[0];
    let roles = request.domain().coordinates();
    let tolerance = Rational::from((1, 2)).pow(100);
    for chart in family.charts() {
        let cell = owner.cells().nth(chart.cell_index()).unwrap();
        // Independent expression from the original prepared source, with the
        // lower-lip phase formed directly from its retained native cell sign.
        let mut original = source.prefactor().clone();
        for (role, exponent) in roles.iter().zip(source.monomial_powers()) {
            original *= Atom::var(*role).pow(exponent.clone());
        }
        for (index, factor) in source.factors().iter().enumerate() {
            let sign = if factor.role() == FactorRole::Polynomial {
                1
            } else {
                cell.signed_factors()
                    .find(|(f, _)| f.term_index == 0 && f.factor_index == index)
                    .unwrap()
                    .1
            };
            original *=
                (Atom::num(i64::from(sign)) * factor.polynomial()).pow(factor.exponent().clone());
            if sign < 0 {
                original *= (-Atom::i() * Atom::var(Symbol::PI) * factor.exponent()).exp();
            }
        }
        let term = &chart.terms()[0];
        let mut resolved = term.prefactor() * term.regular();
        for (role, exponent) in family.coordinates().iter().zip(term.powers()) {
            resolved *= Atom::var(*role).pow(exponent.clone());
        }
        for point in points {
            let cf = |q: &Rational| {
                Complex::new(
                    q.to_multi_prec_float(128),
                    Rational::zero().to_multi_prec_float(128),
                )
            };
            let unit = family
                .coordinates()
                .iter()
                .copied()
                .map(Atom::var)
                .zip(point.iter().map(cf))
                .chain([(Atom::var(request.input().regulator()), cf(&epsilon))])
                .collect::<std::collections::HashMap<_, _>>();
            let mapped = chart
                .images()
                .iter()
                .map(|image| {
                    image
                        .evaluate_with_prec::<_, Complex<Float>>(&unit, 128)
                        .unwrap()
                })
                .collect::<Vec<_>>();
            let named = roles
                .iter()
                .copied()
                .map(Atom::var)
                .zip(mapped)
                .chain([(Atom::var(request.input().regulator()), cf(&epsilon))])
                .collect::<std::collections::HashMap<_, _>>();
            let reference = original
                .evaluate_with_prec::<_, Complex<Float>>(&named, 128)
                .unwrap()
                * chart
                    .positive_measure()
                    .evaluate_with_prec::<_, Complex<Float>>(&unit, 128)
                    .unwrap();
            let actual = resolved
                .evaluate_with_prec::<_, Complex<Float>>(&unit, 128)
                .unwrap();
            for (got, expected) in [(&actual.re, &reference.re), (&actual.im, &reference.im)] {
                let expected = expected.to_rational();
                let difference = (got.to_rational() - &expected).abs();
                assert!(
                    difference <= &tolerance * &expected.abs().max(Rational::one()),
                    "cell {} chart {:?}: {}",
                    chart.cell_index(),
                    chart.endpoint_path(),
                    difference
                );
            }
        }
    }
}

#[test]
fn complete_three_dimensional_box_keeps_signed_measure_and_complex_density() {
    let (x, y, z, w, eps, a, b, c) = symbol!(
        "projective_box_test::x",
        "projective_box_test::y",
        "projective_box_test::z",
        "projective_box_test::w",
        "projective_box_test::eps",
        "projective_box_test::a",
        "projective_box_test::b",
        "projective_box_test::c"
    );
    let u = Atom::var(x) + Atom::var(y) + Atom::var(z) + Atom::var(w);
    // Native graph export stores the original homogeneous monomials. Affine
    // elimination then repeats (1 - x - y - z), unlike a compact U^2 spelling.
    let f = (&u * &u - Atom::num(5) * Atom::var(y) * Atom::var(w) + Atom::var(x) * Atom::var(z))
        .expand();
    let e = Atom::var(eps);
    let input = ParametricIntegrand::new(
        vec![x, y, z, w],
        eps,
        ParametricDomain::ProjectiveSimplex,
        vec![ParametricTerm::new(
            Atom::one(),
            vec![Atom::Zero; 4],
            vec![
                PolynomialFactor::new(u, -Atom::one() + Atom::num(2) * &e, FactorRole::Singularity)
                    .with_semantics(FactorSemantics::Positive),
                PolynomialFactor::new(f, -Atom::num(2) - &e, FactorRole::Singularity)
                    .with_semantics(FactorSemantics::Causal),
                PolynomialFactor::new(
                    (Atom::one() + Atom::num(2) * Atom::i())
                        * (Atom::var(x) + Atom::num(2) * Atom::var(w)),
                    Atom::one(),
                    FactorRole::Polynomial,
                ),
            ],
        )],
    )
    .unwrap();
    let owner = Arc::new(
        GcadRequest::projective(
            crate::threshold::projective::AffineProjectivePreparation::eliminate(&input, 3)
                .unwrap(),
            GcadKinematics::default(),
            SolverOptions {
                order: vec!["v1".into(), "v2".into(), "v0".into()],
                ..Default::default()
            },
            symgcad::Limits {
                workers: 1,
                wall_time_secs: 30.,
                memory_mib: 2048,
                ..Default::default()
            },
        )
        .unwrap()
        .solve_verified()
        .unwrap(),
    );
    assert_eq!(owner.cells().len(), 6);
    let family =
        ResolvedProjectiveFamily::admit(owner.clone(), vec![a, b, c], Default::default(), |_| {
            ControlFlow::Continue(())
        })
        .unwrap();
    let cells = family
        .charts()
        .iter()
        .map(ResolvedChart::cell_index)
        .collect::<BTreeSet<_>>();
    assert_eq!(cells, (0..6).collect::<BTreeSet<_>>());
    for cell in cells {
        assert_eq!(
            family
                .charts()
                .iter()
                .filter(|v| v.cell_index() == cell)
                .map(|v| v.endpoint_path()[0])
                .collect::<BTreeSet<_>>(),
            (0..8).collect::<BTreeSet<_>>()
        );
    }
    for chart in family.charts() {
        check_signed_measure(&family, chart);
        let receipt = chart
            .factors()
            .iter()
            .find(|r| r.origin() == (FactorOrigin::Factor { term: 0, factor: 1 }))
            .unwrap();
        assert_eq!(
            receipt.source(),
            owner.request().prepared_terms()[0].factors()[1].polynomial()
        );
    }
    let epsilon = Rational::from((-4, 3));
    assert!(
        family
            .convergence_strip()
            .lower()
            .is_none_or(|b| b < &epsilon)
    );
    assert!(
        family
            .convergence_strip()
            .upper()
            .is_none_or(|b| &epsilon < b)
    );
    check_full_density(
        &owner,
        &family,
        epsilon,
        &[
            vec![
                Rational::from((1, 3)),
                Rational::from((2, 5)),
                Rational::from((3, 7)),
            ],
            vec![
                Rational::from((3, 4)),
                Rational::from((1, 2)),
                Rational::from((1, 5)),
            ],
        ],
    );
}
