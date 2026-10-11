use super::*;
use crate::{
    parametric::{
        FactorRole, FactorSemantics, ParametricDomain, ParametricIntegrand, ParametricTerm,
        PolynomialFactor,
    },
    threshold::{
        gcad::{PositiveProjectiveQuotient, Problem, SolverOptions},
        maps::{
            PositiveProjectiveCellMap,
            normalized::{Limits, NormalizedPrefix},
        },
        projective::PositiveProjectivePreparation,
    },
};
use symbolica::{
    atom::EvaluationInfo,
    domains::float::{Float, SingleFloat},
    function,
};

fn settings() -> OptimizationSettings {
    OptimizationSettings::default().cores(1)
}
fn budget() -> Budget {
    Budget::new(Limits::default())
}
fn roles() -> (Symbol, Symbol, Symbol, Symbol, Symbol) {
    symbol!(
        "open_density_test::a",
        "open_density_test::x",
        "open_density_test::eps",
        "open_density_test::c",
        "open_density_test::unit"
    )
}
fn fixture(prefactor: Option<Atom>) -> Vec<Arc<CompiledOpenMap>> {
    let (a, x, eps, c, u) = roles();
    let ea = Atom::var(eps);
    let aa = Atom::var(a);
    let xa = Atom::var(x);
    let terms = [
        (
            Atom::num((-2, 3)) - &ea,
            Atom::one() + &ea,
            aa.clone() + Atom::i() * &xa,
        ),
        (
            Atom::num((-7, 5)) - Atom::num(2) * &ea,
            Atom::num(2) - &ea,
            Atom::num(2) * &aa - Atom::i() * &xa,
        ),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (exponent, positive_exponent, numerator))| {
        let coordinate_power = if i == 0 {
            Atom::num((1, 3)) + &ea
        } else {
            Atom::num((2, 5)) - &ea
        };
        ParametricTerm::new(
            prefactor.clone().unwrap_or_else(|| {
                if i == 0 {
                    Atom::var(c)
                } else {
                    Atom::num(2) + Atom::i()
                }
            }),
            vec![
                Atom::num(-3) - &exponent - &positive_exponent - &coordinate_power,
                coordinate_power,
            ],
            vec![
                PolynomialFactor::new(&aa + &xa, positive_exponent, FactorRole::Singularity)
                    .with_semantics(FactorSemantics::Positive),
                PolynomialFactor::new(&aa - &xa, exponent, FactorRole::Singularity)
                    .with_semantics(FactorSemantics::Causal),
                PolynomialFactor::new(numerator, Atom::one(), FactorRole::Polynomial),
            ],
        )
    })
    .collect();
    let source = Arc::new(
        ParametricIntegrand::new(vec![a, x], eps, ParametricDomain::ProjectiveSimplex, terms)
            .unwrap(),
    );
    let preparation = Arc::new(PositiveProjectivePreparation::anchor(source, 0).unwrap());
    let problem = Problem {
        variables: vec!["x0".into(), "x1".into()],
        constraints: vec!["x0>0".into(), "x1>0".into()],
        split: vec!["x0-x1".into()],
        solver: SolverOptions {
            homogeneous_anchor: Some("x0".into()),
            homogeneous_block: vec!["x0".into(), "x1".into()],
            ordering: "given".into(),
            ..Default::default()
        },
        limits: symgcad::Limits {
            workers: 1,
            wall_time_secs: 10.,
            memory_mib: 1024,
            ..Default::default()
        },
        ..Default::default()
    };
    let raw = Arc::new(symgcad::solver::solve(&problem).unwrap());
    let owner = Arc::new(PositiveProjectiveQuotient::verify(preparation, raw).unwrap());
    assert_eq!(owner.cell_count(), 2);
    (0..2)
        .map(|i| {
            let cell = PositiveProjectiveCellMap::new(owner.clone(), i, vec![u]).unwrap();
            let prefix = NormalizedPrefix::new(cell, &budget())
                .unwrap()
                .open(&budget())
                .unwrap();
            Arc::new(CompiledOpenMap::prepare(prefix, settings(), &budget()).unwrap())
        })
        .collect()
}
fn prepared(map: Arc<CompiledOpenMap>) -> Arc<CompiledOpenDensity> {
    let (_, _, eps, c, _) = roles();
    Arc::new(
        CompiledOpenDensity::prepare(map, &[eps], &[c], FunctionMap::new(), settings(), &budget())
            .unwrap(),
    )
}

