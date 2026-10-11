use super::GenerationInput;
use fastsecdec::generation::{CoefficientExpansionMethod, CoefficientExpansionOptions};
use fastsecdec::parametric::FamilyPreparationPolicy;

#[test]
fn timed_drain_is_explicit_positive_and_supported_by_the_selected_scheduler() {
    let mut settings = super::IntegrationInput::default();
    assert_eq!(settings.max_seconds, None);
    assert!(
        !serde_json::to_value(&settings)
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("max_seconds")
    );
    settings.max_seconds = Some(600.);
    assert!(settings.validate_execution().is_err());
    settings.serial_seconds = Some(1.);
    assert!(settings.validate_execution().is_ok());
    for value in [
        0.,
        -1.,
        f64::NAN,
        f64::INFINITY,
        f64::MAX,
        f64::MIN_POSITIVE,
    ] {
        settings.max_seconds = Some(value);
        assert!(settings.validate_execution().is_err());
    }
    settings.max_seconds = Some(600.);
    settings.serial_seconds = None;
    settings.method = "discrete_mc".into();
    assert!(settings.validate_execution().is_ok());
}

#[test]
fn maintained_kite_card_uses_native_input_and_horner_ten_with_optional_source_subset() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/contour/scalar_benchmarks/kite.toml");
    let loaded = crate::input::load(&path).unwrap();
    assert_eq!(loaded.loops, Some(2));
    assert_eq!(loaded.propagators, 5);
    assert_eq!(loaded.integrand.parameters().len(), 5);
    assert_eq!(loaded.card.generation.evaluator.horner_iterations, 10);
    assert_eq!(loaded.card.generation.source_sectors, None);
    let selected: GenerationInput = toml::from_str("source_sectors=[7,0]").unwrap();
    assert_eq!(selected.source_sectors, Some(vec![7, 0]));
    assert!(toml::from_str::<GenerationInput>("source_sectors=[-1]").is_err());
}

#[test]
fn execution_defaults_preserve_omitted_round_limit_through_artifacts_and_overlays() {
    use super::IntegrationInput;
    let mut settings: IntegrationInput = toml::from_str("").unwrap();
    assert_eq!(settings.max_rounds, None);
    assert_eq!(settings.ordinary_max_rounds(), 1);
    assert_eq!(settings.serial_seconds, None);
    assert!(settings.double_points);
    assert!(!GenerationInput::default().serial);
    let stored = serde_json::to_value(&settings).unwrap();
    assert!(stored.get("max_rounds").is_none());
    settings = serde_json::from_value(stored).unwrap();
    let overlay = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        overlay.path(),
        "serial_seconds = 60.0\ndouble_points = false",
    )
    .unwrap();
    settings.apply_overlay(overlay.path()).unwrap();
    assert_eq!(settings.max_rounds, None);
    assert_eq!(settings.serial_seconds, Some(60.0));
    assert!(!settings.double_points);
    std::fs::write(overlay.path(), "max_rounds = 3").unwrap();
    settings.apply_overlay(overlay.path()).unwrap();
    assert_eq!(settings.max_rounds, Some(3));
    settings.validate_execution().unwrap();
}

#[test]
fn serial_execution_rejects_invalid_residence_and_discrete_mc() {
    use super::IntegrationInput;
    let mut settings = IntegrationInput::default();
    for seconds in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        settings.serial_seconds = Some(seconds);
        assert!(settings.validate_execution().is_err());
    }
    settings.serial_seconds = Some(0.001);
    settings.method = "discrete_mc".into();
    assert!(
        settings
            .validate_execution()
            .unwrap_err()
            .to_string()
            .contains("per-sector Havana")
    );
    settings.serial_seconds = None;
    settings.validate_execution().unwrap();
    settings.max_rounds = Some(0);
    assert!(settings.validate_execution().is_err());
}

#[test]
fn periodization_reuses_native_variants_and_preserves_the_default() {
    use super::IntegrationInput;
    use fastsecdec::integration::Periodization;

    assert_eq!(
        IntegrationInput::default()
            .qmc_settings()
            .unwrap()
            .periodization,
        Periodization::Korobov3
    );
    for (name, expected) in [
        ("none", Periodization::None),
        ("korobov3", Periodization::Korobov3),
        ("korobov2", Periodization::Korobov2),
    ] {
        let input: IntegrationInput = toml::from_str(&format!("periodization='{name}'")).unwrap();
        assert_eq!(input.qmc_settings().unwrap().periodization, expected);
        let restored: IntegrationInput =
            serde_json::from_slice(&serde_json::to_vec(&input).unwrap()).unwrap();
        assert_eq!(
            restored.qmc_settings().unwrap(),
            input.qmc_settings().unwrap()
        );
    }
    for name in ["unknown", "Korobov2"] {
        let input: IntegrationInput = toml::from_str(&format!("periodization='{name}'")).unwrap();
        assert_eq!(
            input.qmc_settings().unwrap_err().to_string(),
            "periodization must be none, korobov2 or korobov3"
        );
    }
}

