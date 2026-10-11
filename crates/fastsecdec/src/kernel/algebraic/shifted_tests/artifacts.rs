use super::*;
#[test]
fn saved_eager_symjit_and_mixed_recipe_inventory() {
    use crate::kernel::{
        AlgebraicCallbackRecipe, CompilationSettings, EvaluatorBackend, KernelSet, PrecisionPolicy,
        program::PreparedCoefficientVector,
    };
    let mut sections = narrow_sections();
    sections.push(super::super::affine_tests::fixture(7, Rational::zero()).0);
    for (index, section) in sections.iter().enumerate() {
        let x = section.source().axes()[0].symbol();
        let root =
            RootProgram::prepare_affine_shifted(section, OptimizationSettings::default().cores(1))
                .unwrap();
        let tag = symbol!("shifted_v3_test::saved");
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
                Some(AlgebraicCallbackRecipe::AffineCoefficientShiftV3)
            );
            let bytes = set.to_bytes().unwrap();
            let mut restored = KernelSet::from_bytes(&bytes).unwrap();
            assert_eq!(
                restored.algebraic_callback_recipe(),
                set.algebraic_callback_recipe()
            );
            for point in [0.9999325137757114, 0., 1., 0.5, 0.] {
                let mut out = [0.; 3];
                restored.sectors_mut()[0]
                    .evaluate(&[point], &mut out)
                    .unwrap();
                assert!(out.iter().all(|v| v.is_finite()));
            }
            if let Some(path) = std::env::var_os("FASTSECDEC_SHIFTED_ARTIFACT_EXPORT") {
                std::fs::create_dir_all(&path).unwrap();
                std::fs::write(
                    std::path::Path::new(&path).join(format!("shifted-{index}-{backend:?}.bin")),
                    bytes,
                )
                .unwrap();
            }
            let old = RootProgram::prepare_affine_from_certificate(
                5,
                Rational::zero(),
                Rational::one(),
                b"mixed old v2".to_vec(),
                OptimizationSettings::default(),
            )
            .unwrap();
            restored.sectors[0]
                .algebraic
                .insert(symbol!("shifted_v3_test::old"), old)
                .unwrap();
            assert_eq!(
                restored.algebraic_callback_recipe(),
                Some(AlgebraicCallbackRecipe::MixedStaticBracketsWithV3)
            );
        }
    }
}
