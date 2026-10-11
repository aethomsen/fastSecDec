//! Actual verified-section V2 callback and native restoration controls.
use super::{RootProgram, Scope, attempt, numeric::Number};
use crate::{
    parametric::{
        FactorRole, FactorSemantics, ParametricDomain, ParametricIntegrand, ParametricTerm,
        PolynomialFactor,
    },
    threshold::{
        gcad::{GcadKinematics, GcadRequest, SolverOptions},
        maps::{
            CellMap,
            regular::{BracketProposal, Budget, Limits, RegularSection, SectionSide},
        },
    },
};
use std::sync::Arc;
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    domains::{
        float::{Complex, DoubleFloat, ErrorPropagatingFloat as Epf, Float},
        rational::Rational,
    },
    evaluate::{EvaluationDomain, OptimizationSettings},
    symbol,
};
pub(super) fn fixture(
    degree: i64,
    shift: Rational,
) -> (Arc<RegularSection>, Symbol, Rational, Rational) {
    let label = format!("{degree}_{}", shift.to_string().replace('/', "_"));
    let (x, y, e, s, t) = symbol!(
        &format!("affine_bracket{label}::x"),
        &format!("affine_bracket{label}::y"),
        &format!("affine_bracket{label}::e"),
        &format!("affine_bracket{label}::s"),
        &format!("affine_bracket{label}::t")
    );
    let a = Rational::from((1, 3));
    let b = Rational::from((4, 5));

    let ga = Atom::num(&a - &shift).pow(degree) + Atom::num(a.clone());
    let gb = Atom::num(&b - &shift).pow(degree) + Atom::num(b.clone());
    let p = ((Atom::var(y) - Atom::num(shift.clone())).pow(degree) + Atom::var(y)
        - &ga
        - (&gb - &ga) * Atom::var(x))
    .expand();
    let input = ParametricIntegrand::new(
        vec![x, y],
        e,
        ParametricDomain::UnitCube,
        vec![ParametricTerm::new(
            Atom::one(),
            vec![Atom::zero(); 2],
            vec![
                PolynomialFactor::new(p, -Atom::one() - Atom::var(e), FactorRole::Singularity)
                    .with_semantics(FactorSemantics::Causal),
            ],
        )],
    )
    .unwrap();
    let raw = Arc::new(
        GcadRequest::unit_cube(
            &input,
            GcadKinematics::default(),
            SolverOptions {
                order: vec!["v0".into(), "v1".into()],
                ..Default::default()
            },
            GcadRequest::default_limits(),
        )
        .unwrap()
        .solve_verified()
        .unwrap(),
    );
    assert_eq!(raw.cells().len(), 2);
    let map = CellMap::new(raw, 0, vec![s, t]).unwrap();
    let original = RegularSection::prepare(
        map,
        1,
        SectionSide::Upper,
        BracketProposal {
            lower: a.clone(),
            upper: b.clone(),
            derivative_margin: Rational::from((1, 2)),
        },
        &mut Budget::new(Limits::default()),
    )
    .unwrap();
    (original, x, a, b)
}

