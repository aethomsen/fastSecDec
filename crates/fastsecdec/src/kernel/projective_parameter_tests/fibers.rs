//! Test-only independent native fixed-fiber and exact rational-gauge references.
use super::*;
use crate::parametric::ParametricDomain;
use crate::threshold::maps::regular::callback::{Number, attempt};
use symbolica::{
    domains::float::{Complex, Float},
    evaluate::{FunctionMap, OptimizationSettings},
};
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
fn compiled_laurent_vectors_match_exact_native_fibers() {
    let (family, s) = family();
    let u = family.coordinates()[0];
    let preparation = family
        .owner()
        .request()
        .projective_preparation()
        .unwrap()
        .clone();
    let continued = family.continue_symbolically(&options()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let mut set = KernelSet::compile_threshold_family(
        &continued,
        dir.path(),
        1,
        PrecisionPolicy::default(),
        CompilationSettings {
            backend: EvaluatorBackend::Eager,
            ..Default::default()
        },
    )
    .unwrap();
    let ir = programs(&set);
    let identity = set.template_content_id().to_owned();
    for value in [Rational::from(16), Rational::from(17)] {
        set.bind_parameters(&BTreeMap::from([(s, value.to_f64())]))
            .unwrap();
        let fixed = (value != 16).then(|| fixed_reference(&preparation, s, u, &value));
        let (expressions, functions) = if let Some(fixed) = &fixed {
            let fc = fixed.continue_symbolically(&options()).unwrap();
            let vector = fc.expand_vector(1).unwrap();
            (
                [-1, 0, 1]
                    .map(|o| {
                        vector
                            .get(&o)
                            .map_or_else(Atom::zero, |v| v.clone().into_inner())
                    })
                    .to_vec(),
                fc.functions().clone(),
            )
        } else {
            (rational_reference(&preparation, u), FunctionMap::new())
        };
        let exact = Atom::evaluator_multiple(&expressions, &[Atom::var(u)])
            .function_map(functions)
            .optimization_settings(OptimizationSettings::default().cores(1))
            .build()
            .unwrap();
        for bits in [128, 256] {
            let convert = |q: &Complex<Rational>| {
                Complex::new(
                    Float::rational_at(&q.re, bits),
                    Float::rational_at(&q.im, bits),
                )
            };
            let mut reference = if let Some(fixed) = &fixed {
                fixed
                    .callback_scope()
                    .enter(bits, || exact.clone().map_coeff_with_prec(&convert, bits))
            } else {
                exact.clone().map_coeff_with_prec(&convert, bits)
            };
            for unit in [0.125, 0.37, 0.875] {
                let mut expected =
                    vec![Complex::new(Float::exact_zero_at(bits), Float::exact_zero_at(bits)); 3];
                attempt(|| {
                    reference.evaluate(
                        &[Complex::new(
                            Float::rational_at(&Rational::try_from(unit).unwrap(), bits),
                            Float::exact_zero_at(bits),
                        )],
                        &mut expected,
                    )
                })
                .unwrap();
                let mut actual = set.exact_coefficients().to_vec();
                for sector in set.sectors_mut() {
                    let mut out = vec![0.; 6];
                    sector.replay_scaled(&[unit], &mut out, 1., 128).unwrap();
                    for (a, b) in actual.iter_mut().zip(out) {
                        *a += b;
                    }
                }
                let expected = expected
                    .iter()
                    .flat_map(|v| [v.re.to_rational().to_f64(), v.im.to_rational().to_f64()])
                    .collect::<Vec<_>>();
                for (a, b) in actual.iter().zip(&expected) {
                    assert!(
                        (a - b).abs() < 2e-11 * (1. + b.abs()),
                        "s={value} u={unit} bits={bits}: {actual:?} vs {expected:?}"
                    );
                }
            }
        }
        assert_eq!(programs(&set), ir);
        assert_eq!(set.template_content_id(), identity);
    }
}