#[test]
fn family_policy_uses_native_type_with_explicit_original_cli_default() {
    let historical: GenerationInput = toml::from_str("order=1").unwrap();
    assert_eq!(
        historical.family_preparation,
        FamilyPreparationPolicy::Original
    );
    let original: GenerationInput = toml::from_str("family_preparation='Original'").unwrap();
    assert_eq!(original.family_preparation, historical.family_preparation);
    let prepared: GenerationInput =
        toml::from_str("[family_preparation.SingleUnitTerm]\nmax_states=32").unwrap();
    assert_eq!(
        prepared.family_preparation,
        FamilyPreparationPolicy::SingleUnitTerm { max_states: 32 }
    );
    for invalid in [
        "family_preparation='Unknown'",
        "[family_preparation.SingleUnitTerm]\nmax_states=-1",
        "[family_preparation.SingleUnitTerm]",
    ] {
        assert!(toml::from_str::<GenerationInput>(invalid).is_err());
    }
}

#[test]
fn coefficient_options_reuse_native_defaults_and_reject_unknown_steering() {
    let historical: GenerationInput = toml::from_str("order=1").unwrap();
    assert_eq!(
        historical.coefficient_expansion,
        CoefficientExpansionOptions::default()
    );
    assert_eq!(
        serde_json::to_string(&historical.coefficient_expansion).unwrap(),
        r#"{"method":"full_expression","max_series_attempts":null,"max_relative_width":null,"max_unique_requests":null}"#,
        "default1 retains the historical serialized options"
    );
    let explicit_default: GenerationInput =
        toml::from_str("[coefficient_expansion]\ninitial_relative_width=1").unwrap();
    assert_eq!(
        explicit_default.coefficient_expansion,
        historical.coefficient_expansion
    );
    let opted_in: GenerationInput = toml::from_str(
        "[coefficient_expansion]\nmethod='coefficient_series'\ninitial_relative_width=2\nmax_series_attempts=4\nmax_relative_width=12\nmax_unique_requests=32",
    ).unwrap();
    assert_eq!(
        opted_in.coefficient_expansion,
        CoefficientExpansionOptions {
            method: CoefficientExpansionMethod::NativeNamed,
            initial_relative_width: 2,
            max_series_attempts: Some(4),
            max_relative_width: Some(12),
            max_unique_requests: Some(32),
        }
    );
    for invalid in [
        "[coefficient_expansion]\nmethod='unknown'",
        "[coefficient_expansion]\nresolver='interleaved'",
        "[coefficient_expansion]\nmax_unique_requests=-1",
        "[coefficient_expansion]\ninitial_relative_width=9223372036854775808",
    ] {
        assert!(toml::from_str::<GenerationInput>(invalid).is_err());
    }
}

#[test]
fn generation_mode_and_subtraction_use_native_defaults_and_strict_names() {
    use fastsecdec::generation::{GenerationMode, GenerationOptions, SubtractionStrategy};
    let historical: GenerationInput = toml::from_str("order=1").unwrap();
    let native = GenerationOptions::default();
    assert_eq!(historical.mode, native.mode);
    assert_eq!(historical.mode, GenerationMode::Symbolic);
    assert_eq!(historical.subtraction, native.subtraction);
    assert_eq!(historical.subtraction, SubtractionStrategy::Taylor);
    for mode in [GenerationMode::Symbolic, GenerationMode::NumericalDual] {
        for subtraction in [
            SubtractionStrategy::Taylor,
            SubtractionStrategy::IntegrateByParts,
        ] {
            let input: GenerationInput = toml::from_str(&format!(
                "mode='{}'\nsubtraction='{}'",
                mode.name(),
                subtraction.name(),
            ))
            .unwrap();
            assert_eq!(input.mode, mode);
            assert_eq!(input.subtraction, subtraction);
        }
    }
    for invalid in [
        "mode='numeric'",
        "mode='Symbolic'",
        "mode=1",
        "subtraction='ibp'",
        "subtraction='Taylor'",
    ] {
        assert!(
            toml::from_str::<GenerationInput>(invalid).is_err(),
            "{invalid}"
        );
    }
}
#[test]
fn contour_native_settings_roundtrip_with_separate_validation_policy() {
    use fastsecdec::contour::{ContourMode, ContourValidation};
    let settings: super::IntegrationInput = toml::from_str(
        "[contour.deformation]\nmode='fixed'\nlambda=2.5\n[contour.validation]\npolicy='pilot'\npilot_points=128\n",
    ).unwrap();
    settings.validate_execution().unwrap();
    assert_eq!(
        settings.contour.deformation,
        ContourMode::Fixed { lambda: 2.5 }
    );
    assert_eq!(settings.contour.validation.policy, ContourValidation::Pilot);
    let restored: super::IntegrationInput =
        serde_json::from_value(serde_json::to_value(&settings).unwrap()).unwrap();
    assert_eq!(restored.contour, settings.contour);
    assert!(!GenerationInput::default().contour);
    let generation: GenerationInput = toml::from_str("contour=true").unwrap();
    assert!(generation.contour);
}