#[test]
fn open_density_preserves_two_causal_powers_complex_numerators_and_single_jacobian() {
    for map in fixture(None) {
        let density = prepared(map.clone());
        assert_eq!(density.term_bodies().len(), 2);
        assert_eq!(density.term_phases().len(), 2);
        let mut eval = density.map::<f64>(53).unwrap();
        let mut seen_negative = false;
        for (u, e, c) in [
            (0.25, 0.25, Complex::new(1.2, -0.4)),
            (0.5, 0.125, Complex::new(-0.5, 0.75)),
            (0.75, 0.25, Complex::new(2., 1.)),
        ] {
            let out = eval.evaluate(&[u], &[e], std::slice::from_ref(&c)).unwrap();
            let x = out.coordinates[0];
            let j = out.jacobian;
            let negative = x > 1.;
            seen_negative |= negative;
            let expected_x = if negative { 1. / (1. - u) } else { u };
            let expected_j = if negative { 1. / (1. - u).powi(2) } else { 1. };
            assert!((x - expected_x).abs() < 1e-14 && (j - expected_j).abs() < 1e-13);
            let exponents = [-2. / 3. - e, -7. / 5. - 2. * e];
            let positive_exponents = [1. + e, 2. - e];
            let coordinate_powers = [1. / 3. + e, 2. / 5. - e];
            let ns = [Complex::new(1., x), Complex::new(2., -x)];
            let prefs = [c, Complex::new(2., 1.)];
            let mut sum = Complex::new(0., 0.);
            for i in 0..2 {
                // Independent scalar lower-lip phase; no production phase helper.
                let angle = if negative {
                    -std::f64::consts::PI * exponents[i]
                } else {
                    0.
                };
                let phase = Complex::new(angle.cos(), angle.sin());
                let scale = x.powf(coordinate_powers[i])
                    * (1. + x).powf(positive_exponents[i])
                    * (1. - x).abs().powf(exponents[i])
                    * expected_j;
                let expected = prefs[i] * ns[i] * phase * Complex::new(scale, 0.);
                assert!((out.terms[i].re - expected.re).abs() < 2e-12 * (1. + expected.re.abs()));
                assert!((out.terms[i].im - expected.im).abs() < 2e-12 * (1. + expected.im.abs()));
                sum += expected;
            }
            assert!((out.total.re - sum.re).abs() < 4e-12 * (1. + sum.re.abs()));
            assert!((out.total.im - sum.im).abs() < 4e-12 * (1. + sum.im.abs()));
        }
        assert_eq!(
            density.term_phases().iter().all(|p| p == &Atom::one()),
            !seen_negative
        );
    }
}

