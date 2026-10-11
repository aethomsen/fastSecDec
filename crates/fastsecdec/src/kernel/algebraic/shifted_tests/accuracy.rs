use super::*;
#[test]
fn nondyadic_faces_reordered_queries_and_implicit_jets() {
    for degree in [5, 7] {
        super::super::affine_tests::check_recipe::<f64>(degree, Rational::zero(), 53, true);
        super::super::affine_tests::check_recipe::<DoubleFloat>(
            degree,
            Rational::zero(),
            106,
            true,
        );
        super::super::affine_tests::check_recipe::<Float>(degree, Rational::zero(), 256, true);
        super::super::affine_tests::check_recipe::<Epf<Float>>(degree, Rational::zero(), 256, true);
    }
    super::super::affine_tests::check_recipe::<Float>(17, Rational::from((1, 2)), 256, true);
}
pub(super) fn evaluator(
    section: &RegularSection,
    x: Symbol,
) -> (super::super::program::Exact, Scope) {
    let program =
        RootProgram::prepare_affine_shifted(section, OptimizationSettings::default().cores(1))
            .unwrap();
    let tag = symbol!("shifted_v3_test::root");
    let call = program.call(tag, section.coefficients()).unwrap();
    let exact = Atom::evaluator_multiple(
        &[
            call.clone(),
            call.derivative(x),
            call.derivative(x).derivative(x),
        ],
        &[Atom::var(x)],
    )
    .build()
    .unwrap();
    let mut scope = Scope::default();
    let restored = RootProgram::from_bytes(program.bytes()).unwrap();
    assert_eq!(program.bytes(), restored.bytes());
    scope.insert(tag, restored).unwrap();
    (exact, scope)
}
pub(super) fn exact_fiber_check(section: &RegularSection, x: Symbol, points: &[Rational]) {
    let (exact, scope) = evaluator(section, x);
    let algebra =
        symgcad::algebra::Algebra::new(&section.source().decomposition().native_result().order)
            .unwrap();
    for bits in [128, 256] {
        let mut eval = scope.enter(bits, || {
            exact.clone().map_coeff_with_prec(
                &|q| {
                    Complex::new(
                        Float::rational_at(&q.re, bits),
                        Float::rational_at(&q.im, bits),
                    )
                },
                bits,
            )
        });
        for point in points {
            let mut out =
                vec![Complex::new(Float::exact_zero_at(bits), Float::exact_zero_at(bits)); 3];
            attempt(|| {
                eval.evaluate(
                    &[Complex::new(
                        Float::rational_at(point, bits),
                        Float::exact_zero_at(bits),
                    )],
                    &mut out,
                )
            })
            .unwrap();
            let fiber = algebra
                .specialize_univariate(section.polynomial(), 1, std::slice::from_ref(point))
                .unwrap();
            let mut roots = symgcad::roots::isolate_union(vec![(0, fiber)]).unwrap();
            for r in &mut roots {
                for _ in 0..16 {
                    symgcad::roots::refine_once(r);
                }
            }
            // Independent native exact isolation, selected by the certified cylinder.
            let root = roots
                .iter_mut()
                .find(|r| {
                    r.interval.1 >= section.bracket().lower
                        && r.interval.0 <= section.bracket().upper
                })
                .unwrap();
            for _ in 0..bits + 64 {
                symgcad::roots::refine_once(root);
            }
            let actual = out[0].re.to_rational();
            let tolerance = Rational::from((1, 2)).pow(u64::from(bits - 12));
            assert!(
                actual >= &root.interval.0 - &tolerance && actual <= &root.interval.1 + &tolerance,
                "{bits}-bit root disagrees with exact fiber: {actual}, interval {:?}",
                root.interval
            );
            assert!(out.iter().all(|v| v.re.valid() && v.im.valid()));
        }
    }
}
#[test]
fn working_precision_exact_fibers_and_rational_closed_faces() {
    for (degree, shift) in [
        (5, Rational::zero()),
        (7, Rational::zero()),
        (17, Rational::from((1, 2))),
    ] {
        let (section, x, a, b) = super::super::affine_tests::fixture(degree, shift);
        exact_fiber_check(
            &section,
            x,
            &[Rational::zero(), Rational::from((1, 2)), Rational::one()],
        );
        let (exact, scope) = evaluator(&section, x);
        for bits in [128, 256] {
            let mut eval = scope.enter(bits, || {
                exact.clone().map_coeff_with_prec(
                    &|q| {
                        Complex::new(
                            Float::rational_at(&q.re, bits),
                            Float::rational_at(&q.im, bits),
                        )
                    },
                    bits,
                )
            });
            for (point, expected) in [(Rational::zero(), a.clone()), (Rational::one(), b.clone())] {
                let mut out =
                    vec![Complex::new(Float::exact_zero_at(bits), Float::exact_zero_at(bits)); 3];
                attempt(|| {
                    eval.evaluate(
                        &[Complex::new(
                            Float::rational_at(&point, bits),
                            Float::exact_zero_at(bits),
                        )],
                        &mut out,
                    )
                })
                .unwrap();
                let error = out[0].re.to_rational() - expected;
                let tolerance = Rational::from((1, 2)).pow(u64::from(bits - 12));
                assert!(error >= -tolerance.clone() && error <= tolerance);
            }
        }
    }
}
#[test]
fn original_input_real_and_imaginary_uncertainty_survives_shift_and_faces() {
    let (section, x, _, _) = super::super::affine_tests::fixture(5, Rational::zero());
    let (exact, scope) = evaluator(&section, x);
    type E = Epf<Float>;
    let bits = 256;
    let mut eval = scope.enter(bits, || {
        exact.map_coeff_with_prec(
            &|q| Complex::new(E::rational_at(&q.re, bits), E::rational_at(&q.im, bits)),
            bits,
        )
    });
    for point in [0., 1., 0.5, 1., 0.] {
        let input = Complex::new(
            E::new(Float::with_val(bits, point), 20.),
            E::new(Float::with_val(bits, 0), 21.),
        );
        let mut out = vec![Complex::new(E::exact_zero_at(bits), E::exact_zero_at(bits)); 3];
        attempt(|| eval.evaluate(&[input], &mut out)).unwrap();
        assert!(out.iter().all(|v| v.re.valid() && v.im.valid()));
        assert!(out[0].re.get_absolute_error() > 0.);
        assert!(out[0].im.get_absolute_error() > 0.);
    }
    let mut out = vec![Complex::new(E::exact_zero_at(bits), E::exact_zero_at(bits)); 3];
    let invalid = Complex::new(
        E::new(Float::with_val(bits, 0), 400.),
        E::exact_zero_at(bits),
    );
    assert!(
        attempt(|| eval.evaluate(&[invalid], &mut out))
            .unwrap_err()
            .contains("uncertainty")
    );
}
