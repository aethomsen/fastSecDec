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
        let field = AtomField {
            statistical_zero_test: false,
            ..AtomField::new()
        };
        let matrix = Matrix::from_linear(
            chart
                .images()
                .iter()
                .flat_map(|a| {
                    [
                        a.derivative(family.coordinates()[0]),
                        a.derivative(family.coordinates()[1]),
                    ]
                })
                .collect(),
            2,
            2,
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
                (determinant
                    - Atom::num(i64::from(chart.orientation())) * chart.positive_measure())
                .as_view(),
                vars,
            )
            .unwrap();
        assert!(ratio.numerator.is_zero());
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
    let request = owner.request();
    let source = &request.prepared_terms()[0];
    let roles = request.domain().coordinates();
    let epsilon = Rational::from((-1, 3));
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
        for point in [
            [Rational::from((1, 3)), Rational::from((2, 5))],
            [Rational::from((3, 4)), Rational::from((1, 2))],
        ] {
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