#[test]
fn open_density_source_body_composition_refreshes_128_256_and_native_codec() {
    for map in fixture(None) {
        let density = prepared(map);
        let encoded =
            bincode::serde::encode_to_vec(density.exact(), bincode::config::standard()).unwrap();
        let (restored, used): (ExpressionEvaluator<Complex<Rational>>, usize) =
            bincode::serde::decode_from_slice(&encoded, bincode::config::standard()).unwrap();
        assert_eq!(used, encoded.len());
        for bits in [128u32, 256] {
            let real = |q: &Rational| Float::with_val(bits, 1).from_rational(q);
            let scalar = |q: &Complex<Rational>| Complex::new(real(&q.re), real(&q.im));
            let mut body = density
                .map
                .scope()
                .map_checked(density.body_exact(), bits, scalar)
                .unwrap();
            let mut restored = density
                .map
                .scope()
                .map_checked(&restored, bits, scalar)
                .unwrap();
            let mut eval = density.map::<Float>(bits).unwrap();
            let mut geometry = density.map.map::<Float>(bits);
            for (u, e) in [((1, 4), (1, 4)), ((2, 3), (1, 8))] {
                let u = real(&Rational::from(u));
                let e = real(&Rational::from(e));
                let c = Complex::new(
                    real(&Rational::from((3, 2))),
                    real(&Rational::from((-1, 3))),
                );
                let out = eval
                    .evaluate(
                        std::slice::from_ref(&u),
                        std::slice::from_ref(&e),
                        std::slice::from_ref(&c),
                    )
                    .unwrap();
                let coords = geometry.evaluate(std::slice::from_ref(&u)).unwrap();
                let zero = Float::new(bits);
                let input = [
                    Complex::new(coords[0].clone(), zero.clone()),
                    Complex::new(e.clone(), zero.clone()),
                    c.clone(),
                ];
                let mut values = vec![Complex::new(zero.clone(), zero.clone()); 2];
                body.evaluate(&input, &mut values);
                let mut decoded = vec![Complex::new(zero.clone(), zero.clone()); 6];
                restored.evaluate(
                    &[Complex::new(u, zero.clone()), Complex::new(e, zero), c],
                    &mut decoded,
                );
                let tolerance = Rational::from((1, 2)).pow(u64::from(bits - 20));
                for (i, v) in values.iter().enumerate() {
                    let expected = v * Complex::new(coords[1].clone(), Float::new(bits));
                    for (a, b) in [
                        (&out.terms[i].re, &expected.re),
                        (&out.terms[i].im, &expected.im),
                    ] {
                        let delta = (a.clone() - b).to_rational().abs();
                        assert!(delta < &tolerance * &(Rational::one() + b.to_rational().abs()));
                    }
                    assert_eq!(decoded[i + 1], out.terms[i]);
                }
            }
        }
    }
}

#[test]
fn open_density_native_function_map_keeps_coordinate_roles_and_complex_coefficients() {
    let map = fixture(None).remove(0);
    let (_, x, eps, c, _) = roles();
    let k = symbol!("open_density_test::runtime_k");
    let mut functions = FunctionMap::new();
    functions
        .add_aliases([
            (Atom::var(c), Atom::num(2) * Atom::var(k) + Atom::i()),
            // Declared native source/regulator inputs take precedence over aliases.
            (Atom::var(x), Atom::num(100)),
            (Atom::var(eps), Atom::num(100)),
        ])
        .unwrap();
    let with_functions = Arc::new(
        CompiledOpenDensity::prepare(map.clone(), &[eps], &[k], functions, settings(), &budget())
            .unwrap(),
    );
    let reference = prepared(map);
    let mut a = with_functions.map::<f64>(53).unwrap();
    let mut b = reference.map::<f64>(53).unwrap();
    for k in [Complex::new(0.5, 0.25), Complex::new(-0.75, 1.25)] {
        let c = Complex::new(2. * k.re, 2. * k.im + 1.);
        let a = a.evaluate(&[0.25], &[0.125], &[k]).unwrap();
        let b = b.evaluate(&[0.25], &[0.125], &[c]).unwrap();
        assert_eq!(a.coordinates, b.coordinates);
        for (a, b) in a.terms.iter().zip(&b.terms) {
            assert!((a.re - b.re).abs() < 1e-13 && (a.im - b.im).abs() < 1e-13);
        }
    }
}