fn check<T: Number>(degree: i64, shift: Rational, bits: u32)
where
    Complex<T>: EvaluationDomain,
{
    check_recipe::<T>(degree, shift, bits, false);
}
pub(super) fn check_recipe<T: Number>(degree: i64, shift: Rational, bits: u32, shifted: bool)
where
    Complex<T>: EvaluationDomain,
{
    let (section, x, a, b) = fixture(degree, shift.clone());
    let prepare = if shifted {
        RootProgram::prepare_affine_shifted
    } else {
        RootProgram::prepare_affine
    };
    let root = prepare(&section, OptimizationSettings::default().cores(1)).unwrap();
    assert!(RootProgram::prepare(&section, OptimizationSettings::default()).is_err());
    let tag = symbol!("affine_v2_test::root");
    let call = root.call(tag, section.coefficients()).unwrap();
    let first = call.derivative(x);
    let second = first.derivative(x);
    let exact = Atom::evaluator_multiple(&[call, first, second], &[Atom::var(x)])
        .build()
        .unwrap();
    let restored = RootProgram::from_bytes(root.bytes()).unwrap();
    assert_eq!(restored.bytes(), root.bytes());
    let mut scope = Scope::default();
    scope.insert(tag, restored).unwrap();
    let mut eval = scope.enter(bits, || {
        exact.map_coeff_with_prec(
            &|c| Complex::new(T::rational_at(&c.re, bits), T::rational_at(&c.im, bits)),
            bits,
        )
    });
    let delta = (&b - &shift).pow(degree as u64) + &b - (&a - &shift).pow(degree as u64) - &a;
    // Deliberately nonmonotonic query order: no previous-root choice or state.
    for numerator in [8, 0, 4, 7, 1, 0, 8] {
        let xvalue = Rational::from((numerator, 8));
        let zero = T::exact_zero_at(bits);
        let mut out = vec![Complex::new(zero.clone(), zero.clone()); 3];
        attempt(|| {
            eval.evaluate(
                &[Complex::new(T::rational_at(&xvalue, bits), zero)],
                &mut out,
            )
        })
        .unwrap();
        let z = out[0].re.to_f64();
        let min = 1. + degree as f64 * (z - shift.to_f64()).powi(degree as i32 - 1);
        let d1 = delta.to_f64() / min;
        let d2 = -(degree * (degree - 1)) as f64
            * (z - shift.to_f64()).powi(degree as i32 - 2)
            * delta.to_f64().powi(2)
            / min.powi(3);
        let algebra =
            symgcad::algebra::Algebra::new(&section.source().decomposition().native_result().order)
                .unwrap();
        let fiber = algebra
            .specialize_univariate(section.polynomial(), 1, &[xvalue])
            .unwrap();
        let mut roots = symgcad::roots::isolate_union(vec![(0, fiber)]).unwrap();
        assert_eq!(roots.len(), 1);
        for _ in 0..90 {
            symgcad::roots::refine_once(&mut roots[0]);
        }
        assert!(
            z >= roots[0].interval.0.to_f64() - 3e-14 && z <= roots[0].interval.1.to_f64() + 3e-14,
            "root {z}"
        );
        assert!(
            (out[1].re.to_f64() - d1).abs() < 3e-12,
            "d1 {:?} expected{d1}",
            out[1]
        );
        assert!(
            (out[2].re.to_f64() - d2).abs() < 3e-11,
            "d2 {:?} expected{d2}",
            out[2]
        );
    }
}
#[test]
fn certified_nondyadic_faces_and_physical_implicit_jets() {
    for degree in [5, 7] {
        check::<f64>(degree, Rational::zero(), 53);
        check::<DoubleFloat>(degree, Rational::zero(), 106);
        check::<Float>(degree, Rational::zero(), 192);
        check::<Epf<Float>>(degree, Rational::zero(), 192);
    }
    check::<f64>(17, Rational::from((1, 2)), 53);
    check::<Epf<Float>>(17, Rational::from((1, 2)), 192);
}
#[test]
fn incoming_complex_uncertainty_and_codec_version_association() {
    let (section, x, _, _) = fixture(5, Rational::zero());
    let root =
        RootProgram::prepare_affine(&section, OptimizationSettings::default().cores(1)).unwrap();
    let tag = symbol!("affine_v2_test::uncertain");
    let call = root.call(tag, section.coefficients()).unwrap();
    let exact = Atom::evaluator_multiple(std::slice::from_ref(&call), &[Atom::var(x)])
        .build()
        .unwrap();
    type E = Epf<Float>;
    let bits = 192;
    let mut scope = Scope::default();
    scope.insert(tag, root.clone()).unwrap();
    let mut eval = scope.enter(bits, || {
        exact.map_coeff_with_prec(
            &|c| Complex::new(E::rational_at(&c.re, bits), E::rational_at(&c.im, bits)),
            bits,
        )
    });
    for v in [0, 1] {
        let input = Complex::new(
            E::new(Float::with_val(bits, v), 20.),
            E::new(Float::with_val(bits, 0), 21.),
        );
        let mut out = [Complex::new(E::exact_zero_at(bits), E::exact_zero_at(bits))];
        attempt(|| eval.evaluate(&[input], &mut out)).unwrap();
        assert!(out[0].re.get_absolute_error() > 0. && out[0].re.valid());
        assert!(out[0].im.get_absolute_error() > 0. && out[0].im.valid());
    }
    let bad = Complex::new(
        E::new(Float::with_val(bits, 0), 400.),
        E::exact_zero_at(bits),
    );
    let mut out = [Complex::new(E::exact_zero_at(bits), E::exact_zero_at(bits))];
    assert!(
        attempt(|| eval.evaluate(&[bad], &mut out))
            .unwrap_err()
            .contains("uncertainty")
    );
    let (wire, _): (super::program::Wire<'_>, usize) =
        bincode::borrow_decode_from_slice(root.bytes(), bincode::config::standard()).unwrap();
    assert_eq!(wire.version, 2);
    let bytes = bincode::encode_to_vec(
        super::program::Wire { version: 1, ..wire },
        bincode::config::standard(),
    )
    .unwrap();
    assert!(RootProgram::from_bytes(&bytes).is_err());
    assert!(root.call(tag, &[Atom::zero()]).is_err());
    let args = call
        .as_view()
        .as_fun_view()
        .unwrap()
        .iter()
        .map(|a| a.to_owned())
        .collect::<Vec<_>>();
    let wrong = (*super::numeric::ROOT).call_args(args);
    let mut e = scope.enter(bits, || {
        Atom::evaluator_multiple(&[wrong], &[Atom::var(x)])
            .build()
            .unwrap()
            .map_coeff(&|q| q.re.to_f64())
    });
    let mut out = [0.];
    assert!(
        attempt(|| e.evaluate(&[0.5], &mut out))
            .unwrap_err()
            .contains("recipe")
    );
}

#[test]
fn exact_endpoint_controls_and_native_power_preflight() {
    type E = Epf<Float>;
    let bits = 192;
    let one = E::one_at(bits);
    for value in [0, 1] {
        let exact = E::new_with_accuracy(Float::with_val(bits, value), f64::INFINITY);
        let uncertain = E::new(Float::with_val(bits, value), 20.);
        assert_eq!(
            super::numeric::endpoint_control(&exact, &one),
            Some(value as usize)
        );
        assert_eq!(super::numeric::endpoint_control(&uncertain, &one), None);
    }
    let huge = Rational::from(2).pow(4000);
    assert!(matches!(
        RootProgram::prepare_affine_from_certificate(
            3,
            Rational::zero(),
            huge,
            b"oversized endpoint power".to_vec(),
            OptimizationSettings::default()
        ),
        Err(super::Error::Unsupported(_))
    ));
    let root = RootProgram::prepare_affine_from_certificate(
        5,
        Rational::zero(),
        Rational::one(),
        b"bounded v2 codec".to_vec(),
        OptimizationSettings::default(),
    )
    .unwrap();
    let (wire, _): (super::program::Wire<'_>, usize) =
        bincode::borrow_decode_from_slice(root.bytes(), bincode::config::standard()).unwrap();
    // Dyadic endpoints pass the V1 endpoint check; native IR arity still refuses.
    let changed = bincode::encode_to_vec(
        super::program::Wire { version: 1, ..wire },
        bincode::config::standard(),
    )
    .unwrap();
    assert!(matches!(
        RootProgram::from_bytes(&changed),
        Err(super::Error::Invalid(_))
    ));
}
#[test]
fn nondyadic_native_kernel_artifacts_and_mixed_recipe_inspection() {
    use crate::kernel::{
        AlgebraicCallbackRecipe, CompilationSettings, EvaluatorBackend, KernelSet, PrecisionPolicy,
        program::PreparedCoefficientVector,
    };
    for (degree, shift) in [
        (5, Rational::zero()),
        (7, Rational::zero()),
        (17, Rational::from((1, 2))),
    ] {
        let (section, x, _, _) = fixture(degree, shift);
        let root = RootProgram::prepare_affine(&section, OptimizationSettings::default().cores(1))
            .unwrap();
        let tag = symbol!("affine_v2_test::saved");
        let call = root.call(tag, section.coefficients()).unwrap();
        let values = [
            call.clone(),
            call.derivative(x),
            call.derivative(x).derivative(x),
        ];
        let mut scope = Scope::default();
        scope.insert(tag, root).unwrap();
        for backend in [EvaluatorBackend::Eager, EvaluatorBackend::Symjit] {
            let settings = CompilationSettings {
                backend,
                ..Default::default()
            };
            let program = PreparedCoefficientVector {
                runtime_parameters: Vec::new(),
                coordinates: vec![x],
                coefficients: values.iter().cloned().map(Into::into).collect(),
                functions: Default::default(),
                endpoint_profiles: vec![],
            }
            .build(settings)
            .unwrap();
            let set = scope
                .enter(53, || {
                    KernelSet::from_programs_for_load_with_progress(
                        vec![0, 1, 2],
                        vec![program],
                        vec![Atom::zero(); 3],
                        PrecisionPolicy::default(),
                        None,
                        false,
                        vec![],
                        settings,
                        None,
                        None,
                        false,
                        &mut |_| std::ops::ControlFlow::Continue(()),
                    )
                })
                .unwrap();
            assert_eq!(
                set.algebraic_callback_recipe(),
                Some(AlgebraicCallbackRecipe::AffineRationalBracketV2)
            );
            let bytes = set.to_bytes().unwrap();
            let mut restored = KernelSet::from_bytes(&bytes).unwrap();
            assert_eq!(
                restored.algebraic_callback_recipe(),
                set.algebraic_callback_recipe()
            );
            for q in [0., 1., 0.375, 1., 0.] {
                let mut out = [0.; 3];
                restored.sectors_mut()[0].evaluate(&[q], &mut out).unwrap();
                assert!(out.iter().all(|x| x.is_finite()));
            }
            if let Some(path) = std::env::var_os("FASTSECDEC_AFFINE_ARTIFACT_EXPORT") {
                std::fs::create_dir_all(&path).unwrap();
                std::fs::write(
                    std::path::Path::new(&path).join(format!("nondyadic-{degree}-{backend:?}.bin")),
                    bytes,
                )
                .unwrap();
            }
            // Inspection depends on actual immutable helper inventory, including
            // a saved mixture; no fabricated geometry authority is introduced.
            let old = RootProgram::prepare_from_certificate(
                2,
                Rational::zero(),
                Rational::from(2),
                b"mixed inspection v1".to_vec(),
                OptimizationSettings::default(),
            )
            .unwrap();
            restored.sectors[0]
                .algebraic
                .insert(symbol!("affine_v2_test::legacy"), old)
                .unwrap();
            assert_eq!(
                restored.algebraic_callback_recipe(),
                Some(AlgebraicCallbackRecipe::MixedStaticBracketsV1V2)
            );
        }
    }
}
