use super::*;
use crate::{
    contour::{DynamicConstruction, functions::dynamic},
    generation::{GenerationOptions, generate},
    kernel::{CompilationSettings, PrecisionPolicy, ProgramRecipe, compilation},
    parametric::{
        FactorRole, FactorSemantics, ParametricDomain, ParametricIntegrand, ParametricTerm,
        PolynomialFactor,
    },
};
use std::ops::ControlFlow;
use symbolica::{
    atom::{Atom, AtomCore},
    symbol,
};

struct Fixture {
    prepared: compilation::PreparedDynamic,
    source: Atom,
    coordinates: [Symbol; 2],
    hidden_axis: usize,
    runtime: Vec<Symbol>,
    mode: ContourMode,
    program: crate::kernel::program::SectorProgram,
}
fn fixture(hidden_dependency: bool) -> Fixture {
    fixture_with_power(hidden_dependency, false)
}
fn fixture_with_power(hidden_dependency: bool, higher_pole: bool) -> Fixture {
    let x = symbol!("dynamic_validation_scope::x");
    let y = symbol!("dynamic_validation_scope::y");
    let eps = symbol!("dynamic_validation_scope::eps");
    let causal = Atom::num((1, 4)) - Atom::var(x)
        + if hidden_dependency {
            Atom::var(y).pow(2)
        } else {
            Atom::Zero
        };
    let input = ParametricIntegrand::new(
        vec![x, y],
        eps,
        ParametricDomain::UnitCube,
        vec![ParametricTerm::new(
            Atom::one(),
            vec![
                if higher_pole {
                    Atom::num(-3) - Atom::var(eps)
                } else {
                    Atom::Zero
                },
                Atom::Zero,
            ],
            vec![
                PolynomialFactor::new(causal, Atom::num(-1), FactorRole::Singularity)
                    .with_semantics(FactorSemantics::Causal),
                PolynomialFactor::new(
                    Atom::one() + Atom::var(y).pow(2),
                    Atom::Zero,
                    FactorRole::Polynomial,
                )
                .with_semantics(FactorSemantics::Positive),
            ],
        )],
    )
    .unwrap();
    let generated = generate(
        &input,
        &GenerationOptions {
            program_recipe: ProgramRecipe::DynamicPolynomialV1,
            ..Default::default()
        },
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    let runtime = compilation::runtime_inputs(&generated, &[]);
    let check_source = &generated.dynamic_check_sources()[0];
    let contour = generated.metadata().charts()[0].contour().unwrap();
    let positive_symbols = contour.positive_polynomials()[0].get_all_symbols(false);
    let hidden_axis = check_source
        .parameters
        .iter()
        .position(|symbol| positive_symbols.contains(symbol))
        .unwrap();
    let coordinates = [
        check_source.parameters[1 - hidden_axis],
        check_source.parameters[hidden_axis],
    ];
    let source = check_source.full_strength.clone();
    let prepared =
        compilation::PreparedDynamic::build(&generated, &runtime, CompilationSettings::default())
            .unwrap()
            .unwrap();
    let program = {
        let _owner = prepared.descriptor.enter();
        crate::kernel::program::build_sector_with_lowering(
            &generated.sectors()[0],
            &runtime,
            CompilationSettings::default(),
            Some(&prepared.lookup),
        )
        .unwrap()
    };
    Fixture {
        program,
        prepared,
        source,
        coordinates,
        hidden_axis,
        runtime,
        mode: ContourMode::Dynamical {
            safety_fraction: 0.8,
            lambda_cap: 1.,
            displacement_cap: 1.,
            construction: DynamicConstruction::Polynomial,
        },
    }
}
impl Fixture {
    fn specification(&self, parameters: &[Symbol], bundle: Bundle) -> Arc<Specification> {
        Specification::new(
            &self.prepared.descriptor,
            parameters,
            vec![0.8, 1., 1.],
            self.mode,
            [bundle].into_iter().collect(),
            &PrecisionPolicy {
                max_bits: 256,
                ..Default::default()
            },
        )
        .unwrap()
    }
    fn bundle(&self) -> Bundle {
        self.prepared.lookup.bundle(&self.source).unwrap()
    }
    fn candidate(&self, x: f64, y: f64) -> Rational {
        let _owner = self.prepared.descriptor.enter();
        let exact = self
            .source
            .evaluator(
                &self
                    .coordinates
                    .iter()
                    .chain(&self.runtime)
                    .copied()
                    .map(Atom::var)
                    .collect::<Vec<_>>(),
            )
            .build()
            .unwrap();
        let mut evaluator = exact.map_coeff(&|value| value.re.to_f64());
        let mut output = [0.];
        let (_, failure) =
            dynamic::isolated_attempt(|| evaluator.evaluate(&[x, y, 0.8, 1., 1.], &mut output));
        assert!(failure.is_none(), "{failure:?}");
        Rational::try_from(output[0]).unwrap()
    }
}

#[test]
fn missing_source_axis_is_certified_uniformly_and_never_filled_with_a_sample() {
    let fixture = fixture(false);
    let bundle = fixture.bundle();
    let specification = fixture.specification(&fixture.coordinates[..1], bundle.clone());
    let mut validation = Validation::new(specification);
    let candidate = fixture.candidate(0.3, 0.75);
    assert_eq!(candidate, fixture.candidate(0.3, 0.25));
    assert!(validation.checks.is_none());
    assert!(
        validation.evaluate(&[0.3, 0.8, 1., 1.], || {
            dynamic::observation::record(&bundle, candidate.clone(), 53).unwrap();
        }),
        "{:?}",
        validation.last_error
    );
    assert!(validation.checked_arguments > 0);
    assert!(validation.maximum_bits >= 96);

    let dependent = fixture_with_hidden_dependency();
    let bundle = dependent.bundle();
    let candidate = dependent.candidate(0.3, 0.75);
    let mut validation =
        Validation::new(dependent.specification(&dependent.coordinates[..1], bundle.clone()));
    assert!(!validation.evaluate(&[0.3, 0.8, 1., 1.], || {
        dynamic::observation::record(&bundle, candidate, 53).unwrap();
    }));
    let failure = validation.last_error.as_ref().unwrap();
    assert!(failure.contains("None"), "{failure}");
    assert!(failure.contains("certificate precisions"), "{failure}");
    assert!(failure.contains("namespace"), "{failure}");
    assert!(!failure.contains("pole"));
}
fn fixture_with_hidden_dependency() -> Fixture {
    fixture(true)
}

#[test]
fn every_distinct_candidate_is_certified_in_every_saved_context() {
    let mut fixture = fixture(false);
    let mut descriptor = (*fixture.prepared.descriptor).clone();
    let mut second = descriptor.clone();
    second.remap_charts(&[1]).unwrap();
    descriptor.merge(second).unwrap();
    fixture.prepared.descriptor = Arc::new(descriptor);
    let bundle = fixture.bundle();
    let specification = fixture.specification(&fixture.coordinates[..1], bundle.clone());
    let contexts = specification.bindings[0].len();
    assert_eq!(contexts, 2);
    let candidate = fixture.candidate(0.3, 0.75);
    let adjacent = Rational::try_from(candidate.to_f64().next_up()).unwrap();
    assert_ne!(candidate, adjacent);
    let mut validation = Validation::new(specification);
    assert!(
        validation.evaluate(&[0.3, 0.8, 1., 1.], || {
            dynamic::observation::record(&bundle, candidate.clone(), 53).unwrap();
            dynamic::observation::record(&bundle, candidate.clone(), 53).unwrap();
            dynamic::observation::record(&bundle, adjacent, 53).unwrap();
            dynamic::observation::record(&bundle, candidate, 106).unwrap();
        }),
        "{:?}",
        validation.last_error
    );
    let coverage = validation.coverage().unwrap();
    assert_eq!(coverage.checked_arguments, 3 * contexts);
    assert_eq!(coverage.bundles.len(), 1);
    assert_eq!(coverage.contexts.len(), contexts);
    assert!(
        coverage
            .contexts
            .iter()
            .all(|context| context.checked_arguments == 3)
    );
}

#[test]
fn one_bad_candidate_rejects_the_attempt_even_when_another_is_valid() {
    let fixture = fixture(false);
    let bundle = fixture.bundle();
    let candidate = fixture.candidate(0.3, 0.75);
    for (wrong, reason) in [
        (
            &candidate / &Rational::from(2),
            "intended-root approximation accuracy",
        ),
        (
            &candidate * &Rational::from(100),
            "outside the sufficient causal radius",
        ),
    ] {
        let mut validation =
            Validation::new(fixture.specification(&fixture.coordinates[..1], bundle.clone()));
        let invalid_follows_valid = wrong > candidate;
        assert!(!validation.evaluate(&[0.3, 0.8, 1., 1.], || {
            dynamic::observation::record(&bundle, candidate.clone(), 53).unwrap();
            dynamic::observation::record(&bundle, wrong, 53).unwrap();
        }));
        assert!(
            validation.last_error.as_ref().unwrap().contains(reason),
            "{:?}",
            validation.last_error
        );
        assert!(validation.coverage().is_none());
        if invalid_follows_valid {
            assert_eq!(
                validation.checked_arguments,
                validation.specification.bindings[0].len() + 1
            );
        }
        // A failed candidate is not retained into the next precision/sample attempt.
        assert!(
            validation.evaluate(&[0.3, 0.8, 1., 1.], || {
                dynamic::observation::record(&bundle, candidate.clone(), 53).unwrap();
            }),
            "{:?}",
            validation.last_error
        );
    }
}

#[test]
fn actual_callback_attempts_restore_nested_state_and_clone_independent_counters() {
    let fixture = fixture(false);
    let bundle = fixture.bundle();
    let specification = fixture.specification(&fixture.coordinates[..1], bundle.clone());
    let _owner = fixture.prepared.descriptor.enter();
    let lowered = fixture
        .prepared
        .lookup
        .lower(&fixture.source, dynamic::requested::symbol())
        .unwrap();
    let exact = lowered
        .evaluator(
            &std::iter::once(&fixture.coordinates[0])
                .chain(&fixture.runtime)
                .copied()
                .map(Atom::var)
                .collect::<Vec<_>>(),
        )
        .build()
        .unwrap();
    let mut evaluator = {
        let _mode = dynamic::requested::Mode::checked().enter();
        exact.map_coeff(&|value| value.re.to_f64())
    };
    let point = [0.3, 0.8, 1., 1.];
    let mut output = [0.];
    let mut validation = Validation::new(specification.clone());
    assert!(
        validation.evaluate(&point, || evaluator.evaluate(&point, &mut output)),
        "{:?}",
        validation.last_error
    );
    let mut cloned = validation.clone();
    assert_eq!(cloned.checked_arguments, 0);
    assert!(cloned.last_error.is_none());
    let previous_count = validation.checked_arguments;
    let (_, outer_failure) = dynamic::isolated_attempt(|| {
        dynamic::failure("outer diagnostic survives".into());
        assert!(!cloned.evaluate(&point, || dynamic::failure("masked native failure".into())));
        assert!(
            cloned
                .last_error
                .as_ref()
                .unwrap()
                .contains("masked native failure")
        );
        assert!(cloned.evaluate(&point, || evaluator.evaluate(&point, &mut output)));
    });
    assert_eq!(outer_failure.as_deref(), Some("outer diagnostic survives"));
    assert_eq!(validation.checked_arguments, previous_count);
    assert!(cloned.last_error.is_none());
    let outer = specification.enter();
    assert!(Arc::ptr_eq(&capture().unwrap(), &specification));
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _empty = enter_optional(None);
            assert!(capture().is_none());
            panic!("preparation unwind");
        }))
        .is_err()
    );
    assert!(Arc::ptr_eq(&capture().unwrap(), &specification));
    drop(outer);
    assert!(capture().is_none());
}