#[test]
fn explicit_generation_recipes_preserve_legacy_cards_and_override_priority() {
    use fastsecdec::kernel::ProgramRecipe;
    assert_eq!(
        GenerationInput::default().program_recipe(),
        ProgramRecipe::UndeformedV1
    );
    assert_eq!(
        toml::from_str::<GenerationInput>("contour=true")
            .unwrap()
            .program_recipe(),
        ProgramRecipe::UndeformedV1
    );
    for recipe in [
        ProgramRecipe::UndeformedV1,
        ProgramRecipe::FixedV1,
        ProgramRecipe::DynamicPolynomialV1,
        ProgramRecipe::DynamicSignAwareV1,
    ] {
        let source = format!("[generation]\ncontour=true\nrecipe='{}'", recipe.name());
        let mut card: super::RunCard = toml::from_str(&source).unwrap();
        assert_eq!(card.generation.program_recipe(), recipe);
        super::GenerationOverrides {
            contour: true,
            ..Default::default()
        }
        .apply(&mut card);
        assert_eq!(
            card.generation.program_recipe(),
            ProgramRecipe::UndeformedV1
        );
        assert_eq!(card.generation.recipe_family().recipes().len(), 4);
        let overrides = super::GenerationOverrides {
            contour: false,
            threshold_decomposition: false,
            recipe: Some(recipe),
            contour_jacobian: None,
        };
        overrides.apply(&mut card);
        assert_eq!(card.generation.program_recipe(), recipe);
        assert!(card.generation.validate_resident_recipe(recipe).is_ok());
        assert_eq!(
            serde_json::from_value::<super::GenerationOverrides>(
                serde_json::to_value(overrides).unwrap()
            )
            .unwrap(),
            overrides
        );
    }
    let historical: super::GenerationOverrides =
        serde_json::from_str(r#"{"contour":true}"#).unwrap();
    assert_eq!(historical.recipe, None);
    assert_eq!(
        serde_json::to_value(historical).unwrap(),
        serde_json::json!({"contour": true})
    );
    assert!(toml::from_str::<GenerationInput>("recipe='dynamic-sign-aware-v2'").is_err());
}

#[test]
fn contour_jacobian_choice_resolves_before_input_preparation() {
    use fastsecdec::contour::ContourJacobian;
    use fastsecdec::generation::GenerationMode;
    let mut historical = GenerationInput::default();
    historical.resolve_jacobian().unwrap();
    assert_eq!(
        historical.evaluator.contour_jacobian,
        ContourJacobian::Symbolic
    );
    for mode in [GenerationMode::Symbolic, GenerationMode::NumericalDual] {
        for choice in [ContourJacobian::Symbolic, ContourJacobian::Dual] {
            let mut options = GenerationInput {
                mode,
                contour_jacobian: choice,
                ..Default::default()
            };
            options.resolve_jacobian().unwrap();
            assert_eq!(options.mode, mode);
            assert_eq!(options.evaluator.contour_jacobian, choice);
        }
    }
    let mut symbolic_endpoints: GenerationInput =
        toml::from_str("contour_jacobian='dual'").unwrap();
    symbolic_endpoints.resolve_jacobian().unwrap();
    assert_eq!(symbolic_endpoints.mode, GenerationMode::Symbolic);
    assert_eq!(
        symbolic_endpoints.evaluator.contour_jacobian,
        ContourJacobian::Dual
    );
    let mut contradictory: GenerationInput =
        toml::from_str("[evaluator]\ncontour_jacobian='dual'").unwrap();
    assert!(contradictory.resolve_jacobian().is_err());
    let mut card: super::RunCard = toml::from_str("[generation]\nmode='numerical_dual'").unwrap();
    let override_choice = super::GenerationOverrides {
        contour_jacobian: Some(ContourJacobian::Dual),
        ..Default::default()
    };
    let decoded = serde_json::from_value::<super::GenerationOverrides>(
        serde_json::to_value(override_choice).unwrap(),
    )
    .unwrap();
    decoded.apply(&mut card);
    card.generation.resolve_jacobian().unwrap();
    assert_eq!(
        card.generation.evaluator.contour_jacobian,
        ContourJacobian::Dual
    );
    assert!(toml::from_str::<GenerationInput>("contour_jacobian='automatic'").is_err());
}