#[test]
fn open_density_causal_phase_ignores_caller_pi_alias_and_coefficient_slot() {
    let map = fixture(None)
        .into_iter()
        .find(|m| {
            let source = m.owner().source();
            source.decomposition().native_result().cells[source.cell_index()].signs[0] < 0
        })
        .unwrap();
    let (_, _, eps, c, _) = roles();
    let reference = prepared(map.clone());
    let mut baseline = reference.map::<f64>(53).unwrap();
    let cvalue = Complex::new(1.5, -0.25);
    let expected = baseline.evaluate(&[0.25], &[0.25], &[cvalue]).unwrap();
    for explicit_slot in [false, true] {
        let mut functions = FunctionMap::new();
        functions
            .add_aliases([(Atom::var(Symbol::PI), Atom::zero())])
            .unwrap();
        let symbols = if explicit_slot {
            vec![c, Symbol::PI]
        } else {
            vec![c]
        };
        let owner = Arc::new(
            CompiledOpenDensity::prepare(
                map.clone(),
                &[eps],
                &symbols,
                functions,
                settings(),
                &budget(),
            )
            .unwrap(),
        );
        let mut evaluator = owner.map::<f64>(53).unwrap();
        let coefficients = if explicit_slot {
            vec![cvalue, Complex::new(0., 0.)]
        } else {
            vec![cvalue]
        };
        let actual = evaluator.evaluate(&[0.25], &[0.25], &coefficients).unwrap();
        for (a, b) in actual.terms.iter().zip(&expected.terms) {
            assert!((a.re - b.re).abs() < 1e-13 && (a.im - b.im).abs() < 1e-13);
        }
        // The caller's freedom to specialize *source coefficient* PI is
        // separate; this fixture has no PI outside the protected causal phase.
    }
}

#[test]
fn open_density_reuses_genuine_two_section_scope_and_tracks_incoming_uncertainty() {
    use crate::threshold::maps::normalized::{PrefixStep, tests::fixture};
    use symbolica::domains::float::{ErrorPropagatingFloat as Epf, RealLike};
    let (power, first, second) = fixture();
    let mut b = budget();
    let prefix = power
        .section(first, &mut b)
        .unwrap()
        .section(second, &mut b)
        .unwrap();
    let eps = prefix
        .source()
        .decomposition()
        .preparation()
        .input()
        .regulator();
    let map = Arc::new(CompiledOpenMap::prepare(prefix.clone(), settings(), &b).unwrap());
    for step in prefix.steps() {
        if let PrefixStep::Section { section, .. } = step {
            for forbidden in std::iter::once(section.root_symbol())
                .chain(section.chart().parameters.iter().copied())
            {
                assert!(
                    CompiledOpenDensity::prepare(
                        map.clone(),
                        &[eps],
                        &[forbidden],
                        FunctionMap::new(),
                        settings(),
                        &b
                    )
                    .is_err()
                );
            }
        }
    }
    let density = Arc::new(
        CompiledOpenDensity::prepare(map, &[eps], &[], FunctionMap::new(), settings(), &b).unwrap(),
    );
    let mut ordinary = density.map::<f64>(53).unwrap();
    for [u, v, t] in [[0.5, 0.75, 0.25], [0.25, 0.125, 0.75], [0.5, 0.75, 0.25]] {
        let w = (1.0_f64 + v * v).sqrt();
        let f = (1. - u * u) * u.powi(4) * (1. - v * v) * (1. + v * v) * (1. - t * t);
        let expected = f.powf(-4. / 5.) * (2. * u.powi(3) * w);
        let out = ordinary.evaluate(&[u, v, t], &[0.25], &[]).unwrap();
        assert!((out.total.re - expected).abs() < 1e-11 * (1. + expected));
        assert_eq!(out.total.im, 0.);
    }
    for bits in [128u32, 256] {
        let mut tracked = density.map::<Epf<Float>>(bits).unwrap();
        let mut nominal = density.map::<Float>(bits).unwrap();
        let mut geometry = density.map.map::<Float>(bits);
        let mut body = density
            .map
            .scope()
            .map_checked(density.body_exact(), bits, |q| {
                Complex::new(
                    Float::rational_at(&q.re, bits),
                    Float::rational_at(&q.im, bits),
                )
            })
            .unwrap();
        for v in [
            Rational::from((3, 4)),
            Rational::from((1, 3)),
            Rational::from((3, 4)),
        ] {
            let inputs = [Rational::from((1, 2)), v, Rational::from((1, 4))]
                .map(|q| Float::rational_at(&q, bits));
            let eps = Float::rational_at(&Rational::from((1, 4)), bits);
            let out = nominal
                .evaluate(&inputs, std::slice::from_ref(&eps), &[])
                .unwrap();
            let coordinates = geometry.evaluate(&inputs).unwrap();
            let body_input = coordinates[..3]
                .iter()
                .chain(std::iter::once(&eps))
                .map(|v| Complex::new(v.clone(), Float::new(bits)))
                .collect::<Vec<_>>();
            let mut independent = vec![Complex::new(Float::new(bits), Float::new(bits)); 1];
            body.evaluate(&body_input, &mut independent);
            let reference =
                &independent[0] * Complex::new(coordinates[3].clone(), Float::new(bits));
            let tolerance = Rational::from((1, 2)).pow(u64::from(bits - 20));
            assert!(
                (out.total.re.clone() - &reference.re).to_rational().abs()
                    < &tolerance * &(Rational::one() + reference.re.to_rational().abs())
            );
            let tracked_inputs = inputs.map(|v| Epf::new(v, 20.));
            let out = tracked
                .evaluate(&tracked_inputs, &[Epf::new(eps, 20.)], &[])
                .unwrap();
            assert!(out.total.re.valid() && out.total.im.valid());
            assert!(out.total.re.get_absolute_error() > 0.);
            assert!(
                out.coordinates
                    .iter()
                    .all(|x| x.valid() && x.get_absolute_error() > 0.)
            );
            // Keep the native tracked center; this is ordinary error tracking,
            // not a closed-face or a posteriori accuracy certificate.
            assert!(out.total.re.to_f64().is_finite());
        }
    }
}

