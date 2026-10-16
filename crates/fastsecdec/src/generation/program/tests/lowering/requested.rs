//! Actual callback execution through the complete native Laurent vector.
use super::*;
use crate::contour::functions::dynamic::{self, observation, requested, requests::Lookup};
use symbolica::domains::float::Float;

#[test]
fn requested_native_jets_share_actual_roots_across_the_laurent_vector() {
    check_vectors(false);
}

#[cfg(feature = "native")]
#[test]
fn requested_symjit_jets_share_actual_roots_across_the_laurent_vector() {
    check_vectors(true);
}

fn check_vectors(native_jit: bool) {
    #[cfg(not(feature = "native"))]
    assert!(!native_jit, "native JIT requires the native feature");
    for recipe in [
        ProgramRecipe::DynamicPolynomialV1,
        ProgramRecipe::DynamicSignAwareV1,
    ] {
        for subtraction in [
            SubtractionStrategy::Taylor,
            SubtractionStrategy::IntegrateByParts,
        ] {
            let generated = generate(
                &repeated_source(),
                &GenerationOptions {
                    program_recipe: recipe,
                    mode: GenerationMode::NumericalDual,
                    subtraction,
                    ..Default::default()
                },
                |_| ControlFlow::Continue(()),
            )
            .unwrap();
            assert_eq!(generated.orders(), &[-1, 0]);
            let runtime = recipe
                .recipe_parameters()
                .iter()
                .map(|name| symbol!(*name))
                .collect::<Vec<_>>();
            let descriptor = generated.program_descriptor().unwrap();
            let _preparing = descriptor.enter();
            let source = &generated.dynamic_check_sources()[0];
            let faces = generated.metadata().charts()[0]
                .contour()
                .unwrap()
                .validation_faces();
            let mut lookup = Lookup::default();
            lookup
                .add_definitions(
                    generated.metadata().charts()[0]
                        .contour()
                        .unwrap()
                        .function_definitions(),
                )
                .unwrap();
            lookup
                .insert(
                    &source.namespace,
                    &source.full_strength,
                    &source.parameters,
                    faces,
                )
                .unwrap();
            let settings = CompilationSettings {
                backend: EvaluatorBackend::Eager,
                ..Default::default()
            };
            for sector in generated.sectors() {
                let deferred = sector.deferred.as_ref().unwrap();
                let plain = native::build(deferred, &runtime, settings).unwrap();
                let mut bundles = BTreeSet::new();
                let exact = native::build_with_lowering(
                    deferred,
                    &runtime,
                    settings,
                    &mut |body, projection| {
                        let mut face = Vec::new();
                        for (axis, coordinate) in projection.iter().enumerate() {
                            match coordinate {
                                Coordinate::Variable(index) if *index == axis => {}
                                Coordinate::Zero => face.push((axis, 0)),
                                Coordinate::One => face.push((axis, 1)),
                                Coordinate::Variable(_) => {
                                    return Err(KernelError::Compilation(
                                        "nonidentity diagnostic coordinate projection".into(),
                                    ));
                                }
                            }
                        }
                        let lowered = lookup
                            .lower_on_face(body, requested::symbol(), &source.parameters, &face)
                            .map_err(KernelError::Compilation)?;
                        bundles.extend(
                            dynamic::requests::referenced_bundles(std::iter::once(&lowered))
                                .map_err(KernelError::Compilation)?,
                        );
                        Ok(lowered)
                    },
                )
                .unwrap();
                // Numerical jets visit distinct physical faces even when the
                // radius values coincide. Repeated derivatives and Laurent
                // outputs retain the same request identity for each face.
                // Distinct rounded candidates for that identity are all observed.
                assert_eq!(
                    bundles.len(),
                    if subtraction == SubtractionStrategy::Taylor {
                        2
                    } else {
                        3
                    },
                );
                if let Some(directory) = std::env::var_os("FASTSECDEC_REQUESTED_IR_DUMP") {
                    let directory = std::path::PathBuf::from(directory);
                    std::fs::create_dir_all(&directory).unwrap();
                    std::fs::write(
                        directory.join(format!("{recipe:?}-{subtraction:?}.txt")),
                        format!("{:#?}", exact.export_instructions()),
                    )
                    .unwrap();
                }
                let requests = observation::Requests::new(bundles).unwrap();
                let map =
                    |value: &Complex<Rational>| Complex::new(value.re.to_f64(), value.im.to_f64());
                let mut reference = plain.clone().map_coeff(&map);
                let mut eager = {
                    let _checked = requested::Mode::checked().enter();
                    exact.clone().map_coeff(&map)
                };
                let mut dd_reference = plain.clone().map_coeff(&|value| {
                    Complex::new(DoubleFloat::from(&value.re), DoubleFloat::from(&value.im))
                });
                let mut dd = {
                    let _checked = requested::Mode::checked().enter();
                    exact.clone().map_coeff(&|value| {
                        Complex::new(DoubleFloat::from(&value.re), DoubleFloat::from(&value.im))
                    })
                };
                let mut mp_reference = dynamic::with_precision(192, || {
                    plain.map_coeff_with_prec(
                        &|value| {
                            Complex::new(
                                value.re.to_multi_prec_float(192),
                                value.im.to_multi_prec_float(192),
                            )
                        },
                        192,
                    )
                });
                let mut mp = {
                    let _checked = requested::Mode::checked().enter();
                    dynamic::with_precision(192, || {
                        exact.clone().map_coeff_with_prec(
                            &|value| {
                                Complex::new(
                                    value.re.to_multi_prec_float(192),
                                    value.im.to_multi_prec_float(192),
                                )
                            },
                            192,
                        )
                    })
                };
                #[cfg(feature = "native")]
                let mut jit = if native_jit {
                    let _checked = requested::Mode::checked().enter();
                    Some(
                        exact
                            .jit_compile::<Complex<f64>>(
                                symbolica::evaluate::JITCompilationSettings::default()
                                    .optimization_level(2)
                                    .direct_translation(true)
                                    .with_option("use_threads", "false"),
                            )
                            .unwrap(),
                    )
                } else {
                    None
                };
                for coordinate in [0.13, 0.25, 0.71] {
                    let values = [coordinate, 0.8, 0.2, 1.];
                    let point = values.map(|value| Complex::new(value, 0.));
                    let mut expected = vec![Complex::new(0., 0.); 2];
                    let mut actual = expected.clone();
                    reference.evaluate(&point, &mut expected);
                    check_attempt(&requests, 53, || eager.evaluate(&point, &mut actual));
                    compare(&actual, &expected, 1e-10);
                    #[cfg(feature = "native")]
                    {
                        if let Some(jit) = &mut jit {
                            check_attempt(&requests, 53, || jit.evaluate(&point, &mut actual));
                            compare(&actual, &expected, 1e-9);
                        }
                    }
                    let point = values
                        .map(|value| Complex::new(DoubleFloat::from(value), DoubleFloat::from(0.)));
                    let zero = Complex::new(DoubleFloat::from(0.), DoubleFloat::from(0.));
                    let mut expected = vec![zero; 2];
                    let mut actual = expected.clone();
                    dd_reference.evaluate(&point, &mut expected);
                    check_attempt(&requests, 106, || dd.evaluate(&point, &mut actual));
                    compare(
                        &actual
                            .iter()
                            .map(|v| Complex::new(v.re.to_f64(), v.im.to_f64()))
                            .collect::<Vec<_>>(),
                        &expected
                            .iter()
                            .map(|v| Complex::new(v.re.to_f64(), v.im.to_f64()))
                            .collect::<Vec<_>>(),
                        1e-14,
                    );
                    let point = values.map(|value| {
                        Complex::new(Float::with_val(192, value), Float::with_val(192, 0))
                    });
                    let zero = Complex::new(Float::with_val(192, 0), Float::with_val(192, 0));
                    let mut expected = vec![zero; 2];
                    let mut actual = expected.clone();
                    mp_reference.evaluate(&point, &mut expected);
                    check_attempt(&requests, 192, || mp.evaluate(&point, &mut actual));
                    for (a, b) in actual.iter().zip(&expected) {
                        assert!((a.re.clone() - &b.re).to_f64().abs() < 1e-45);
                        assert!((a.im.clone() - &b.im).to_f64().abs() < 1e-45);
                    }
                }
                assert!(sector.materialized.get().is_none());
            }
        }
    }
}

fn check_attempt(requests: &observation::Requests, bits: u32, evaluate: impl FnOnce()) {
    let (result, failure) = dynamic::isolated_attempt(|| {
        let attempt = requests.begin();
        evaluate();
        attempt.finish()
    });
    assert!(failure.is_none(), "native callback: {failure:?}");
    let candidates = result.unwrap();
    assert_eq!(
        candidates
            .iter()
            .map(|(index, _)| *index)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        requests.bundles().count()
    );
    assert!(
        candidates
            .iter()
            .all(|(_, candidate)| { candidate.lambda > 0 && candidate.bits == bits })
    );
}

fn compare(actual: &[Complex<f64>], expected: &[Complex<f64>], tolerance: f64) {
    for (a, b) in actual.iter().zip(expected) {
        assert!((a.re - b.re).abs() <= tolerance * (1. + b.re.abs()));
        assert!((a.im - b.im).abs() <= tolerance * (1. + b.im.abs()));
    }
}