#[test]
fn native_symbol_projection_preserves_permuted_source_coordinates_and_fixed_faces() {
    let fixture = fixture(true);
    let mut bundle = fixture.bundle();
    for request in &mut bundle.0 {
        request.face = vec![(fixture.hidden_axis, 0)];
    }
    let specification = fixture.specification(
        &[fixture.coordinates[1], fixture.coordinates[0]],
        bundle.clone(),
    );
    let candidate = fixture.candidate(0.3, 0.);
    let mut validation = Validation::new(specification);
    assert!(
        validation.evaluate(&[0.91, 0.3, 0.8, 1., 1.], || {
            dynamic::observation::record(&bundle, candidate, 53).unwrap();
        }),
        "{:?}",
        validation.last_error
    );
}

#[test]
fn checked_native_owners_certify_complete_vectors_batches_and_precision_remaps() {
    use crate::kernel::{EvaluatorBackend, evaluator, precision_cache::PrecisionCache};
    use symbolica::domains::float::{Complex, DoubleFloat, ErrorPropagatingFloat, Float, RealLike};
    let fixture = fixture_with_power(false, true);
    let exact = &fixture.program.exact;
    assert!(exact.get_output_len() >= 2);
    let callbacks = crate::kernel::program::callbacks(exact).unwrap();
    let bundles = callbacks
        .iter()
        .filter(|callback| callback.symbol == dynamic::requested::symbol())
        .map(|callback| Bundle::from_atom(callback.tags[2].as_view()).unwrap())
        .collect();
    let specification = Specification::new(
        &fixture.prepared.descriptor,
        &fixture.program.parameters,
        vec![0.8, 1., 1.],
        fixture.mode,
        bundles,
        &PrecisionPolicy::default(),
    )
    .unwrap();
    let (mut eager, mut plain, requirements) = {
        let _owner = fixture.prepared.descriptor.enter();
        let plain = evaluator::complex(exact, EvaluatorBackend::Eager).unwrap();
        let _checking = specification.enter();
        (
            evaluator::complex(exact, EvaluatorBackend::Eager).unwrap(),
            plain,
            evaluator::MappingRequirements::new(exact).unwrap(),
        )
    };
    let mut point = fixture
        .program
        .parameters
        .iter()
        .map(|symbol| {
            if *symbol == fixture.coordinates[0] {
                0.3
            } else {
                0.6
            }
        })
        .collect::<Vec<_>>();
    point.extend([0.8, 1., 1.]);
    let input = point
        .iter()
        .map(|x| Complex::new(*x, 0.))
        .collect::<Vec<_>>();
    let mut expected = vec![Complex::new(0., 0.); exact.get_output_len()];
    let mut actual = expected.clone();
    plain.evaluate(&input, &mut expected);
    eager.evaluate(&input, &mut actual);
    assert!(
        actual
            .iter()
            .all(|value| value.re.is_finite() && value.im.is_finite()),
        "{:?}",
        eager.last_dynamic_error()
    );
    assert_eq!(actual, expected);
    assert!(plain.validation().is_none());
    assert!(
        eager
            .validation()
            .unwrap()
            .coverage()
            .unwrap()
            .checked_arguments
            > 0
    );
    let matrix = [input.clone(), input.clone(), input.clone()].concat();
    let mut values = vec![Complex::new(0., 0.); 3 * actual.len()];
    assert_eq!(
        eager
            .evaluate_batch(&matrix, &mut values, 3, input.len(), actual.len())
            .len(),
        3
    );
    assert!(values.chunks_exact(actual.len()).all(|row| row == expected));
    #[cfg(feature = "native")]
    {
        let mut jit = {
            let _owner = fixture.prepared.descriptor.enter();
            let _checking = specification.enter();
            evaluator::complex(exact, EvaluatorBackend::Symjit).unwrap()
        };
        jit.evaluate(&input, &mut actual);
        assert!(
            actual
                .iter()
                .all(|value| value.re.is_finite() && value.im.is_finite()),
            "{:?}",
            jit.last_dynamic_error()
        );
        for (left, right) in actual.iter().zip(&expected) {
            assert!((left.re - right.re).abs() < 1e-9 * (1. + right.re.abs()));
            assert!((left.im - right.im).abs() < 1e-9 * (1. + right.im.abs()));
        }
    }
    let mut dd = PrecisionCache::<Complex<DoubleFloat>>::new(requirements.clone());
    assert!(
        dd.evaluate(
            exact,
            &point,
            106,
            |value| Complex::new(DoubleFloat::from(&value.re), DoubleFloat::from(&value.im)),
            |value| Complex::new(DoubleFloat::from(value), DoubleFloat::from(0.))
        )
        .unwrap()
        .iter()
        .all(|value| value.re.to_f64().is_finite() && value.im.to_f64().is_finite()),
        "{:?}",
        dd.last_dynamic_error
    );
    let mut mp = PrecisionCache::<Complex<Float>>::new(requirements.clone());
    for bits in [128, 192] {
        assert!(
            mp.evaluate(
                exact,
                &point,
                bits,
                |value| Complex::new(
                    value.re.to_multi_prec_float(bits),
                    value.im.to_multi_prec_float(bits)
                ),
                |value| Complex::new(Float::with_val(bits, value), Float::with_val(bits, 0))
            )
            .unwrap()
            .iter()
            .all(|value| value.re.is_finite() && value.im.is_finite()),
            "{:?}",
            mp.last_dynamic_error
        );
    }
    let mut conditioning =
        evaluator::Conditioning::<Complex<ErrorPropagatingFloat<f64>>>::new(requirements);
    let numeric = conditioning
        .get_or_map(exact, |value| {
            Complex::new(
                ErrorPropagatingFloat::new(value.re.to_f64(), 15.),
                ErrorPropagatingFloat::new(value.im.to_f64(), 15.),
            )
        })
        .unwrap();
    let input = point
        .iter()
        .map(|x| {
            Complex::new(
                ErrorPropagatingFloat::new(*x, 15.),
                ErrorPropagatingFloat::new(0., 15.),
            )
        })
        .collect::<Vec<_>>();
    let mut output = vec![
        Complex::new(
            ErrorPropagatingFloat::new(0., 15.),
            ErrorPropagatingFloat::new(0., 15.)
        );
        expected.len()
    ];
    assert!(!numeric.evaluate_at(&input, &mut output, &point));
    assert!(
        conditioning
            .validation()
            .unwrap()
            .coverage()
            .unwrap()
            .checked_arguments
            > 0
    );
}