#[test]
fn open_density_rejects_boundaries_role_capture_and_masked_callback_preparation_failure() {
    let map = fixture(None).remove(0);
    let (_, x, eps, c, u) = roles();
    for (regs, coeffs) in [
        (vec![], vec![c]),
        (vec![eps, eps], vec![c]),
        (vec![eps], vec![x]),
        (vec![eps], vec![u]),
    ] {
        assert!(
            CompiledOpenDensity::prepare(
                map.clone(),
                &regs,
                &coeffs,
                FunctionMap::new(),
                settings(),
                &budget()
            )
            .is_err()
        );
    }
    let density = prepared(map);
    let weak = Arc::downgrade(&density);
    let mut eval = density.map::<f64>(53).unwrap();
    drop(density);
    assert!(weak.upgrade().is_some());
    let c = Complex::new(1., 0.);
    for p in [0., 1., f64::NAN] {
        assert!(eval.evaluate(&[p], &[0.25], &[c]).is_err());
    }
    assert!(eval.evaluate(&[0.5], &[f64::NAN], &[c]).is_err());
    assert!(eval.evaluate(&[0.5], &[0.25], &[]).is_err());
    assert!(eval.evaluate(&[0.5], &[0.25], &[c]).is_ok());
    drop(eval);
    assert!(weak.upgrade().is_none());
    let failing = symbol!(
        "open_density_test::failing_constant",
        eval =
            EvaluationInfo::constant(|_, _| Err("deliberate open-density constant failure".into()))
    );
    let masked = (Atom::one() + Atom::var(failing)).pow(-1);
    let map = fixture(Some(masked)).remove(0);
    let density = prepared(map);
    let error = match density.map::<Float>(128) {
        Ok(_) => panic!("masked failing constant must refuse before evaluation"),
        Err(e) => e,
    };
    assert!(
        error
            .to_string()
            .contains("deliberate open-density constant failure")
    );
    let unsupported = symbol!(
        "open_density_test::real_only",
        eval = EvaluationInfo::new().register(|args: &[f64]| args[0] + 1.)
    );
    let map = fixture(Some(function!(unsupported, Atom::num(1)))).remove(0);
    let density = prepared(map);
    assert!(density.map::<Float>(128).is_err());
}

mod body;
